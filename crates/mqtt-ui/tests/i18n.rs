//! The languages of the interface.
//!
//! Two things have to hold: every Chinese string of the sources has to be
//! in the catalogue, and switching the language has to change the texts of
//! the window.

use std::fs;
use std::path::{Path, PathBuf};

use egui::Context;
use mqtt_ui::i18n::{self, Language};
use mqtt_ui::panels::menu_bar::menus;
use mqtt_ui::state::{UiActions, UiState};

/// Every source file of the crate.
fn sources(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("the sources are readable") {
        let path = entry.expect("the entry is readable").path();
        if path.is_dir() {
            sources(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

/// The Chinese strings of a text.
///
/// Every string literal that holds a Chinese character is a text of the
/// interface, and every one of them has to be in the catalogue.
fn chinese_strings(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut literal = String::new();
    let mut inside = false;
    let mut escaped = false;

    for character in text.chars() {
        if inside {
            if escaped {
                escaped = false;
                literal.push(character);
                continue;
            }
            match character {
                '\\' => escaped = true,
                '"' => {
                    inside = false;
                    if literal.chars().any(is_chinese) {
                        found.push(literal.clone());
                    }
                    literal.clear();
                }
                _ => literal.push(character),
            }
            continue;
        }
        if character == '"' {
            inside = true;
            literal.clear();
        }
    }

    found
}

/// Whether a character belongs to a Chinese text.
fn is_chinese(character: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&character)
}

/// The texts the menu bar shows.
fn draw(state: &mut UiState) -> String {
    let ctx = Context::default();
    let mut actions = UiActions::default();
    let output = ctx.run_ui(egui::RawInput::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            menus(ui, state, &mut actions);
        });
    });

    let mut text = String::new();
    for shape in &output.shapes {
        if let egui::Shape::Text(text_shape) = &shape.shape {
            text.push_str(&text_shape.galley.job.text);
            text.push('\n');
        }
    }
    output.drop_without_applying_deltas();
    text
}

/// Every Chinese string of the sources is in the catalogue.
///
/// A string that the catalogue forgot would be shown as it is written, which
/// in English means Chinese in the middle of an English window.
#[test]
fn every_chinese_string_of_the_sources_is_in_the_catalogue() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&directory, &mut files);

    let mut missing = Vec::new();
    for file in files {
        // The catalogue itself holds every string.
        if file.ends_with("i18n.rs") {
            continue;
        }
        let text = fs::read_to_string(&file).expect("the file is readable");
        for literal in chinese_strings(&text) {
            if !i18n::knows_any(&literal) {
                missing.push(format!("{}: {literal}", file.display()));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "these strings are not in the catalogue:\n{}",
        missing.join("\n")
    );
}

/// Switching the language changes the texts of the window.
#[test]
fn switching_the_language_changes_the_interface() {
    let mut state = UiState::default();

    i18n::set_language(Language::Chinese);
    let chinese = draw(&mut state);

    i18n::set_language(Language::English);
    let english = draw(&mut state);

    assert!(
        chinese.contains("文件"),
        "the menu is not there:\n{chinese}"
    );
    assert!(
        english.contains("File"),
        "the menu is not translated:\n{english}"
    );
    assert!(
        !english.contains("文件"),
        "the menu is still in Chinese:\n{english}"
    );
}

/// A language that nobody knows falls back to Chinese.
#[test]
fn an_unknown_language_is_chinese() {
    assert_eq!(Language::from_code("zh"), Some(Language::Chinese));
    assert_eq!(Language::from_code("EN"), Some(Language::English));
    assert_eq!(Language::from_code("fr"), None);
    assert_eq!(Language::default(), Language::Chinese);
}
