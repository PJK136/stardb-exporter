mod gi;
mod hsr;
mod zzz;

use std::{
    collections::HashMap, path::{Path, PathBuf}, sync::mpsc, thread
};

use crate::app::{Message, State};
use regex::Regex;

#[derive(Clone, Copy, PartialEq)]
pub enum Game {
    Hsr,
    Gi,
    Zzz,
}

impl Game {
    pub fn achievements(self, message_tx: &mpsc::Sender<Message>) {
        let message_tx = message_tx.clone();

        thread::spawn(move || {
            let achievement_ids = match self.achievement_ids() {
                Ok(achievement_ids) => achievement_ids,
                Err(e) => {
                    message_tx
                        .send(Message::GoTo(State::Error(e.to_string())))
                        .unwrap();
                    return;
                }
            };

            let devices = match self.devices() {
                Ok(devices) => devices,
                Err(e) => {
                    message_tx
                        .send(Message::GoTo(State::Error(e.to_string())))
                        .unwrap();
                    return;
                }
            };

            let (device_tx, device_rx) = mpsc::channel();
            for (i, device) in devices.into_iter().enumerate() {
                let device_tx = device_tx.clone();
                let message_tx = message_tx.clone();
                std::thread::spawn(move || self.capture_device(i, device, &device_tx, &message_tx));
            }

            let achievements = match self {
                Game::Hsr => hsr::sniff(&achievement_ids, &device_rx),
                Game::Gi => gi::sniff(&achievement_ids, &device_rx),
                _ => unimplemented!(),
            };
            let achievements = match achievements {
                Ok(achievements) => achievements,
                Err(e) => {
                    message_tx
                        .send(Message::GoTo(State::Error(e.to_string())))
                        .unwrap();
                    return;
                }
            };

            message_tx
                .send(Message::GoTo(State::Achievements(achievements)))
                .unwrap();
        });
    }

    pub fn artifacts(self, message_tx: &mpsc::Sender<Message>) {
        let message_tx = message_tx.clone();

        thread::spawn(move || {
            let artifact_id_map = match build_artifact_id_map() {
                Ok(artifact_id_map) => artifact_id_map,
                Err(e) => {
                    message_tx
                        .send(Message::GoTo(State::Error(e.to_string())))
                        .unwrap();
                    return;
                }
            };
            let main_prop_map = match build_main_prop_map() {
                Ok(main_prop_map) => main_prop_map,
                Err(e) => {
                    message_tx
                        .send(Message::GoTo(State::Error(e.to_string())))
                        .unwrap();
                    return;
                }
            };
            let affix_prop_map = match build_affix_prop_map() {
                Ok(affix_prop_map) => affix_prop_map,
                Err(e) => {
                    message_tx
                        .send(Message::GoTo(State::Error(e.to_string())))
                        .unwrap();
                    return;
                }
            };

            let devices = match self.devices() {
                Ok(devices) => devices,
                Err(e) => {
                    message_tx
                        .send(Message::GoTo(State::Error(e.to_string())))
                        .unwrap();
                    return;
                }
            };

            let (device_tx, device_rx) = mpsc::channel();
            for (i, device) in devices.into_iter().enumerate() {
                let device_tx = device_tx.clone();
                let message_tx = message_tx.clone();
                std::thread::spawn(move || self.capture_device(i, device, &device_tx, &message_tx));
            }

            let artifacts = match self {
                Game::Gi => gi::sniff_artifacts(&artifact_id_map, &main_prop_map, &affix_prop_map, &device_rx),
                _ => unimplemented!(),
            };
            let artifacts = match artifacts {
                Ok(artifacts) => artifacts,
                Err(e) => {
                    message_tx
                        .send(Message::GoTo(State::Error(e.to_string())))
                        .unwrap();
                    return;
                }
            };

            message_tx
                .send(Message::GoTo(State::Artifacts(artifacts)))
                .unwrap();
        });
    }

    pub fn game_path(self) -> anyhow::Result<PathBuf> {
        match self {
            Game::Hsr => hsr::game_path(),
            Game::Gi => gi::game_path(),
            Game::Zzz => zzz::game_path(),
        }
    }

    pub fn achievement_url(self) -> String {
        let prefix = match self {
            Game::Hsr => "",
            Game::Gi => "genshin",
            Game::Zzz => "zzz",
        };

        format!("https://stardb.gg/{prefix}/achievement-tracker")
    }

    pub fn pull_url(self) -> String {
        let path = match self {
            Game::Hsr => "warp-tracker",
            Game::Gi => "genshin/wish-tracker",
            Game::Zzz => "zzz/signal-tracker",
        };

        format!("https://stardb.gg/{path}")
    }

    fn achievement_ids(self) -> anyhow::Result<Vec<u32>> {
        #[derive(serde::Deserialize)]
        struct Achievement {
            id: u32,
        }

        let url = match self {
            Game::Hsr => "https://stardb.gg/api/achievements",
            Game::Gi => "https://stardb.gg/api/gi/achievements",
            _ => unimplemented!(),
        };

        let achievements: Vec<Achievement> = ureq::get(url).call()?.body_mut().read_json()?;
        let achievement_ids: Vec<_> = achievements.into_iter().map(|a| a.id).collect();

        Ok(achievement_ids)
    }

