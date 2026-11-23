//! Headless tests of the data panel.
//!
//! The data panel lists the data series and draws them as charts. These tests
//! click the panel and check that the panel reports what the user asked for.

use std::collections::{HashMap, VecDeque};

use egui::{Context, Event, PointerButton, Pos2, RawInput, Rect};
use mqtt_core::models::{DataPoint, DataSeries};
use mqtt_ui::panels::data_panel::{DataPanelLayout, Samples, data_panel};
use mqtt_ui::state::{SeriesAction, UiActions, UiState};

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

/// Draw the panel and report where its buttons landed.
fn draw(
    ctx: &Context,
    state: &mut UiState,
    series: &[DataSeries],
    samples: &Samples,
) -> (DataPanelLayout, UiActions) {
    draw_with(ctx, state, series, samples, RawInput::default())
}

/// Draw the panel with the given input.
///
/// The button has to be drawn in the frame the mouse is pressed in, so the
/// click is registered.
fn draw_with(
    ctx: &Context,
    state: &mut UiState,
    series: &[DataSeries],
    samples: &Samples,
    input: RawInput,
) -> (DataPanelLayout, UiActions) {
    let mut actions = UiActions::default();
    let mut layout = DataPanelLayout {
        add_button: Rect::ZERO,
        remove_buttons: Vec::new(),
    };

    let output = ctx.run_ui(input, |ui| {
        layout = data_panel(ui, state, series, samples, &[], &[], &mut actions);
    });
    output.drop_without_applying_deltas();

    (layout, actions)
}

/// Draw the panel and report the text it shows.
fn texts(
    ctx: &Context,
    state: &mut UiState,
    series: &[DataSeries],
    samples: &Samples,
    input: RawInput,
) -> String {
    let mut actions = UiActions::default();
    let output = ctx.run_ui(input, |ui| {
        data_panel(ui, state, series, samples, &[], &[], &mut actions);
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

/// A series and the samples it collected.
fn series_with_samples() -> (DataSeries, Samples) {
    let series = DataSeries::new("s1", "温度", "c1", "sensors/room", "data.temperature");
    let mut samples: Samples = HashMap::new();
    samples.insert(
        series.id.clone(),
        VecDeque::from([
            DataPoint {
                at: 1_000,
                value: 20.0,
            },
            DataPoint {
                at: 2_000,
                value: 21.5,
            },
        ]),
    );
    (series, samples)
}

#[test]
fn the_panel_lists_the_series() {
    let ctx = Context::default();
    let (series, samples) = series_with_samples();
    let mut state = UiState::default();

    let mut text = String::new();
    let output = ctx.run_ui(RawInput::default(), |ui| {
        let mut actions = UiActions::default();
        data_panel(
            ui,
            &mut state,
            std::slice::from_ref(&series),
            &samples,
            &[],
            &[],
            &mut actions,
        );
    });
    for shape in &output.shapes {
        if let egui::Shape::Text(text_shape) = &shape.shape {
            text.push_str(&text_shape.galley.job.text);
            text.push('\n');
        }
    }
    output.drop_without_applying_deltas();

    assert!(text.contains("温度"), "the series is missing:\n{text}");
    assert!(
        text.contains("sensors/room"),
        "the topic is missing:\n{text}"
    );
    assert!(
        text.contains("data.temperature"),
        "the field path is missing:\n{text}"
    );
    assert!(
        text.contains("2 点"),
        "the sample count is missing:\n{text}"
    );
}

#[test]
fn the_add_button_opens_the_editor() {
    let ctx = Context::default();
    let mut state = UiState::default();

    let (layout, _) = draw(&ctx, &mut state, &[], &HashMap::new());
    let button = layout.add_button.center();

    draw_with(&ctx, &mut state, &[], &HashMap::new(), press_at(button));
    draw_with(&ctx, &mut state, &[], &HashMap::new(), release_at(button));

    assert!(state.series_draft.is_some(), "the editor did not open");
}

/// The editor has to open even when there is no series yet: that is where
/// the first one is made.
///
/// A floating window is measured in the frame it opens and is drawn in the
/// frame after that, so the text is looked for in a later frame.
#[test]
fn the_editor_opens_without_any_series() {
    let ctx = Context::default();
    let mut state = UiState::default();

    let (layout, _) = draw(&ctx, &mut state, &[], &HashMap::new());
    let button = layout.add_button.center();

    draw_with(&ctx, &mut state, &[], &HashMap::new(), press_at(button));
    draw_with(&ctx, &mut state, &[], &HashMap::new(), release_at(button));
    let text = texts(&ctx, &mut state, &[], &HashMap::new(), RawInput::default());

    assert!(
        text.contains("Field path"),
        "the editor was not drawn:\n{text}"
    );
    assert!(
        text.contains("New data series"),
        "the editor has no title:\n{text}"
    );
}

#[test]
fn the_remove_button_deletes_the_series() {
    let ctx = Context::default();
    let (series, samples) = series_with_samples();
    let mut state = UiState::default();

    let (layout, _) = draw(&ctx, &mut state, std::slice::from_ref(&series), &samples);
    let (_, rect) = layout
        .remove_buttons
        .iter()
        .find(|(id, _)| id == &series.id)
        .expect("the delete button was drawn");

    let (_, _) = draw_with(
        &ctx,
        &mut state,
        std::slice::from_ref(&series),
        &samples,
        press_at(rect.center()),
    );
    let (_, actions) = draw_with(
        &ctx,
        &mut state,
        std::slice::from_ref(&series),
        &samples,
        release_at(rect.center()),
    );

    assert_eq!(
        actions.series,
        vec![SeriesAction::Remove(series.id.clone())],
        "the series was not deleted"
    );
}
