//! Headless tests of the settings window.

use egui::{Context, RawInput};
use mqtt_ui::i18n::{self, LanguageChoice};
use mqtt_ui::state::UiState;

/// Draw the settings window a few times and return the texts it shows.
///
/// A floating window is laid out invisible in its first frame, so the
/// window is drawn again before its texts are looked for.
fn draw(ctx: &Context, state: &mut UiState) -> Vec<String> {
    let mut texts = Vec::new();
    for _ in 0..3 {
        let output = ctx.run_ui(RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                mqtt_ui::dialogs::settings::settings_window(ui.ctx(), state, 1000);
            });
        });
        for shape in &output.shapes {
            if let egui::Shape::Text(text_shape) = &shape.shape {
                texts.push(text_shape.galley.job.text.to_string());
            }
        }
        output.drop_without_applying_deltas();
    }
    texts
}

/// The language is chosen in the settings.
///
/// The three choices are offered: the one of the system and the two the
/// interface speaks.
#[test]
fn the_language_is_chosen_in_the_settings() {
    i18n::set_language(LanguageChoice::English);

    let ctx = Context::default();
    let mut state = UiState {
        show_settings: true,
        ..UiState::default()
    };

    let texts = draw(&ctx, &mut state);

    assert!(
        texts.iter().any(|text| text == "Follow the system"),
        "the choice of the system is missing: {texts:?}"
    );
    assert!(
        texts.iter().any(|text| text == "中文"),
        "the Chinese choice is missing: {texts:?}"
    );
    assert!(
        texts.iter().any(|text| text == "English"),
        "the English choice is missing: {texts:?}"
    );
}

/// The interface follows the system until it is told otherwise.
#[test]
fn the_interface_follows_the_system_by_default() {
    let state = UiState::default();
    assert_eq!(state.language, LanguageChoice::System);
}
