use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    sync::mpsc,
};

use auto_artifactarium::{matches_achievement_packet, matches_artifact_packet, GamePacket, GameSniffer};
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
    lock: bool,
    substats: Vec<super::Substat>
}

pub fn sniff_artifacts(
    artifact_id_map: &HashMap<u32, super::ArtifactData>,
    main_prop_map: &HashMap<u32, String>,
    affix_prop_map: &HashMap<u32, super::Substat>,
    device_rx: &mpsc::Receiver<Vec<u8>>,
) -> anyhow::Result<Vec<Artifact>> {
    let keys = load_keys()?;
    let mut sniffer = GameSniffer::new().set_initial_keys(keys);

    let mut artifacts = Vec::new();

    while let Ok(data) = device_rx.recv() {
        let Some(GamePacket::Commands(commands)) = sniffer.receive_packet(data) else {
            continue;
        };

        for command in commands {
            if let Some(read_artifacts) = matches_artifact_packet(&command) {
                tracing::info!("Found artifact packet");

                if !artifacts.is_empty() {
                    continue;
                }

                for artifact in read_artifacts {
                    if let Some(artifact_type) = artifact_id_map.get(&artifact.id) {
                        let mut substats = Vec::<super::Substat>::new();
                        for substat_id in artifact.append_prop_id_list {
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
                                substat.value = ((substat.value * 100.0).round() / 10.0).round() / 10.0;
                            }
                            else {
                                substat.value = substat.value.round();
                            }
                        }

                        artifacts.push(
                            Artifact{
                                setKey: artifact_type.setKey.clone(),
                                slotKey: artifact_type.slotKey.clone(),
                                level: artifact.level - 1,
                                rarity: artifact_type.rarity,
                                mainStatKey: main_prop_map.get(&artifact.main_prop_id).cloned().unwrap_or_else(|| "null".to_string()),
                                lock: artifact.is_locked,
                                substats: substats
                            }
                        );
                    }
                }
            }
        }

        if !artifacts.is_empty() {
            break;
        }
    }

    if artifacts.is_empty() {
        return Err(anyhow::anyhow!("No artifacts found"));
    }

    Ok(artifacts)
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
