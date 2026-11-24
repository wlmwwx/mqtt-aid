//! The settings window.

use egui::RichText;

use crate::i18n;
use crate::i18n::Language;
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

    egui::Window::new(i18n::t("设置"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_width(320.0);

            ui.label(RichText::new(i18n::t("外观")).strong());
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(state.theme == Theme::Dark, i18n::t("深色"))
                    .clicked()
                {
                    state.theme = Theme::Dark;
                }
                if ui
                    .selectable_label(state.theme == Theme::Light, i18n::t("浅色"))
                    .clicked()
                {
                    state.theme = Theme::Light;
                }
            });

            ui.separator();
            ui.label(RichText::new(i18n::t("语言")).strong());
            ui.horizontal(|ui| {
                for language in Language::all() {
                    if ui
                        .selectable_label(state.language == *language, language.label())
                        .clicked()
                    {
                        state.language = *language;
                    }
                }
            });

            ui.separator();
            ui.label(RichText::new(i18n::t("消息")).strong());
            ui.label(i18n::t("缓存的消息"));
            ui.add(
                egui::Slider::new(&mut new_limit, 100.0..=100_000.0)
                    .step_by(100.0)
                    .clamping(egui::SliderClamping::Always),
            );

            ui.separator();
            if ui.button(i18n::t("关闭")).clicked() {
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