    fn devices(self) -> anyhow::Result<Vec<pcap::Device>> {
        Ok(pcap::Device::list()?
            .into_iter()
            .filter(|d| d.flags.connection_status == pcap::ConnectionStatus::Connected)
            .filter(|d| !d.addresses.is_empty())
            .filter(|d| !d.flags.is_loopback())
            .collect())
    }

    fn capture_device(
        self,
        i: usize,
        device: pcap::Device,
        device_tx: &mpsc::Sender<Vec<u8>>,
        message_tx: &mpsc::Sender<Message>,
    ) -> anyhow::Result<()> {
        let packet_filer = match self {
            Game::Hsr => "udp portrange 23301-23302",
            Game::Gi => "udp portrange 22101-22102",
            _ => unimplemented!(),
        };

        tracing::debug!("Finding devices...");

        loop {
            let mut capture = pcap::Capture::from_device(device.clone())?
                .immediate_mode(true)
                .promisc(true)
                .timeout(0)
                .open()?;

            capture.filter(packet_filer, true)?;

            message_tx
                .send(Message::Toast({
                    let mut toast = egui_notify::Toast::success(format!("Device {i} Ready~!"));
                    toast.duration(None);
                    toast
                }))
                .unwrap();

            message_tx
                .send(Message::GoTo(State::Waiting("Running".to_string())))
                .unwrap();
            tracing::info!("Device {i} Ready~!");

            let mut has_captured = false;

            loop {
                match capture.next_packet() {
                    Ok(packet) => {
                        device_tx.send(packet.data.to_vec())?;
                        has_captured = true;
                    }
                    Err(_) if !has_captured => break,
                    Err(pcap::Error::TimeoutExpired) => continue,
                    Err(e) => return Err(anyhow::anyhow!("{e}")),
                }
            }

            message_tx
                .send(Message::Toast({
                    let mut toast = egui_notify::Toast::error(format!(
                        "Device {i} Error. Starting up again..."
                    ));
                    toast.duration(None);
                    toast
                }))
                .unwrap();
            tracing::info!("Device {i} Error. Starting up again...");
        }
    }
}

