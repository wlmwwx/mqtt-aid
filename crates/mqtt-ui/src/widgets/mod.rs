//! Reusable widgets for the message monitor.

pub mod icons;
pub mod panel_header;
pub mod payload_views;
pub mod tabs;

use egui::{RichText, Ui};
use mqtt_core::models::{MqttMessage, PayloadFormat};

use crate::state::{CopyKind, CopyRequest, UiActions, UiState};
use crate::theme::spacing;

pub use payload_views::{hex_view, json_text_view, json_view, mono_text, monospace, preview};

/// Format an epoch millisecond stamp as `MM-DD HH:MM:SS` (UTC).
pub fn format_time(millis: i64) -> String {
    let (year, month, day, hour, minute, second) = parts(millis);
    let _ = year;
    format!("{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
}

/// Format an epoch millisecond stamp as `YYYY-MM-DD HH:MM:SS` (UTC).
pub fn format_stamp(millis: i64) -> String {
    let (year, month, day, hour, minute, second) = parts(millis);
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
}

/// The parts of a date in UTC: the year, the month, the day and the time.
fn parts(millis: i64) -> (i64, u32, u32, u32, u32, u32) {
    let seconds = millis.div_euclid(1000);
    let days = seconds.div_euclid(86_400);
    let time_of_day = seconds.rem_euclid(86_400);

    // The day of the month out of the days since the epoch, the way
    // everybody does it since 1970.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_of_year = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_of_year + 2) / 5 + 1) as u32;
    let month = if month_of_year < 10 {
        month_of_year + 3
    } else {
        month_of_year - 9
    } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);

    (
        year,
        month,
        day,
        (time_of_day / 3600) as u32,
        ((time_of_day % 3600) / 60) as u32,
        (time_of_day % 60) as u32,
    )
}

/// The widths of the columns of the message table.
struct Columns {
    time: f32,
    topic: f32,
    qos: f32,
    retain: f32,
    size: f32,
    preview: f32,
}

/// The width of the time column.
const TIME_WIDTH: f32 = 96.0;
/// The width of the QoS column.
const QOS_WIDTH: f32 = 34.0;
/// The width of the retain column.
const RETAIN_WIDTH: f32 = 38.0;
/// The width of the size column.
const SIZE_WIDTH: f32 = 72.0;
/// The gap between two columns.
const COLUMN_GAP: f32 = 8.0;

/// The width of the columns, for a table of the given width.
///
/// The topic and the preview of the payload share what is left: the topic
/// gets a bit less than half of it, because a payload is easier to cut short
/// than a topic.
fn columns(width: f32) -> Columns {
    let gaps = 5.0 * COLUMN_GAP;
    let rest = (width - TIME_WIDTH - QOS_WIDTH - RETAIN_WIDTH - SIZE_WIDTH - gaps).max(120.0);
    let topic = rest * 0.45;
    Columns {
        time: TIME_WIDTH,
        topic,
        qos: QOS_WIDTH,
        retain: RETAIN_WIDTH,
        size: SIZE_WIDTH,
        preview: rest - topic,
    }
}

/// The rects of the cells of a row.
fn cells(row: egui::Rect, columns: &Columns) -> Vec<egui::Rect> {
    let mut rects = Vec::with_capacity(6);
    let mut x = row.left();
    for width in [
        columns.time,
        columns.topic,
        columns.qos,
        columns.retain,
        columns.size,
        columns.preview,
    ] {
        rects.push(egui::Rect::from_min_size(
            egui::pos2(x, row.top()),
            egui::vec2(width, row.height()),
        ));
        x += width + COLUMN_GAP;
    }
    rects
}

/// Draw a cell of the table.
///
/// The text is cut short with an ellipsis when it does not fit, so that a
/// long topic never pushes the other columns out of the panel.
fn cell(ui: &mut Ui, rect: egui::Rect, text: impl Into<egui::WidgetText>, right: bool) {
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            if right {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(egui::Label::new(text).truncate());
                });
            } else {
                ui.add(egui::Label::new(text).truncate());
            }
        },
    );
}

