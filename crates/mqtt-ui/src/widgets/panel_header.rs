//! The header of a dockable panel.
//!
//! SolidCraft's panels have a header with a drag grip on the left, the panel
//! title, and on the right an overflow menu and a collapse chevron. This is
//! the same header, drawn with the vector icons of this application.

use egui::{Color32, RichText, Ui};

use crate::i18n;
use crate::widgets::icons::{self, Icon};

/// What the user asked the header to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderAction {
    /// Nothing happened.
    None,
    /// The collapse chevron was pressed.
    Collapse,
}

/// Draw a panel header.
///
/// `title` is the panel name, `menu` draws the overflow menu when the user
/// presses the `…` button, and the chevron toggles the panel.
pub fn panel_header(ui: &mut Ui, title: &str, mut menu: impl FnMut(&mut Ui)) -> HeaderAction {
    let mut action = HeaderAction::None;

    ui.horizontal(|ui| {
        // The grip: it shows the panel can be dragged.
        let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 16.0), egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            icons::draw(
                ui.painter(),
                Icon::Grip,
                rect,
                ui.visuals().weak_text_color(),
                1.0,
            );
        }

        ui.label(RichText::new(title).strong());

        let mut overflow = None;

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::click());
            if ui.is_rect_visible(rect) {
                icons::draw(
                    ui.painter(),
                    Icon::ChevronUp,
                    rect,
                    ui.visuals().weak_text_color(),
                    1.4,
                );
            }
            if response.clicked() {
                action = HeaderAction::Collapse;
            }
            response.on_hover_text(i18n::t("折叠面板"));

            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::click());
            if ui.is_rect_visible(rect) {
                icons::draw(
                    ui.painter(),
                    Icon::Ellipsis,
                    rect,
                    ui.visuals().weak_text_color(),
                    1.4,
                );
            }
            overflow = Some(response.on_hover_text(i18n::t("面板菜单")));
        });

        // The overflow menu is a popup anchored to the `…` button.
        if let Some(response) = overflow {
            egui::Popup::menu(&response).show(|ui| menu(ui));
        }
    });

    action
}

/// A coloured dot that shows a connection status.
pub fn status_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().circle_filled(rect.center(), 3.5, color);
    }
}
