//! The status bar at the bottom of the window.
//!
//! Like SolidCraft's bottom dock, it has a header with the panel title and
//! the counters below it.

use egui::{RichText, Ui};

use crate::i18n;
use crate::state::{ConnectionView, UiActions, UiState};
use crate::widgets::panel_header::{self, HeaderAction};

/// Connection, counters and cache size.
pub fn status_bar(
    ui: &mut Ui,
    state: &mut UiState,
    connections: &[ConnectionView],
    actions: &mut UiActions,
) {
    let action = panel_header::panel_header(ui, i18n::t("状态"), |ui| {
        if ui.button(i18n::t("清除消息列表")).clicked() {
            actions
                .commands
                .push(crate::state::AppCommand::ClearMessages);
        }
    });
    if action == HeaderAction::Collapse {
        state.status_collapsed = !state.status_collapsed;
    }

    if state.status_collapsed {
        return;
    }

    let current = state
        .selected_connection
        .as_ref()
        .and_then(|id| connections.iter().find(|view| &view.config.id == id));

    let name = current
        .map(|view| view.config.name.clone())
        .unwrap_or_else(|| "—".to_string());
    let status = current
        .map(|view| crate::state::connection_status_label(view.status))
        .unwrap_or(i18n::t("未连接"));

    let dim = ui.visuals().weak_text_color();

    ui.horizontal(|ui| {
        ui.label(RichText::new(i18n::fill(i18n::t("连接：{}"), &[&name])).color(dim));
        ui.label(RichText::new(i18n::fill(i18n::t("状态：{}"), &[status])).color(dim));
        ui.label(
            RichText::new(i18n::fill(
                i18n::t("接收：{}"),
                &[&state.stats.received.to_string()],
            ))
            .color(dim),
        );
        ui.label(
            RichText::new(i18n::fill(
                i18n::t("发布：{}"),
                &[&state.stats.published.to_string()],
            ))
            .color(dim),
        );
        ui.label(
            RichText::new(i18n::fill(
                i18n::t("{} 条/秒"),
                &[&format!("{:.1}", state.stats.messages_per_second)],
            ))
            .color(dim),
        );
        ui.label(
            RichText::new(i18n::fill(
                i18n::t("缓存：{}"),
                &[&state.stats.cached.to_string()],
            ))
            .color(dim),
        );

        if state.paused {
            ui.label(RichText::new(i18n::t("⏸ 已暂停")).color(ui.visuals().warn_fg_color));
        }
    });
}