/// The table of received messages.
pub fn message_table(
    ui: &mut Ui,
    messages: &[MqttMessage],
    state: &mut UiState,
    actions: &mut UiActions,
) {
    let row_height = spacing::ROW_HEIGHT;
    let palette = state.theme.palette();
    let text_dim = ui.visuals().weak_text_color();

    // The names of the columns, so that a row can be read without counting.
    table_header(ui, &palette, text_dim);

    // The table gets its own scroll area, and the detail view below it the
    // rest of the panel. Both need their own id, or egui complains about a
    // doubled id.
    let table_height = (ui.available_height() - spacing::DETAILS_HEIGHT).max(60.0);
    egui::ScrollArea::vertical()
        .id_salt("message_table")
        // The table fills the width of the panel: a scroll area that shrinks
        // to its content would put its scroll bar in the middle of the
        // panel instead of at its edge.
        .auto_shrink([false, true])
        .max_height(table_height)
        .show_rows(ui, row_height, messages.len(), |ui, range| {
            for index in range {
                message_row(ui, index, &messages[index], state, actions);
            }
        });

    if state.selected_message.is_some() {
        let index = state.selected_message.unwrap();
        if let Some(message) = messages.get(index) {
            ui.separator();
            message_details(ui, message, state, actions);
        }
    }
}

/// The names of the columns of the table.
fn table_header(ui: &mut Ui, palette: &crate::theme::Palette, text_dim: egui::Color32) {
    let columns = columns(ui.available_width());
    let header = egui::Rect::from_min_size(
        ui.cursor().left_top(),
        egui::vec2(ui.available_width(), spacing::ROW_HEIGHT),
    );
    ui.allocate_space(header.size());

    let rects = cells(header, &columns);
    let caption = |text: &str| egui::RichText::new(text).small().color(text_dim).strong();
    cell(ui, rects[0], caption("时间"), false);
    cell(ui, rects[1], caption("主题"), false);
    cell(ui, rects[2], caption("QoS"), false);
    cell(ui, rects[3], caption("保留"), false);
    cell(ui, rects[4], caption("大小"), true);
    cell(ui, rects[5], caption("负载"), false);

    // The hairline under the names of the columns.
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::ZERO, palette.border);
    }
}

/// One row of the table.
///
/// The background of the row goes down before its text, so that the text
/// stays readable: a row that is painted over its own labels looks like a
/// glitch.
fn message_row(
    ui: &mut Ui,
    index: usize,
    message: &MqttMessage,
    state: &mut UiState,
    actions: &mut UiActions,
) {
    let row_height = spacing::ROW_HEIGHT;
    let palette = state.theme.palette();
    let text_dim = ui.visuals().weak_text_color();
    let selected = state.selected_message == Some(index);

    let row_rect = egui::Rect::from_min_size(
        ui.cursor().left_top(),
        egui::vec2(ui.available_width(), row_height),
    );
    ui.allocate_space(row_rect.size());

    let response = ui.interact(
        row_rect,
        ui.scope_id().with(("message", index)),
        egui::Sense::click(),
    );
    if response.clicked() {
        state.selected_message = Some(index);
        // The copy commands of the menu bar act on this message.
        state.copy_target = Some(message.clone());
    }
    // The right mouse button opens the context menu of the row.
    response.context_menu(|ui| {
        crate::panels::menu_bar::message_context_menu(ui, actions);
    });

    // The background of the row, under its text.
    let painter = ui.painter();
    if selected {
        painter.rect_filled(
            row_rect,
            egui::CornerRadius::ZERO,
            palette.accent.gamma_multiply(0.28),
        );
        // A bar of the accent colour at the left, so that the selected row
        // is found in a long list at a glance.
        painter.rect_filled(
            egui::Rect::from_min_size(row_rect.left_top(), egui::vec2(2.0, row_height)),
            egui::CornerRadius::ZERO,
            palette.accent,
        );
    } else if response.hovered() {
        painter.rect_filled(row_rect, egui::CornerRadius::ZERO, palette.row_hover);
    }

    let columns = columns(row_rect.width());
    let rects = cells(row_rect, &columns);

    cell(
        ui,
        rects[0],
        egui::RichText::new(format_time(message.received_at))
            .monospace()
            .color(text_dim),
        false,
    );
    cell(
        ui,
        rects[1],
        egui::RichText::new(message.topic.clone()).monospace(),
        false,
    );
    cell(
        ui,
        rects[2],
        egui::RichText::new(message.qos.to_string())
            .monospace()
            .color(text_dim),
        false,
    );
    cell(
        ui,
        rects[3],
        egui::RichText::new(if message.retain { "R" } else { "" }).color(palette.warning),
        false,
    );
    cell(
        ui,
        rects[4],
        egui::RichText::new(format!("{} B", message.payload.len()))
            .monospace()
            .color(text_dim),
        true,
    );
    cell(
        ui,
        rects[5],
        egui::RichText::new(preview(&message.payload, 64))
            .monospace()
            .color(text_dim),
        false,
    );
}

