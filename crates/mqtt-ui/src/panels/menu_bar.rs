//! The menu bar.
//!
//! A classic desktop menu bar: `文件`, `编辑`, `视图`, `连接`, `订阅`, `工具`
//! and `帮助`, each with the commands that belong to it. It is drawn inside
//! the application bar, the way SolidCraft puts its `文件` menu in the title
//! bar row.
//!
//! Every menu lives in its own function, and every menu returns the entries
//! it drew. That way the contents of a menu can be exercised on their own.

use egui::{Response, RichText, Ui};

use mqtt_core::models::PayloadFormat;

use crate::i18n;
use crate::state::{AppCommand, CenterTab, ExportFormat, UiActions, UiState};

/// One entry of a menu: its caption and the button that draws it.
pub struct Entry {
    pub label: &'static str,
    pub response: Response,
}

/// Draw the menu bar inside `ui`, which is expected to be a horizontal layout.
///
/// The menu named by `state.open_menu` is held open, which is how the
/// screenshot helper gets a picture of an open dropdown.
pub fn menus(ui: &mut Ui, state: &mut UiState, actions: &mut UiActions) {
    let open = state.open_menu.clone();

    ui.horizontal(|ui| {
        menu_button(
            ui,
            i18n::t("文件"),
            open.as_deref() == Some("文件"),
            |ui| {
                file_menu(ui, state, actions);
            },
        );
        menu_button(
            ui,
            i18n::t("编辑"),
            open.as_deref() == Some("编辑"),
            |ui| {
                edit_menu(ui, actions);
            },
        );
        menu_button(
            ui,
            i18n::t("视图"),
            open.as_deref() == Some("视图"),
            |ui| {
                view_menu(ui, state, actions);
            },
        );
        menu_button(
            ui,
            i18n::t("连接"),
            open.as_deref() == Some("连接"),
            |ui| {
                connection_menu(ui, actions);
            },
        );
        menu_button(
            ui,
            i18n::t("订阅"),
            open.as_deref() == Some("订阅"),
            |ui| {
                subscription_menu(ui, actions);
            },
        );
        menu_button(
            ui,
            i18n::t("工具"),
            open.as_deref() == Some("工具"),
            |ui| {
                tools_menu(ui, state, actions);
            },
        );
        menu_button(
            ui,
            i18n::t("帮助"),
            open.as_deref() == Some("帮助"),
            |ui| {
                help_menu(ui, actions);
            },
        );
    });
}

/// A menu button: a button that opens a dropdown.
///
/// `held_open` keeps the dropdown open, which the screenshot helper uses.
fn menu_button(ui: &mut Ui, title: &str, held_open: bool, add_contents: impl FnOnce(&mut Ui)) {
    let response = ui.add(egui::Button::new(title));

    let open = if held_open {
        Some(egui::SetOpenCommand::Bool(true))
    } else {
        response.clicked().then_some(egui::SetOpenCommand::Toggle)
    };

    egui::Popup::menu(&response)
        .open_memory(open)
        .show(add_contents);
}

/// The contents of the `文件` menu.
pub fn file_menu(ui: &mut Ui, _state: &mut UiState, actions: &mut UiActions) -> Vec<Entry> {
    let mut entries = Vec::new();

    let label = "新建连接…";
    entries.push(Entry {
        label,
        response: ui
            .button(i18n::t(label))
            .on_hover_text(i18n::t("创建一个新的 Broker 连接")),
    });
    if entries[0].response.clicked() {
        actions.commands.push(AppCommand::NewConnection);
    }

    ui.separator();

    let label = "导出消息…";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[1].response.clicked() {
        actions
            .commands
            .push(AppCommand::Export(ExportFormat::Json));
    }

    ui.separator();

    let label = "退出";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[2].response.clicked() {
        actions.commands.push(AppCommand::Quit);
    }

    entries
}

/// The contents of the `编辑` menu.
pub fn edit_menu(ui: &mut Ui, actions: &mut UiActions) -> Vec<Entry> {
    let mut entries = Vec::new();

    let label = "清除消息列表";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[0].response.clicked() {
        actions.commands.push(AppCommand::ClearMessages);
    }

    ui.separator();

    let label = "复制 Topic";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[1].response.clicked() {
        actions.commands.push(AppCommand::CopyTopic);
    }

    let label = "复制 Payload";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[2].response.clicked() {
        actions.commands.push(AppCommand::CopyPayload);
    }

    let label = "复制 HEX";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[3].response.clicked() {
        actions.commands.push(AppCommand::CopyHex);
    }

    entries
}

