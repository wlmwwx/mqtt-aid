//! The left panel: connections and subscriptions.
//!
//! It follows SolidCraft's browser panel: a header with a grip, the panel
//! title and an overflow menu, and below it a tree of connections with their
//! subscriptions underneath.

use egui::{Color32, RichText, Ui};
use mqtt_core::models::{ConnectionStatus, MqttSubscription, PayloadFormat};

use crate::i18n;
use crate::state::{AppCommand, ConnectionView, UiActions, UiState};
use crate::theme::spacing;
use crate::widgets::icons::{self, Icon};
use crate::widgets::panel_header::{self, HeaderAction};

/// The colour of a connection status dot.
pub fn status_color(status: ConnectionStatus, palette: &crate::theme::Palette) -> Color32 {
    match status {
        ConnectionStatus::Connected => palette.success,
        ConnectionStatus::Connecting | ConnectionStatus::Reconnecting => palette.warning,
        ConnectionStatus::Error => palette.danger,
        ConnectionStatus::Disconnected => palette.text_faint,
    }
}

/// How far the subscriptions of a connection are indented.
const INDENT: f32 = 18.0;

/// Where the buttons of the panel are drawn, so that a test can click them.
#[derive(Debug, Default)]
pub struct LeftPanelLayout {
    /// The button that adds a subscription to a connection, by connection id.
    pub subscribe_buttons: Vec<(String, egui::Rect)>,
}

/// The connection list with the subscriptions of each connection under it.
pub fn left_panel(
    ui: &mut Ui,
    state: &mut UiState,
    connections: &[ConnectionView],
    subscriptions: &[MqttSubscription],
    actions: &mut UiActions,
) -> LeftPanelLayout {
    let mut layout = LeftPanelLayout::default();

    let action = panel_header::panel_header(ui, i18n::t("浏览器"), |ui| {
        if ui.button(i18n::t("新建连接…")).clicked() {
            actions.commands.push(AppCommand::NewConnection);
        }
        if ui.button(i18n::t("新建订阅…")).clicked() {
            actions
                .commands
                .push(AppCommand::NewSubscription(String::new()));
        }
    });
    if action == HeaderAction::Collapse {
        state.left_collapsed = !state.left_collapsed;
    }

    ui.add_space(spacing::GAP);

    if state.left_collapsed {
        return layout;
    }

    section(
        ui,
        i18n::t("连接"),
        connections.len(),
        AppCommand::NewConnection,
        actions,
    );

    if connections.is_empty() {
        empty(ui, i18n::t("还没有连接"));
    }

    for view in connections {
        // The subscriptions of a connection live under it: a subscription
        // belongs to one connection, not to the application.
        let own: Vec<&MqttSubscription> = subscriptions
            .iter()
            .filter(|subscription| subscription.connection_id == view.config.id)
            .collect();
        connection_row(ui, state, view, own.len(), actions, &mut layout);

        if !own.is_empty() {
            // The guide of the tree: a hairline the subscriptions hang from.
            let guide_x = ui.cursor().left() + INDENT / 2.0;
            let top = ui.cursor().top();

            ui.indent(("subscriptions", view.config.id.clone()), |ui| {
                for subscription in own {
                    subscription_row(ui, state, subscription, actions);
                }
            });

            let bottom = ui.cursor().top();
            if ui.is_rect_visible(egui::Rect::from_min_max(
                egui::pos2(guide_x, top),
                egui::pos2(guide_x + 1.0, bottom),
            )) {
                ui.painter().vline(
                    guide_x,
                    egui::emath::Rangef::new(top, bottom),
                    egui::Stroke::new(1.0, state.theme.palette().border_strong),
                );
            }
        }

        ui.separator();
    }

    layout
}

/// A section title with a count and an add button.
fn section(ui: &mut Ui, title: &str, count: usize, add: AppCommand, actions: &mut UiActions) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title)
                .strong()
                .size(11.0)
                .color(ui.visuals().weak_text_color()),
        );
        ui.label(
            RichText::new(count.to_string())
                .small()
                .color(ui.visuals().weak_text_color()),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new(RichText::new("+").size(14.0)))
                .on_hover_text(i18n::t("新建"))
                .clicked()
            {
                actions.commands.push(add.clone());
            }
        });
    });
}

