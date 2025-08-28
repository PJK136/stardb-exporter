use std::collections::HashMap;

use crate::{
    app::{App, Message, State},
    games,
};

#[derive(serde::Serialize)]
#[allow(non_snake_case)]
pub struct GOOD<'a> {
    format: &'a str,
    version: u32,
    source: &'a str,
    characters: &'a Vec<games::Character>,
    artifacts: &'a Vec<games::Artifact>,
    weapons: &'a Vec<games::Weapon>,
    materials: &'a HashMap<String, u32>,
}

pub fn show(ui: &mut egui::Ui, inventory: &games::Inventory, app: &App) {
    ui.label("Finished");

    if ui
        .button(format!(
            "Copy {} artifacts, {} weapons, {} materials, and {} characters to clipboard",
            inventory.artifacts.len(),
            inventory.weapon.len(),
            inventory.materials.len(),
            inventory.characters.len(),
        ))
        .clicked()
    {
        if let Err(e) = arboard::Clipboard::new().and_then(|mut c| {
            c.set_text(
                serde_json::json!(GOOD {
                    format: "GOOD",
                    version: 2,
                    source: "stardb-exporter fork by PJK136",
                    characters: &inventory.characters,
                    artifacts: &inventory.artifacts,
                    weapons: &inventory.weapon,
                    materials: &inventory.materials
                })
                .to_string(),
            )
        }) {
            app.message_tx
                .send(Message::GoTo(State::Error(e.to_string())))
                .unwrap();
        } else {
            app.message_tx
                .send(Message::Toast(egui_notify::Toast::success("Copied")))
                .unwrap();
        }
    }
}
