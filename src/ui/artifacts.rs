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
    artifacts: &'a Vec<games::Artifact>,
}

pub fn show(ui: &mut egui::Ui, artifacts: &Vec<games::Artifact>, app: &App) {
    match app.game {
        games::Game::Gi => "gi_artifacts",
        _ => unimplemented!(),
    };

    ui.label("Finished");

    if ui
        .button(format!(
            "Copy {} artifacts to clipboard",
            artifacts.len()
        ))
        .clicked()
    {
        if let Err(e) = arboard::Clipboard::new()
            .and_then(|mut c| c.set_text(serde_json::json!(
                GOOD{
                    format: "GOOD",
                    version: 2,
                    source: "stardb-exporter",
                    artifacts: artifacts
                }).to_string()))
        {
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