/// The height of a row in the tree.
const ROW_HEIGHT: f32 = 24.0;

/// Draw a row of the tree.
///
/// The background of the row goes down before its content, so the content
/// stays readable: a row painted over its own text looks like a glitch.
fn tree_row(
    ui: &mut Ui,
    id: &str,
    selected: bool,
    palette: &crate::theme::Palette,
    add: impl FnOnce(&mut Ui),
) {
    let rect = egui::Rect::from_min_size(
        ui.cursor().left_top(),
        egui::vec2(ui.available_width(), ROW_HEIGHT),
    );
    ui.allocate_space(rect.size());

    let response = ui.interact(
        rect,
        ui.scope_id().with(("tree_row", id)),
        egui::Sense::click(),
    );
    if selected {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(4),
            palette.accent.gamma_multiply(0.25),
        );
        // A bar of the accent colour at the left marks the selected row.
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.left_top(), egui::vec2(2.0, rect.height())),
            egui::CornerRadius::same(4),
            palette.accent,
        );
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(4), palette.row_hover);
    }

    ui.scope_builder(
        egui::UiBuilder::new()
            .id_salt(("tree_row", id))
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        add,
    );
}

/// The placeholder shown when a list is empty.
fn empty(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .small()
            .italics()
            .color(ui.visuals().weak_text_color()),
    );
    ui.add_space(spacing::GAP);
}

/// A connection in the tree.
fn connection_row(
    ui: &mut Ui,
    state: &mut UiState,
    view: &ConnectionView,
    subscription_count: usize,
    actions: &mut UiActions,
    layout: &mut LeftPanelLayout,
) {
    let selected = state.selected_connection.as_ref() == Some(&view.config.id);
    let palette = state.theme.palette();

    tree_row(ui, &view.config.id, selected, &palette, |ui| {
        ui.horizontal(|ui| {
            let color = status_color(view.status, &state.theme.palette());
            panel_header::status_dot(ui, color);

            let palette = state.theme.palette();
            let label = if selected {
                RichText::new(view.config.name.clone())
                    .color(palette.accent_text)
                    .strong()
            } else {
                RichText::new(view.config.name.clone()).color(palette.text)
            };
            let name = ui
                .add(egui::Label::new(label).sense(egui::Sense::click()))
                .on_hover_text(i18n::t("选择连接"));
            if name.clicked() {
                state.selected_connection = Some(view.config.id.clone());
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icons::icon_button(ui, Icon::Trash, i18n::t("删除连接")) {
                    actions.delete_connection.push(view.config.id.clone());
                }
                if icons::icon_button(ui, Icon::Pencil, i18n::t("编辑连接")) {
                    actions
                        .commands
                        .push(AppCommand::EditConnection(view.config.id.clone()));
                }
                // A subscription is made on this connection, not on the
                // application.
                let subscribe = icons::icon_button_response(ui, Icon::Plus, i18n::t("新建订阅"));
                layout
                    .subscribe_buttons
                    .push((view.config.id.clone(), subscribe.rect));
                if subscribe.clicked() {
                    actions
                        .commands
                        .push(AppCommand::NewSubscription(view.config.id.clone()));
                }
                let plug = if view.status == ConnectionStatus::Connected
                    || view.status == ConnectionStatus::Reconnecting
                {
                    Icon::Power
                } else {
                    Icon::Plug
                };
                let tooltip = if view.status == ConnectionStatus::Connected
                    || view.status == ConnectionStatus::Reconnecting
                {
                    i18n::t("断开连接")
                } else {
                    i18n::t("连接")
                };
                if icons::icon_button(ui, plug, tooltip) {
                    if view.status == ConnectionStatus::Connected
                        || view.status == ConnectionStatus::Reconnecting
                    {
                        actions.disconnect.push(view.config.id.clone());
                    } else {
                        actions.connect.push(view.config.id.clone());
                    }
                }
            });
        });
    });

    // The address of the broker and how many subscriptions the connection
    // carries, on a line of their own: the row above is full of buttons.
    ui.horizontal(|ui| {
        ui.add_space(INDENT);
        ui.label(
            RichText::new(format!("{}:{}", view.config.host, view.config.port))
                .small()
                .color(state.theme.palette().text_dim),
        );
        ui.label(
            RichText::new(i18n::fill(
                i18n::t("· {} 订阅"),
                &[&subscription_count.to_string()],
            ))
            .small()
            .color(state.theme.palette().text_faint),
        );
    });
}