/// The detail view of the selected message.
fn message_details(
    ui: &mut Ui,
    message: &MqttMessage,
    state: &mut UiState,
    actions: &mut UiActions,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(message.topic.clone()).strong());
        ui.label(
            RichText::new(format!(
                "{} · QoS {} · {} · {} bytes · MQTT {}",
                format_stamp(message.received_at),
                message.qos,
                if message.retain { "retained" } else { "live" },
                message.payload.len(),
                message.version.label()
            ))
            .color(ui.visuals().weak_text_color()),
        );
    });

    ui.horizontal(|ui| {
        if ui.button("Copy topic").clicked() {
            actions.copy.push(CopyRequest {
                kind: CopyKind::Topic,
                value: message.topic.clone(),
            });
        }
        if ui.button("Copy payload").clicked() {
            actions.copy.push(CopyRequest {
                kind: CopyKind::Payload,
                value: String::from_utf8_lossy(&message.payload).to_string(),
            });
        }
        if ui.button("Copy hex").clicked() {
            actions.copy.push(CopyRequest {
                kind: CopyKind::Hex,
                value: mqtt_core::format::encode_hex(&message.payload),
            });
        }
        if ui.button("Send to publisher").clicked() {
            actions.send_to_publisher = Some(message.clone());
        }
    });

    if let Some(properties) = &message.properties {
        egui::CollapsingHeader::new("MQTT 5.0 properties")
            .default_open(false)
            .show(ui, |ui| {
                if let Some(content_type) = &properties.content_type {
                    ui.label(format!("Content type: {content_type}"));
                }
                if let Some(indicator) = properties.payload_format_indicator {
                    ui.label(format!("Payload format indicator: {indicator}"));
                }
                if let Some(expiry) = properties.message_expiry_interval {
                    ui.label(format!("Message expiry interval: {expiry} s"));
                }
                if let Some(response_topic) = &properties.response_topic {
                    ui.label(format!("Response topic: {response_topic}"));
                }
                if let Some(correlation) = &properties.correlation_data {
                    ui.label(format!(
                        "Correlation data: {}",
                        mqtt_core::format::encode_hex(correlation)
                    ));
                }
                for (key, value) in &properties.user_properties {
                    ui.label(format!("User property {key} = {value}"));
                }
            });
    }

    ui.horizontal(|ui| {
        ui.label("Format:");
        for format in [
            PayloadFormat::Auto,
            PayloadFormat::Text,
            PayloadFormat::Hex,
            PayloadFormat::Json,
            PayloadFormat::Base64,
            PayloadFormat::Binary,
        ] {
            let selected = state.message_format == format;
            if ui.selectable_label(selected, format.label()).clicked() {
                state.message_format = format;
            }
        }
    });

    let resolved = state.message_format.resolve(&message.payload);
    match resolved {
        PayloadFormat::Hex => hex_view(ui, &message.payload),
        PayloadFormat::Json => {
            let text = String::from_utf8_lossy(&message.payload).to_string();
            json_text_view(ui, &text);
        }
        PayloadFormat::Base64 => {
            let mut text = mqtt_core::format::codec_for(PayloadFormat::Base64)
                .decode(&message.payload)
                .unwrap_or_default();
            monospace(ui, &mut text);
        }
        PayloadFormat::Binary => {
            let mut text = mqtt_core::format::codec_for(PayloadFormat::Binary)
                .decode(&message.payload)
                .unwrap_or_default();
            monospace(ui, &mut text);
        }
        _ => {
            let mut text = String::from_utf8_lossy(&message.payload).to_string();
            monospace(ui, &mut text);
        }
    }
}