/// The contents of the `视图` menu.
pub fn view_menu(ui: &mut Ui, state: &mut UiState, actions: &mut UiActions) -> Vec<Entry> {
    let mut entries = Vec::new();

    ui.checkbox(&mut state.show_left, i18n::t("左栏"));
    ui.checkbox(&mut state.show_right, i18n::t("右栏"));
    ui.checkbox(&mut state.show_status, i18n::t("状态栏"));
    ui.separator();

    let label = "消息面板";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[0].response.clicked() {
        actions
            .commands
            .push(AppCommand::SetCenterTab(CenterTab::Message));
    }

    let label = "数据面板";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[1].response.clicked() {
        actions
            .commands
            .push(AppCommand::SetCenterTab(CenterTab::Data));
    }

    ui.separator();

    let label = "深色主题";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[2].response.clicked() {
        actions.commands.push(AppCommand::ToggleTheme);
    }

    let label = "浅色主题";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[3].response.clicked() {
        actions.commands.push(AppCommand::ToggleTheme);
    }

    entries
}

/// The contents of the `连接` menu.
pub fn connection_menu(ui: &mut Ui, actions: &mut UiActions) -> Vec<Entry> {
    let mut entries = Vec::new();

    let label = "新建连接…";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[0].response.clicked() {
        actions.commands.push(AppCommand::NewConnection);
    }

    let label = "连接";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[1].response.clicked() {
        actions.commands.push(AppCommand::Connect(String::new()));
    }

    let label = "断开";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[2].response.clicked() {
        actions.commands.push(AppCommand::Disconnect(String::new()));
    }

    entries
}

/// The contents of the `订阅` menu.
pub fn subscription_menu(ui: &mut Ui, actions: &mut UiActions) -> Vec<Entry> {
    let mut entries = Vec::new();

    let label = "新建订阅…";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[0].response.clicked() {
        actions
            .commands
            .push(AppCommand::NewSubscription(String::new()));
    }

    let label = "取消订阅…";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[1].response.clicked() {
        actions
            .commands
            .push(AppCommand::Unsubscribe(String::new()));
    }

    let label = "暂停 / 恢复";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[2].response.clicked() {
        actions
            .commands
            .push(AppCommand::ToggleSubscriptionPause(String::new()));
    }

    entries
}

/// The contents of the `工具` menu.
pub fn tools_menu(ui: &mut Ui, state: &mut UiState, actions: &mut UiActions) -> Vec<Entry> {
    let mut entries = Vec::new();

    ui.label(RichText::new(i18n::t("消息格式")).small());
    for format in [
        PayloadFormat::Auto,
        PayloadFormat::Text,
        PayloadFormat::Hex,
        PayloadFormat::Json,
        PayloadFormat::Base64,
        PayloadFormat::Binary,
    ] {
        let selected = state.message_format == format;
        let response = ui.selectable_label(selected, crate::state::format_label(format));
        if response.clicked() {
            actions.commands.push(AppCommand::SetFormat(format));
        }
    }
    ui.separator();

    let label = "格式化 JSON";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[0].response.clicked() {
        actions.commands.push(AppCommand::FormatJson);
    }

    let label = "压缩 JSON";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[1].response.clicked() {
        actions.commands.push(AppCommand::MinifyJson);
    }

    ui.separator();

    let label = "查找…";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[2].response.clicked() {
        actions.commands.push(AppCommand::FocusSearch);
    }

    entries
}

/// The contents of the `帮助` menu.
pub fn help_menu(ui: &mut Ui, actions: &mut UiActions) -> Vec<Entry> {
    let mut entries = Vec::new();

    let label = "快捷键";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[0].response.clicked() {
        actions.commands.push(AppCommand::Shortcuts);
    }

    let label = "关于 MQTT Assistant";
    entries.push(Entry {
        label,
        response: ui.button(i18n::t(label)),
    });
    if entries[1].response.clicked() {
        actions.commands.push(AppCommand::About);
    }

    entries
}

/// The context menu of the message list.
pub fn message_context_menu(ui: &mut Ui, actions: &mut UiActions) {
    ui.menu_button(i18n::t("更多"), |ui| {
        if ui.button(i18n::t("复制 Topic")).clicked() {
            actions.commands.push(AppCommand::CopyTopic);
        }
        if ui.button(i18n::t("复制 Payload")).clicked() {
            actions.commands.push(AppCommand::CopyPayload);
        }
        if ui.button(i18n::t("复制 HEX")).clicked() {
            actions.commands.push(AppCommand::CopyHex);
        }
        ui.separator();
        if ui.button(i18n::t("发送到发布面板")).clicked() {
            actions.commands.push(AppCommand::SendToPublisher);
        }
    });
}
