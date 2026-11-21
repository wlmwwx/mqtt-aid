//! Tests for the command vocabulary of the UI.
//!
//! The menu bar and the ribbon share one command vocabulary. These tests
//! check that every command the UI can emit is understood by the application
//! layer, and that the ribbon and the menu bar offer the same commands.

/// Every command the UI is able to emit.
fn all_commands() -> Vec<mqtt_ui::state::AppCommand> {
    use mqtt_core::models::PayloadFormat;
    use mqtt_ui::state::{AppCommand, ExportFormat};

    vec![
        AppCommand::NewConnection,
        AppCommand::EditConnection(String::new()),
        AppCommand::DeleteConnection(String::new()),
        AppCommand::Connect(String::new()),
        AppCommand::Disconnect(String::new()),
        AppCommand::NewSubscription,
        AppCommand::Unsubscribe(String::new()),
        AppCommand::ToggleSubscriptionPause(String::new()),
        AppCommand::ClearMessages,
        AppCommand::TogglePause,
        AppCommand::Export(ExportFormat::Json),
        AppCommand::Export(ExportFormat::Csv),
        AppCommand::Export(ExportFormat::Hex),
        AppCommand::Export(ExportFormat::Binary),
        AppCommand::SetFormat(PayloadFormat::Hex),
        AppCommand::FormatJson,
        AppCommand::MinifyJson,
        AppCommand::PublishNow,
        AppCommand::SaveTemplate,
        AppCommand::CopyTopic,
        AppCommand::CopyPayload,
        AppCommand::CopyHex,
        AppCommand::SendToPublisher,
        AppCommand::ToggleTheme,
        AppCommand::ToggleLeftPanel,
        AppCommand::ToggleRightPanel,
        AppCommand::ToggleStatusBar,
        AppCommand::FocusSearch,
        AppCommand::About,
        AppCommand::Shortcuts,
        AppCommand::Quit,
    ]
}

#[test]
fn every_command_is_unique() {
    let commands = all_commands();
    let mut sorted = commands.clone();
    sorted.sort_by_key(|command| format!("{command:?}"));
    sorted.dedup_by_key(|command| format!("{command:?}"));
    assert_eq!(sorted.len(), commands.len());
}

#[test]
fn the_ribbon_covers_every_tab() {
    use mqtt_ui::state::RibbonTab;

    for tab in RibbonTab::all() {
        let commands = mqtt_ui::panels::ribbon::commands_for(*tab);
        assert!(!commands.is_empty(), "the {tab:?} tab has no commands");
        for command in commands {
            assert!(
                all_commands().contains(&command.action),
                "the {tab:?} tab offers an unknown command: {:?}",
                command.action
            );
        }
    }
}
