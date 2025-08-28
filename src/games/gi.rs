use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    sync::mpsc,
};

use auto_artifactarium::{
    GamePacket, GameSniffer, matches_achievement_packet, matches_avatar_packet, matches_item_packet,
};
use base64::prelude::*;

use regex::Regex;

pub fn sniff(
    achievement_ids: &[u32],
    device_rx: &mpsc::Receiver<Vec<u8>>,
) -> anyhow::Result<Vec<u32>> {
    let keys = load_keys()?;
    let mut sniffer = GameSniffer::new().set_initial_keys(keys);

    let mut achievements = Vec::new();

    while let Ok(data) = device_rx.recv() {
        let Some(GamePacket::Commands(commands)) = sniffer.receive_packet(data) else {
            continue;
        };

        for command in commands {
            if let Some(read_achievements) = matches_achievement_packet(&command) {
                tracing::info!("Found achievement packet");

                if !achievements.is_empty() {
                    continue;
                }

                for achievement in read_achievements {
                    if achievement_ids.contains(&achievement.id)
                        && (achievement.status == 2 || achievement.status == 3)
                    {
                        achievements.push(achievement.id);
                    }
                }
            }
        }

        if !achievements.is_empty() {
            break;
        }
    }

    if achievements.is_empty() {
        return Err(anyhow::anyhow!("No achievements found"));
    }

    Ok(achievements)
}

#[derive(serde::Serialize)]
#[allow(non_snake_case)]
pub struct Artifact {
    setKey: String,
    slotKey: String,
    level: u32,
    rarity: u32,
    mainStatKey: String,
    location: String,
    lock: bool,
    substats: Vec<super::Substat>,
}

#[derive(serde::Serialize)]
#[allow(non_snake_case)]
pub struct Weapon {
    key: String,
    level: u32,
    ascension: u32,
    refinement: u32,
    location: String,
    lock: bool,
}

#[derive(serde::Serialize)]
#[allow(non_snake_case)]
pub struct Inventory {
    pub artifacts: Vec<Artifact>,
    pub weapon: Vec<Weapon>,
    pub materials: HashMap<String, u32>,
}

