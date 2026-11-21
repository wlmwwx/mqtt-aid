//! Payload renderings: hex dump and JSON tree.

use egui::{Color32, RichText, Ui};
use mqtt_core::format::{hex_dump, parse_json};

use crate::theme::spacing;

/// Show a payload as a hex dump.
pub fn hex_view(ui: &mut Ui, bytes: &[u8]) {
    let mut text = hex_dump(bytes, 16);
    ui.add(
        egui::TextEdit::multiline(&mut text)
            .font(egui::TextStyle::Monospace)
            .desired_width(f32::INFINITY)
            .desired_rows(12)
            .interactive(true),
    );
}

/// Show a JSON document as an expandable tree.
pub fn json_view(ui: &mut Ui, value: &serde_json::Value) {
    // The id salt matters: the message table above is a scroll area in the
    // same UI, and two scroll areas with the same id make egui draw a
    // "Double use of ScrollArea ID" error over the window.
    egui::ScrollArea::vertical()
        .id_salt("json_payload")
        .max_height(spacing::JSON_VIEW_HEIGHT)
        .show(ui, |ui| {
            json_node(ui, value, 0);
        });
}

/// Show a JSON document, or the reason why it cannot be parsed.
pub fn json_text_view(ui: &mut Ui, text: &str) {
    match parse_json(text) {
        Ok(value) => json_view(ui, &value),
        Err(error) => {
            ui.colored_label(Color32::from_rgb(206, 106, 106), format!("{error}"));
        }
    }
}

fn json_node(ui: &mut Ui, value: &serde_json::Value, depth: usize) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                let header = egui::CollapsingHeader::new(egui::RichText::new(key.clone()).strong())
                    .default_open(depth < 1);
                header.show(ui, |ui| {
                    json_node(ui, child, depth + 1);
                });
            }
        }
        serde_json::Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                let label = format!("[{index}]");
                let header = egui::CollapsingHeader::new(egui::RichText::new(label).strong())
                    .default_open(depth < 1);
                header.show(ui, |ui| {
                    json_node(ui, child, depth + 1);
                });
            }
        }
        serde_json::Value::String(text) => {
            ui.label(RichText::new(format!("\"{text}\"")).color(Color32::from_rgb(126, 187, 118)));
        }
        serde_json::Value::Number(number) => {
            ui.label(RichText::new(number.to_string()).color(Color32::from_rgb(181, 148, 108)));
        }
        serde_json::Value::Bool(flag) => {
            ui.label(RichText::new(flag.to_string()).color(Color32::from_rgb(114, 159, 207)));
        }
        serde_json::Value::Null => {
            ui.label(RichText::new("null").color(Color32::GRAY));
        }
    }
}

/// A short, single line preview of a payload.
pub fn preview(bytes: &[u8], max_len: usize) -> String {
    let text = String::from_utf8_lossy(bytes);
    let cleaned: String = text
        .chars()
        .map(|c| if c.is_control() { '·' } else { c })
        .collect();
    if cleaned.chars().count() <= max_len {
        cleaned
    } else {
        let mut truncated: String = cleaned.chars().take(max_len).collect();
        truncated.push('…');
        truncated
    }
}

/// A monospaced, editable label.
pub fn monospace(ui: &mut Ui, text: &mut String) {
    ui.add(
        egui::TextEdit::singleline(text)
            .font(egui::TextStyle::Monospace)
            .desired_width(f32::INFINITY),
    );
}

/// Text with a monospaced font, used for previews.
pub fn mono_text(text: &str) -> RichText {
    RichText::new(text.to_string()).monospace()
}