pub fn pulls_from_game_path(path: &Path) -> anyhow::Result<String> {
    let mut path = path.to_path_buf();

    path.push("webCaches");

    let re = Regex::new(r"^\d+\.\d+\.\d+\.\d+$")?;
    let mut paths: Vec<_> = path
        .read_dir()?
        .flat_map(|r| r.ok().map(|d| d.path()))
        .filter(|p| re.is_match(p.file_name().and_then(|o| o.to_str()).unwrap_or_default()))
        .collect();
    paths.sort();

    let mut cache_path = paths[paths.len() - 1].clone();
    cache_path.push("Cache");
    cache_path.push("Cache_Data");
    cache_path.push("data_2");

    let bytes = std::fs::read(cache_path)?;
    let data = String::from_utf8_lossy(&bytes);
    let lines: Vec<_> = data.split("1/0/").collect();

    for line in lines.iter().rev() {
        if line.starts_with("https://")
            && (line.contains("getGachaLog") || line.contains("getLdGachaLog"))
        {
            if let Some(url) = line.split('\0').next() {
                if ureq::get(url)
                    .call()
                    .ok()
                    .and_then(|mut r| r.body_mut().read_json::<serde_json::Value>().ok())
                    .map(|j| j["retcode"] == 0)
                    .unwrap_or_default()
                {
                    return Ok(url.to_string());
                }
            }
        }
    }

    Err(anyhow::anyhow!("Couldn't find pull url"))
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct ReliquaryExcelConfigDataEntry {
    equipType: String,
    id: u32,
    rankLevel: u32,
    setId: u32,
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct DisplayItemExcelConfigDataEntry {
    displayType: String,
    nameTextMapHash: u32,
    param: u32,
}

#[derive(Debug)]
#[allow(non_snake_case)]
pub struct ArtifactData {
    pub setKey: String,
    pub slotKey: String,
    pub rarity: u32,
}

fn map_equip_type_to_good(input: &str) -> String {
    match input {
        "EQUIP_BRACER" => "flower",
        "EQUIP_NECKLACE" => "plume",
        "EQUIP_SHOES" => "sands",
        "EQUIP_RING" => "goblet",
        "EQUIP_DRESS" => "circlet",
        _ => input,
    }.to_owned()
}

fn map_set_name_to_good(input: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in input.chars() {
        if c.is_alphabetic() {
            if capitalize_next {
                result.extend(c.to_uppercase());
                capitalize_next = false;
            } else {
                result.extend(c.to_lowercase());
            }
        } else if c != '\'' {
            capitalize_next = true;
        }
    }

    result
}

pub fn build_artifact_id_map() -> anyhow::Result<HashMap<u32, ArtifactData>> {
    let reliquary_excel_config_data: Vec<ReliquaryExcelConfigDataEntry> =
        serde_json::from_str(include_str!("../../data/ReliquaryExcelConfigData.json"))?;

    let display_item_excel_config_data: Vec<DisplayItemExcelConfigDataEntry> =
        serde_json::from_str(include_str!("../../data/DisplayItemExcelConfigData.json"))?;

    let text_map_en: HashMap<String, String> =
        serde_json::from_str(include_str!("../../data/TextMapEN.json"))?;

    // Map setId -> nameTextMapHash
    let mut setid_to_hash = HashMap::new();
    for entry in display_item_excel_config_data {
        if entry.displayType == "RELIQUARY_ITEM" {
            setid_to_hash.insert(entry.param, entry.nameTextMapHash);
        }
    }

    // Final id -> ArtifactData map
    let mut result = HashMap::new();
    for entry in reliquary_excel_config_data {
        if let Some(hash_num) = setid_to_hash.get(&entry.setId) {
            let hash_str = hash_num.to_string();
            if let Some(text) = text_map_en.get(&hash_str) {
                result.insert(
                    entry.id,
                    ArtifactData {
                        setKey: map_set_name_to_good(text),
                        slotKey: map_equip_type_to_good(&entry.equipType).to_string(),
                        rarity: entry.rankLevel,
                    },
                );
            }
        }
    }

    for (id, data) in &result {
        tracing::trace!("ID {} => {:?}", id, data);
    }

    Ok(result)
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct ReliquaryMainPropExcelConfigDataEntry {
    id: u32,
    propType: String,
}

fn map_main_prop_to_good(input: &str) -> String {
    match input {
        "FIGHT_PROP_HP" => "hp",
        "FIGHT_PROP_HP_PERCENT" => "hp_",
        "FIGHT_PROP_ATTACK" => "atk",
        "FIGHT_PROP_ATTACK_PERCENT" => "atk_",
        "FIGHT_PROP_DEFENSE" => "def",
        "FIGHT_PROP_DEFENSE_PERCENT" => "def_",
        "FIGHT_PROP_ELEMENT_MASTERY" => "eleMas",
        "FIGHT_PROP_CHARGE_EFFICIENCY" => "enerRech_",
        "FIGHT_PROP_HEAL_ADD" => "heal_",
        "FIGHT_PROP_CRITICAL" => "critRate_",
        "FIGHT_PROP_CRITICAL_HURT" => "critDMG_",
        "FIGHT_PROP_PHYSICAL_ADD_HURT" => "physical_dmg_",
        "FIGHT_PROP_WIND_ADD_HURT" => "anemo_dmg_",
        "FIGHT_PROP_ROCK_ADD_HURT" => "geo_dmg_",
        "FIGHT_PROP_ELEC_ADD_HURT" => "electro_dmg_",
        "FIGHT_PROP_WATER_ADD_HURT" => "hydro_dmg_",
        "FIGHT_PROP_FIRE_ADD_HURT" => "pyro_dmg_",
        "FIGHT_PROP_ICE_ADD_HURT" => "cryo_dmg_",
        "FIGHT_PROP_GRASS_ADD_HURT" => "dendro_dmg_",
        _ => input,
    }.to_owned()
}

pub fn build_main_prop_map() -> anyhow::Result<HashMap<u32, String>> {
    let reliquary_main_prop_excel_config: Vec<ReliquaryMainPropExcelConfigDataEntry> =
        serde_json::from_str(include_str!("../../data/ReliquaryMainPropExcelConfigData.json"))?;

    let mut result = HashMap::new();
    for entry in reliquary_main_prop_excel_config {
        result.insert(
            entry.id,
            map_main_prop_to_good(&entry.propType),
        );
    }

    for (id, data) in &result {
        tracing::trace!("ID {} => {:?}", id, data);
    }

    Ok(result)
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct ReliquaryAffixExcelConfigDataEntry {
    id: u32,
    propType: String,
    propValue: f64
}

#[derive(Debug, Clone, serde::Serialize)]
#[allow(non_snake_case)]
pub struct Substat {
    pub key: String,
    pub value: f64,
}

pub fn build_affix_prop_map() -> anyhow::Result<HashMap<u32, Substat>> {
    let reliquary_affix_excel_config: Vec<ReliquaryAffixExcelConfigDataEntry> =
        serde_json::from_str(include_str!("../../data/ReliquaryAffixExcelConfigData.json"))?;

    let mut result = HashMap::new();
    for entry in reliquary_affix_excel_config {
        let key = map_main_prop_to_good(&entry.propType);
        result.insert(
            entry.id,
            Substat{
                value: if key.ends_with("_") { entry.propValue * 100. } else { entry.propValue },
                key: key,
            }
        );
    }

    for (id, data) in &result {
        tracing::trace!("ID {} => {:?}", id, data);
    }

    Ok(result)
}

pub use gi::Artifact;