pub fn sniff_inventory(
    artifact_id_map: &HashMap<u32, super::ArtifactData>,
    main_prop_map: &HashMap<u32, String>,
    affix_prop_map: &HashMap<u32, super::Substat>,
    weapon_id_map: &HashMap<u32, super::WeaponData>,
    material_id_map: &HashMap<u32, String>,
    character_id_map: &HashMap<u32, String>,
    device_rx: &mpsc::Receiver<Vec<u8>>,
    no_artifact_filter: bool,
    without_character: bool,
) -> anyhow::Result<Inventory> {
    let keys = load_keys()?;
    let mut sniffer = GameSniffer::new().set_initial_keys(keys);

    let mut inventory = Inventory {
        artifacts: Vec::new(),
        weapon: Vec::new(),
        materials: HashMap::new(),
    };
    let mut items = Vec::new();
    let mut item_guid_character_name_map = HashMap::<u64, &String>::new();

    let mut found = false;

    while !found && let Ok(data) = device_rx.recv() {
        let Some(GamePacket::Commands(commands)) = sniffer.receive_packet(data) else {
            continue;
        };

        for command in commands {
            if let Some(mut read_items) = matches_item_packet(&command)
                && !read_items.is_empty()
            {
                tracing::info!("Found item packet");
                items.append(&mut read_items);
            };

            if !without_character
                && let Some(read_avatars) = matches_avatar_packet(&command)
                && !read_avatars.is_empty()
            {
                tracing::info!("Found avatar packet");

                for avatar in read_avatars {
                    if let Some(name) = character_id_map.get(&avatar.avatar_id) {
                        for guid in avatar.equip_guid_list {
                            item_guid_character_name_map.insert(guid, name);
                        }
                    }
                }
            };

            if !without_character && item_guid_character_name_map.is_empty() {
                continue;
            }

            for item in &items {
                if item.has_equip() {
                    let equip = item.equip();
                    let location = item_guid_character_name_map
                        .get(&item.guid)
                        .map(|s| (*s).clone())
                        .unwrap_or_default();

                    if equip.has_reliquary()
                        && let Some(artifact_type) = artifact_id_map.get(&item.item_id)
                    {
                        let artifact = equip.reliquary();
                        if !no_artifact_filter
                            && (artifact.level < 2 || artifact_type.rarity < 3)
                            && location.is_empty()
                        {
                            continue;
                        }

                        let mut substats = Vec::<super::Substat>::new();
                        for substat_id in &artifact.append_prop_id_list {
                            if let Some(current_substat) = affix_prop_map.get(&substat_id) {
                                let mut found = false;
                                for substat in substats.iter_mut() {
                                    if substat.key == current_substat.key {
                                        substat.value += current_substat.value;
                                        found = true;
                                        break;
                                    }
                                }

                                if !found {
                                    substats.push(current_substat.clone());
                                }
                            }
                        }

                        for substat in substats.iter_mut() {
                            if substat.key.ends_with("_") {
                                substat.value =
                                    ((substat.value * 100.0).round() / 10.0).round() / 10.0;
                            } else {
                                substat.value = substat.value.round();
                            }
                        }

                        inventory.artifacts.push(Artifact {
                            setKey: artifact_type.setKey.clone(),
                            slotKey: artifact_type.slotKey.clone(),
                            level: artifact.level - 1,
                            rarity: artifact_type.rarity,
                            mainStatKey: main_prop_map
                                .get(&artifact.main_prop_id)
                                .cloned()
                                .unwrap_or("null".to_string()),
                            location: location,
                            lock: equip.is_locked,
                            substats: substats,
                        });

                        found = true;
                    } else if equip.has_weapon()
                        && let Some(weapon_data) = weapon_id_map.get(&item.item_id)
                    {
                        let weapon = equip.weapon();
                        let refinement = match weapon.affix_map.values().next() {
                            Some(&x) => 1 + x,
                            None => 1,
                        };

                        if weapon_data.rarity < 4
                            && weapon.level == 1
                            && refinement == 1
                            && !equip.is_locked
                            && location.is_empty()
                        {
                            continue;
                        }

                        inventory.weapon.push(Weapon {
                            key: weapon_data.name.clone(),
                            level: weapon.level,
                            ascension: weapon.promote_level,
                            refinement: refinement,
                            location: location,
                            lock: equip.is_locked,
                        });

                        found = true;
                    }
                }

                if item.has_material()
                    && let material = item.material()
                    && let Some(name) = material_id_map.get(&item.item_id)
                {
                    inventory.materials.insert(name.clone(), material.count);

                    found = true;
                }
            }

            items.clear();
        }
    }

    if !found {
        return Err(anyhow::anyhow!("No items found"));
    }

    Ok(inventory)
}

fn load_keys() -> anyhow::Result<HashMap<u16, Vec<u8>>> {
    let keys: HashMap<u16, String> = serde_json::from_slice(include_bytes!("../../keys/gi.json"))?;

    let mut keys_bytes = HashMap::new();

    for (k, v) in keys {
        keys_bytes.insert(k, BASE64_STANDARD.decode(v)?);
    }

    Ok(keys_bytes)
}

pub fn game_path() -> anyhow::Result<PathBuf> {
    let mut log_path = PathBuf::from(&std::env::var("APPDATA")?);
    log_path.pop();
    log_path.push("LocalLow");
    log_path.push("miHoYo");

    let mut log_path_cn = log_path.clone();

    log_path.push("Genshin Impact");
    log_path_cn.push("原神");

    log_path.push("output_log.txt");
    log_path_cn.push("output_log.txt");

    let log_path = match (log_path.exists(), log_path_cn.exists()) {
        (true, _) => log_path,
        (_, true) => log_path_cn,
        _ => return Err(anyhow::anyhow!("Can't find log file")),
    };

    let re = Regex::new(r".:\\.+(GenshinImpact_Data|YuanShen_Data)")?;

    for line in BufReader::new(File::open(log_path)?).lines() {
        let Ok(line) = line else {
            break;
        };

        if let Some(m) = re.find(&line) {
            return Ok(PathBuf::from(m.as_str()));
        }
    }

    Err(anyhow::anyhow!("Couldn't find game path"))
}