/// A subscription in the tree.
fn subscription_row(
    ui: &mut Ui,
    state: &mut UiState,
    subscription: &MqttSubscription,
    actions: &mut UiActions,
) {
    let selected = state.selected_subscription.as_ref() == Some(&subscription.id);
    let palette = state.theme.palette();

    tree_row(ui, &subscription.id, selected, &palette, |ui| {
        ui.horizontal(|ui| {
            icons::draw(
                ui.painter(),
                Icon::Topic,
                egui::Rect::from_center_size(
                    ui.cursor().left_center() + egui::vec2(6.0, 0.0),
                    egui::vec2(12.0, 12.0),
                ),
                ui.visuals().weak_text_color(),
                1.2,
            );
            ui.add_space(12.0);

            let palette = state.theme.palette();
            let label = if subscription.paused {
                RichText::new(subscription.display_name())
                    .color(palette.text_dim)
                    .strikethrough()
            } else {
                RichText::new(subscription.display_name()).color(palette.text)
            };
            let name = ui
                .add(egui::Label::new(label).sense(egui::Sense::click()))
                .on_hover_text(i18n::t("选择订阅"));
            if name.clicked() {
                // The menu bar commands act on the selected subscription, and
                // the connection it belongs to becomes the current one.
                state.selected_subscription = Some(subscription.id.clone());
                state.selected_connection = Some(subscription.connection_id.clone());
            }

            ui.label(
                RichText::new(format!("QoS {}", subscription.qos))
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icons::icon_button(ui, Icon::Trash, i18n::t("取消订阅")) {
                    actions.unsubscribe.push((
                        subscription.connection_id.clone(),
                        subscription.topic.clone(),
                    ));
                }
                let pause = if subscription.paused {
                    Icon::Play
                } else {
                    Icon::Pause
                };
                let tooltip = if subscription.paused {
                    i18n::t("恢复显示")
                } else {
                    i18n::t("暂停显示")
                };
                if icons::icon_button(ui, pause, tooltip) {
                    actions
                        .toggle_subscription_pause
                        .push(subscription.id.clone());
                }
            });
        });
    });

    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt(format!("sub-format-{}", subscription.id))
            .selected_text(crate::state::format_label(subscription.format))
            .width(80.0)
            .show_ui(ui, |ui| {
                for format in [
                    PayloadFormat::Auto,
                    PayloadFormat::Text,
                    PayloadFormat::Hex,
                    PayloadFormat::Json,
                    PayloadFormat::Base64,
                ] {
                    if ui
                        .selectable_label(
                            subscription.format == format,
                            crate::state::format_label(format),
                        )
                        .clicked()
                        && subscription.format != format
                    {
                        actions
                            .set_subscription_format
                            .push((subscription.id.clone(), format));
                    }
                }
            });

        ui.label(
            RichText::new(subscription.topic.clone())
                .small()
                .color(ui.visuals().weak_text_color()),
        );
    });

    ui.horizontal(|ui| {
        for qos in 0..=2u8 {
            let selected = subscription.qos == qos;
            if ui.selectable_label(selected, format!("Q{qos}")).clicked() && !selected {
                actions.set_qos.push((
                    subscription.connection_id.clone(),
                    subscription.topic.clone(),
                    qos,
                ));
            }
        }
    });
}

/// A unique identifier for a new entity.
pub fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("id-{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

/// The default format offered for a new subscription.
pub fn default_subscription_format() -> PayloadFormat {
    PayloadFormat::Auto
}
