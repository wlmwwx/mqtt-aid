//! The about window and the shortcut list.

use egui::RichText;

use crate::panels::app_bar::APP_NAME;
use crate::state::UiState;

/// The about window.
pub fn about_window(ctx: &egui::Context, state: &mut UiState) {
    if !state.show_about {
        return;
    }

    let mut close = false;

    egui::Window::new("关于")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_width(360.0);
            ui.label(RichText::new(APP_NAME).strong().size(18.0));
            ui.label("MQTT 3.1.1 / 5.0 桌面调试工具");
            ui.separator();
            ui.label("连接 · 订阅 · 发布 · 实时监控");
            ui.label("消息负载永远是原始字节：Text / HEX / JSON / Binary / Base64 都只是视图。");
            ui.separator();
            ui.label(
                RichText::new("v1.0.0")
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
            if ui.button("关闭").clicked() {
                close = true;
            }
        });

    if close {
        state.show_about = false;
    }
}

/// The keyboard shortcuts.
pub fn shortcuts_window(ctx: &egui::Context, state: &mut UiState) {
    if !state.show_shortcuts {
        return;
    }

    let mut close = false;

    egui::Window::new("快捷键")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_width(340.0);
            for (keys, description) in [
                ("Ctrl+F", "聚焦搜索框"),
                ("Ctrl+E", "导出消息"),
                ("Ctrl+K", "清除消息列表"),
                ("Space", "暂停 / 继续接收"),
                ("Ctrl+,", "设置"),
            ] {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(keys).monospace().strong());
                    ui.label(description);
                });
            }
            ui.separator();
            if ui.button("关闭").clicked() {
                close = true;
            }
        });

    if close {
        state.show_shortcuts = false;
    }
}
