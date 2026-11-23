//! Headless tests of the menu bar.
//!
//! The menu bar is a row of dropdown menus whose entries share one command
//! vocabulary with the ribbon. These tests open a menu, click an entry and
//! check that the command reaches the application layer.

use egui::{Context, Event, PointerButton, Pos2, RawInput, Vec2};
use mqtt_ui::panels::menu_bar::{self, Entry};
use mqtt_ui::state::{AppCommand, UiActions, UiState};

/// The centre of an entry, in screen coordinates.
fn centre(entry: &Entry) -> Pos2 {
    entry.response.rect.center()
}

/// Run a frame and throw away the textures it produced.
fn frame(ctx: &Context, input: RawInput, run: impl FnMut(&mut egui::Ui)) {
    let output = ctx.run_ui(input, run);
    output.drop_without_applying_deltas();
}

/// The menus that take the UI state.
fn file_menu(ui: &mut egui::Ui, state: &mut UiState, actions: &mut UiActions) -> Vec<Entry> {
    menu_bar::file_menu(ui, state, actions)
}

fn tools_menu(ui: &mut egui::Ui, state: &mut UiState, actions: &mut UiActions) -> Vec<Entry> {
    menu_bar::tools_menu(ui, state, actions)
}

/// The menus that do not.
fn edit_menu(ui: &mut egui::Ui, _state: &mut UiState, actions: &mut UiActions) -> Vec<Entry> {
    menu_bar::edit_menu(ui, actions)
}

fn connection_menu(ui: &mut egui::Ui, _state: &mut UiState, actions: &mut UiActions) -> Vec<Entry> {
    menu_bar::connection_menu(ui, actions)
}

fn subscription_menu(
    ui: &mut egui::Ui,
    _state: &mut UiState,
    actions: &mut UiActions,
) -> Vec<Entry> {
    menu_bar::subscription_menu(ui, actions)
}

fn help_menu(ui: &mut egui::Ui, _state: &mut UiState, actions: &mut UiActions) -> Vec<Entry> {
    menu_bar::help_menu(ui, actions)
}

/// Draw a menu and click one of its entries.
///
/// The menu is drawn once to find out where its entries are, then the entry
/// is clicked and the menu is drawn again so that the click is seen.
fn click_entry(
    ctx: &Context,
    entries_of: impl Fn(&mut egui::Ui, &mut UiState, &mut UiActions) -> Vec<Entry>,
    label: &str,
) -> Vec<AppCommand> {
    let mut state = UiState::default();
    let mut actions = UiActions::default();

    let entries = {
        let mut found = Vec::new();
        frame(ctx, RawInput::default(), |ui| {
            found = entries_of(ui, &mut state, &mut actions);
        });
        found
    };

    let entry = entries
        .iter()
        .find(|entry| entry.label == label)
        .expect("the menu has that entry");
    let position = centre(entry);

    // Press on the entry.
    let mut input = RawInput::default();
    input.events.push(Event::PointerMoved(position));
    input.events.push(Event::PointerButton {
        pos: position,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    frame(ctx, input, |ui| {
        entries_of(ui, &mut state, &mut actions);
    });

    // Release on the entry: this is what `clicked()` reports.
    let mut input = RawInput::default();
    input.events.push(Event::PointerButton {
        pos: position,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    frame(ctx, input, |ui| {
        entries_of(ui, &mut state, &mut actions);
    });

    actions.commands
}

#[test]
fn the_file_menu_creates_a_connection() {
    let ctx = Context::default();
    let commands = click_entry(&ctx, file_menu, "新建连接…");
    assert_eq!(commands, vec![AppCommand::NewConnection]);
}

#[test]
fn the_file_menu_quits() {
    let ctx = Context::default();
    let commands = click_entry(&ctx, file_menu, "退出");
    assert_eq!(commands, vec![AppCommand::Quit]);
}

#[test]
fn the_edit_menu_copies_the_topic() {
    let ctx = Context::default();
    let commands = click_entry(&ctx, edit_menu, "复制 Topic");
    assert_eq!(commands, vec![AppCommand::CopyTopic]);
}

#[test]
fn the_edit_menu_clears_the_messages() {
    let ctx = Context::default();
    let commands = click_entry(&ctx, edit_menu, "清除消息列表");
    assert_eq!(commands, vec![AppCommand::ClearMessages]);
}

#[test]
fn the_connection_menu_connects() {
    let ctx = Context::default();
    let commands = click_entry(&ctx, connection_menu, "连接");
    assert_eq!(commands, vec![AppCommand::Connect(String::new())]);
}

#[test]
fn the_subscription_menu_subscribes() {
    let ctx = Context::default();
    let commands = click_entry(&ctx, subscription_menu, "新建订阅…");
    assert_eq!(commands, vec![AppCommand::NewSubscription(String::new())]);
}

#[test]
fn the_help_menu_opens_the_about_window() {
    let ctx = Context::default();
    let commands = click_entry(&ctx, help_menu, "关于 MQTT Assistant");
    assert_eq!(commands, vec![AppCommand::About]);
}

#[test]
fn the_tools_menu_formats_the_json() {
    let ctx = Context::default();
    let commands = click_entry(&ctx, tools_menu, "格式化 JSON");
    assert_eq!(commands, vec![AppCommand::FormatJson]);
}

#[test]
fn clicking_nothing_produces_no_command() {
    let ctx = Context::default();
    let mut state = UiState::default();
    let mut actions = UiActions::default();

    // Draw the menu without touching the mouse.
    frame(&ctx, RawInput::default(), |ui| {
        help_menu(ui, &mut state, &mut actions);
    });

    assert!(actions.commands.is_empty());
}

/// The menus never draw outside the window.
#[test]
fn the_menu_bar_fits_in_the_window() {
    let ctx = Context::default();
    let mut state = UiState::default();
    let mut actions = UiActions::default();

    let output = ctx.run_ui(
        RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(800.0, 600.0),
            )),
            ..Default::default()
        },
        |ui| {
            menu_bar::menus(ui, &mut state, &mut actions);
        },
    );
    output.drop_without_applying_deltas();
}
