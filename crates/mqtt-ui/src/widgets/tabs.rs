//! The tab buttons of the panels.
//!
//! The selected tab gets a blue plate, the way SolidCraft marks the active
//! category. The plate is a frame, so it is painted under the caption.

use egui::{Color32, Response, RichText, Ui};

use crate::theme::spacing;

/// A tab button. Returns the response, so the caller can see the click.
pub fn tab_button(ui: &mut Ui, text: RichText, active: bool, active_fill: Color32) -> Response {
    let fill = if active {
        active_fill
    } else {
        Color32::TRANSPARENT
    };

    egui::Frame::new()
        .fill(fill)
        .corner_radius(spacing::CORNER_SMALL)
        .inner_margin(egui::Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.add(
                egui::Button::new(text)
                    .fill(Color32::TRANSPARENT)
                    .stroke(egui::Stroke::NONE),
            )
        })
        .inner
}
