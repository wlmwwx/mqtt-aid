//! The data panel: the charts of the data series.
//!
//! A data series watches a topic and reads one number out of every message
//! it receives. The panel lists the series and draws them as time series.

use std::collections::{HashMap, VecDeque};

use egui::{Align, Color32, RichText, Ui};
use egui_plot::{Line, Plot, PlotPoints};
use mqtt_core::models::{DataPoint, DataSeries, MqttSubscription};

use crate::ConnectionView;
use crate::i18n;
use crate::state::{SeriesAction, UiActions, UiState};
use crate::theme::Palette;
use crate::widgets::icons::{self, Icon};

/// The samples of every series, by series id.
pub type Samples = HashMap<String, VecDeque<DataPoint>>;

/// Where the buttons of the panel are drawn, so that a test can click them.
#[derive(Debug)]
pub struct DataPanelLayout {
    /// The button that opens the editor.
    pub add_button: egui::Rect,
    /// The button that deletes a series, by series id.
    pub remove_buttons: Vec<(String, egui::Rect)>,
}

/// Draw the data panel.
pub fn data_panel(
    ui: &mut Ui,
    state: &mut UiState,
    series: &[DataSeries],
    samples: &Samples,
    connections: &[ConnectionView],
    subscriptions: &[MqttSubscription],
    actions: &mut UiActions,
) -> DataPanelLayout {
    let palette = state.theme.palette();
    let mut layout = DataPanelLayout {
        add_button: egui::Rect::ZERO,
        remove_buttons: Vec::new(),
    };

    ui.horizontal(|ui| {
        let response = ui.button(i18n::t("+ 添加序列"));
        layout.add_button = response.rect;
        if response.clicked() {
            state.series_draft = Some(crate::state::SeriesDraft {
                series: DataSeries::new(
                    crate::panels::left_panel::new_id(),
                    i18n::t("新序列"),
                    state
                        .selected_connection
                        .clone()
                        .or_else(|| connections.first().map(|c| c.config.id.clone()))
                        .unwrap_or_default(),
                    String::new(),
                    String::new(),
                ),
                is_new: true,
            });
        }
        ui.label(
            RichText::new(i18n::t("从消息里取值，画成曲线"))
                .small()
                .color(ui.visuals().weak_text_color()),
        );
    });

    ui.separator();

    if series.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                RichText::new(i18n::t("还没有数据序列"))
                    .color(ui.visuals().weak_text_color())
                    .italics(),
            );
        });
    } else {
        // The list of the series.
        egui::ScrollArea::vertical()
            .id_salt("data_series_list")
            // The list fills the width of the panel, so that its scroll bar
            // sits at the edge of the panel.
            .auto_shrink([false, true])
            .max_height(140.0)
            .show(ui, |ui| {
                for (index, entry) in series.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let color = series_color(&palette, index);
                        ui.colored_label(color, "●");
                        ui.label(RichText::new(entry.name.clone()).strong());
                        let connection = connections
                            .iter()
                            .find(|connection| connection.config.id == entry.connection_id)
                            .map(|connection| connection.config.name.clone())
                            .unwrap_or_default();
                        ui.label(
                            RichText::new(format!(
                                "{connection} · {} · {}",
                                entry.topic,
                                path_label(&entry.path)
                            ))
                            .small()
                            .color(ui.visuals().weak_text_color()),
                        );

                        let count = samples.get(&entry.id).map_or(0, |points| points.len());
                        ui.label(
                            RichText::new(i18n::fill(i18n::t("{} 点"), &[&count.to_string()]))
                                .small()
                                .color(ui.visuals().weak_text_color()),
                        );
                        if let Some(last) = samples.get(&entry.id).and_then(|points| points.back())
                        {
                            ui.label(
                                RichText::new(i18n::fill(
                                    i18n::t("最新 {:.3}"),
                                    &[&format!("{:.3}", last.value)],
                                ))
                                .small()
                                .color(color),
                            );
                        }

                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            let remove =
                                icons::icon_button_response(ui, Icon::Trash, i18n::t("删除序列"));
                            layout.remove_buttons.push((entry.id.clone(), remove.rect));
                            if remove.clicked() {
                                actions.series.push(SeriesAction::Remove(entry.id.clone()));
                            }
                            if icons::icon_button(ui, Icon::Pencil, i18n::t("编辑序列")) {
                                state.series_draft = Some(crate::state::SeriesDraft {
                                    series: entry.clone(),
                                    is_new: false,
                                });
                            }
                        });
                    });
                }
            });

        ui.separator();

        chart(ui, series, samples, &palette);
    }

    // The editor of a series, opened by the buttons above. It is drawn even
    // when the list is empty: that is where the first series is made.
    if let Some(draft) =
        crate::dialogs::series::series_dialog(ui.ctx(), state, connections, subscriptions)
    {
        let action = if draft.is_new {
            SeriesAction::Add(draft.series)
        } else {
            SeriesAction::Update(draft.series)
        };
        actions.series.push(action);
    }

    layout
}

/// Draw the chart of every series.
fn chart(ui: &mut Ui, series: &[DataSeries], samples: &Samples, palette: &Palette) {
    // The x axis counts the seconds from the oldest sample, so the numbers on
    // the axis stay readable.
    let oldest = samples
        .values()
        .filter_map(|points| points.front())
        .map(|point| point.at)
        .min()
        .unwrap_or(0);

    // The plot has no legend of its own: the list above names every series
    // and shows its colour.
    Plot::new("data_plot")
        .height(ui.available_height().max(160.0))
        .x_axis_label(i18n::t("秒"))
        .y_axis_label(i18n::t("数值"))
        // The grid is a hint, not the subject: it stays quiet so that the
        // curve is the loudest thing in the panel.
        .grid_color(palette.border)
        .grid_fade(0.7)
        .allow_zoom(true)
        .allow_drag(true)
        .show(ui, |plot_ui| {
            for (index, entry) in series.iter().enumerate() {
                let Some(points) = samples.get(&entry.id) else {
                    continue;
                };
                let plotted: Vec<[f64; 2]> = points
                    .iter()
                    .map(|point| [(point.at - oldest) as f64 / 1000.0, point.value])
                    .collect();
                if plotted.is_empty() {
                    continue;
                }
                plot_ui.line(
                    Line::new(entry.name.clone(), PlotPoints::from(plotted))
                        .color(series_color(palette, index))
                        .width(1.6),
                );
            }
        });
}

/// The colour of a series.
pub fn series_color(palette: &Palette, index: usize) -> Color32 {
    // The colours are far enough apart to tell the curves apart.
    let colors = [
        palette.icon,
        palette.success,
        palette.warning,
        palette.danger,
        palette.accent_text,
        palette.icon_alt,
    ];
    colors[index % colors.len()]
}

/// How the path of a series is shown.
fn path_label(path: &str) -> String {
    if path.trim().is_empty() {
        i18n::t("整个负载").to_string()
    } else {
        path.to_string()
    }
}
