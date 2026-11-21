//! Headless tests of the ribbon.
//!
//! The ribbon is the toolbar: a row of category tabs, each with the commands
//! of that category. These tests click a command and check that the command
//! reaches the application layer.

use egui::{Context, Event, PointerButton, Pos2, RawInput, Vec2};
use mqtt_ui::panels::ribbon;
use mqtt_ui::state::{AppCommand, RibbonTab, UiActions, UiState};

/// Run a frame and throw away the textures it produced.
fn frame(ctx: &Context, input: RawInput, run: impl FnMut(&mut egui::Ui)) {
    let output = ctx.run_ui(input, run);
    output.drop_without_applying_deltas();
}

/// Press the mouse at a position.
fn press_at(position: Pos2) -> RawInput {
    let mut input = RawInput::default();
    input.events.push(Event::PointerMoved(position));
    input.events.push(Event::PointerButton {
        pos: position,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    input
}

/// Release the mouse at a position.
fn release_at(position: Pos2) -> RawInput {
    let mut input = RawInput::default();
    input.events.push(Event::PointerMoved(position));
    input.events.push(Event::PointerButton {
        pos: position,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    input
}

/// Draw the ribbon and report where each command landed.
fn commands_of(ctx: &Context, tab: RibbonTab) -> Vec<(String, Pos2)> {
    let mut state = UiState {
        ribbon_tab: tab,
        ..Default::default()
    };
    let mut actions = UiActions::default();

    let mut placed = Vec::new();
    frame(ctx, RawInput::default(), |ui| {
        placed = ribbon::ribbon(ui, &mut state, &mut actions);
    });

    placed
        .into_iter()
        .map(|(label, rect)| (label, rect.center()))
        .collect()
}

/// Click a command of a tab and return the commands it produced.
fn click_command(ctx: &Context, tab: RibbonTab, label: &str) -> Vec<AppCommand> {
    let mut state = UiState {
        ribbon_tab: tab,
        ..Default::default()
    };
    let mut actions = UiActions::default();

    let positions = commands_of(ctx, tab);
    let position = positions
        .iter()
        .find(|(name, _)| name == label)
        .map(|(_, position)| *position)
        .unwrap_or_else(|| panic!("the {tab:?} tab has a {label} command"));

    // The ribbon has to be drawn in the frame of the press and in the frame
    // of the release, otherwise the click is never seen.
    frame(ctx, press_at(position), |ui| {
        ribbon::ribbon(ui, &mut state, &mut actions);
    });
    frame(ctx, release_at(position), |ui| {
        ribbon::ribbon(ui, &mut state, &mut actions);
    });

    actions.commands
}

#[test]
fn the_connection_tab_creates_a_connection() {
    let ctx = Context::default();
    let commands = click_command(&ctx, RibbonTab::Connection, "新建连接");
    assert_eq!(commands, vec![AppCommand::NewConnection]);
}

#[test]
fn the_connection_tab_connects() {
    let ctx = Context::default();
    let commands = click_command(&ctx, RibbonTab::Connection, "连接");
    assert_eq!(commands, vec![AppCommand::Connect(String::new())]);
}

#[test]
fn the_message_tab_clears_the_messages() {
    let ctx = Context::default();
    let commands = click_command(&ctx, RibbonTab::Message, "清除");
    assert_eq!(commands, vec![AppCommand::ClearMessages]);
}

#[test]
fn the_publish_tab_publishes() {
    let ctx = Context::default();
    let commands = click_command(&ctx, RibbonTab::Publish, "发布");
    assert_eq!(commands, vec![AppCommand::PublishNow]);
}

#[test]
fn the_view_tab_hides_the_left_panel() {
    let ctx = Context::default();
    let commands = click_command(&ctx, RibbonTab::View, "左栏");
    assert_eq!(commands, vec![AppCommand::ToggleLeftPanel]);
}

#[test]
fn every_tab_has_commands_that_fit_in_the_ribbon() {
    let ctx = Context::default();
    for tab in RibbonTab::all() {
        let positions = commands_of(&ctx, *tab);
        assert!(!positions.is_empty(), "the {tab:?} tab has no commands");
        for (name, position) in positions {
            assert!(
                position.x > 0.0 && position.x < 1280.0,
                "the {name} command of the {tab:?} tab is outside the window: {position:?}"
            );
        }
    }
}

/// The ribbon does not draw outside the window.
#[test]
fn the_ribbon_fits_in_the_window() {
    let ctx = Context::default();
    let mut state = UiState::default();
    let mut actions = UiActions::default();

    let output = ctx.run_ui(
        RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(1280.0, 800.0),
            )),
            ..Default::default()
        },
        |ui| {
            ribbon::ribbon(ui, &mut state, &mut actions);
        },
    );
    output.drop_without_applying_deltas();
}
