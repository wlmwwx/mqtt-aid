//! The settings window.

use egui::RichText;

use crate::state::UiState;
use crate::theme::Theme;

/// The application settings.
pub fn settings_window(
    ctx: &egui::Context,
    state: &mut UiState,
    cache_limit: usize,
) -> Option<usize> {
    if !state.show_settings {
        return None;
    }

    let mut new_limit = cache_limit as f64;
    let mut close = false;

    egui::Window::new("Settings")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_width(320.0);

            ui.label(RichText::new("Appearance").strong());
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(state.theme == Theme::Dark, "Dark")
                    .clicked()
                {
                    state.theme = Theme::Dark;
                }
                if ui
                    .selectable_label(state.theme == Theme::Light, "Light")
                    .clicked()
                {
                    state.theme = Theme::Light;
                }
            });

            ui.separator();
            ui.label(RichText::new("Messages").strong());
            ui.label("Cached messages");
            ui.add(
                egui::Slider::new(&mut new_limit, 100.0..=100_000.0)
                    .step_by(100.0)
                    .clamping(egui::SliderClamping::Always),
            );

            ui.separator();
            if ui.button("Close").clicked() {
                close = true;
            }
        });

    if close {
        state.show_settings = false;
    }

    if (new_limit as usize) != cache_limit {
        Some(new_limit as usize)
    } else {
        None
    }
}
