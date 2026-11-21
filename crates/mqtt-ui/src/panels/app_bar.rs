//! The application bar: the dark strip at the very top of the window.
//!
//! It follows SolidCraft's title bar: the application mark and name on the
//! left, the menu bar next to them, and the window-level controls (status
//! pill, theme, settings) on the right.

use egui::{Align, Layout, RichText, Ui};
use mqtt_core::models::ConnectionStatus;

use crate::panels::menu_bar::menus;
use crate::state::{ConnectionView, UiActions, UiState};
use crate::theme::spacing;
use crate::widgets::icons::{self, Icon};

/// Application name.
pub const APP_NAME: &str = "MQTT Assistant";

/// The title bar of the window.
pub fn app_bar(
    ui: &mut Ui,
    state: &mut UiState,
    connections: &[ConnectionView],
    actions: &mut UiActions,
) {
    let palette = state.theme.palette();

    egui::Frame::new()
        .fill(palette.app_bar)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .stroke(egui::Stroke::NONE)
        .show(ui, |ui| {
            ui.set_min_height(spacing::APP_BAR_HEIGHT);

            ui.horizontal(|ui| {
                app_mark(ui);
                ui.label(RichText::new(APP_NAME).strong().size(15.0));

                ui.add(egui::Separator::default().vertical().spacing(spacing::GAP));

                menus(ui, state, actions);

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icons::icon_button(ui, Icon::Gear, "设置") {
                        state.show_settings = true;
                    }
                    let theme_icon = if state.theme == crate::theme::Theme::Dark {
                        Icon::Sun
                    } else {
                        Icon::Moon
                    };
                    if icons::icon_button(ui, theme_icon, "切换主题") {
                        actions.toggle_theme = true;
                    }
                    version_label(ui);
                    status_pill(ui, state, connections);
                });
            });
        });
}

/// The application mark: a rounded square with a signal glyph in it.
fn app_mark(ui: &mut Ui) {
    let size = egui::vec2(22.0, 22.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }

    let painter = ui.painter();
    let accent = ui.visuals().selection.stroke.color;
    painter.rect_filled(
        rect,
        egui::CornerRadius::same(6),
        accent.gamma_multiply(0.28),
    );
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(6),
        egui::Stroke::new(1.0, accent),
        egui::StrokeKind::Middle,
    );

    // Three arcs radiating from the bottom-left corner: a signal mark.
    let origin = rect.left_bottom() + egui::vec2(6.0, -6.0);
    for (index, radius) in [5.0, 9.0, 13.0].into_iter().enumerate() {
        let color = accent.gamma_multiply(0.7 + 0.15 * index as f32);
        let mut points = Vec::new();
        for step in 0..=10 {
            let angle = (step as f32 / 10.0) * std::f32::consts::FRAC_PI_2;
            points.push(origin + egui::vec2(angle.cos(), -angle.sin()) * radius);
        }
        painter.add(egui::Shape::line(points, egui::Stroke::new(1.6, color)));
    }
    painter.circle_filled(origin, 1.8, accent);
}

/// The version, at the far right of the title bar.
fn version_label(ui: &mut Ui) {
    ui.label(
        RichText::new("v1.0.0")
            .small()
            .color(ui.visuals().weak_text_color()),
    );
}

/// A pill that shows the state of the selected connection.
fn status_pill(ui: &mut Ui, state: &UiState, connections: &[ConnectionView]) {
    let current = state
        .selected_connection
        .as_ref()
        .and_then(|id| connections.iter().find(|view| &view.config.id == id));

    let status = current
        .map(|view| view.status)
        .unwrap_or(ConnectionStatus::Disconnected);

    let palette = state.theme.palette();
    let (label, color) = match status {
        ConnectionStatus::Connected => ("已连接", palette.success),
        ConnectionStatus::Connecting | ConnectionStatus::Reconnecting => {
            ("连接中", palette.warning)
        }
        ConnectionStatus::Error => ("错误", palette.danger),
        ConnectionStatus::Disconnected => ("未连接", palette.text_faint),
    };

    let text_color = ui.visuals().text_color();
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(12.0),
        text_color,
    );

    let width = galley.size().x + 26.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 22.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            egui::CornerRadius::same(11),
            color.gamma_multiply(0.22),
        );
        painter.circle_filled(rect.left_center() + egui::vec2(11.0, 0.0), 3.5, color);
        painter.galley(
            rect.left_center() + egui::vec2(20.0, 0.0) - galley.size() / 2.0,
            galley,
            text_color,
        );
    }

    response.on_hover_text(status.label());
}
