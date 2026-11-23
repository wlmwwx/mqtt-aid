//! Headless tests of the browser panel.
//!
//! The panel is a tree: every connection carries its own subscriptions, so a
//! subscription always shows up under the connection it belongs to.

use egui::{Context, Event, PointerButton, Pos2, RawInput};
use mqtt_core::models::{ConnectionStatus, MqttConnection, MqttSubscription, MqttVersion};
use mqtt_ui::panels::left_panel::{LeftPanelLayout, left_panel};
use mqtt_ui::state::{AppCommand, ConnectionView, UiActions, UiState};

/// A connection as the panel sees it.
fn connection(id: &str, name: &str) -> ConnectionView {
    let mut config = MqttConnection::new(id, name, "127.0.0.1", 1883);
    config.protocol = MqttVersion::V311;
    ConnectionView {
        config,
        status: ConnectionStatus::Connected,
    }
}

/// A subscription of a connection.
fn subscription(id: &str, connection_id: &str, topic: &str) -> MqttSubscription {
    MqttSubscription::new(id, connection_id, topic, 1)
}

/// Draw the panel and return the texts it shows with where they are.
fn draw(
    ctx: &Context,
    state: &mut UiState,
    connections: &[ConnectionView],
    subscriptions: &[MqttSubscription],
) -> (LeftPanelLayout, UiActions, Vec<(String, Pos2)>) {
    draw_with(ctx, state, connections, subscriptions, RawInput::default())
}

/// Draw the panel with the given input.
fn draw_with(
    ctx: &Context,
    state: &mut UiState,
    connections: &[ConnectionView],
    subscriptions: &[MqttSubscription],
    input: RawInput,
) -> (LeftPanelLayout, UiActions, Vec<(String, Pos2)>) {
    let mut actions = UiActions::default();
    let mut layout = LeftPanelLayout::default();

    let output = ctx.run_ui(input, |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            layout = left_panel(ui, state, connections, subscriptions, &mut actions);
        });
    });

    let mut texts = Vec::new();
    for shape in &output.shapes {
        if let egui::Shape::Text(text_shape) = &shape.shape {
            texts.push((text_shape.galley.job.text.to_string(), text_shape.pos));
        }
    }
    output.drop_without_applying_deltas();

    (layout, actions, texts)
}

/// The place of a text in the panel.
fn place(texts: &[(String, Pos2)], needle: &str) -> Pos2 {
    texts
        .iter()
        .find(|(text, _)| text == needle)
        .map(|(_, place)| *place)
        .unwrap_or_else(|| panic!("{needle} is not shown"))
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

/// A subscription is shown under the connection it belongs to.
#[test]
fn a_subscription_lives_under_its_connection() {
    let ctx = Context::default();
    let mut state = UiState::default();
    let connections = vec![connection("c1", "Local"), connection("c2", "Public")];
    let subscriptions = vec![
        subscription("s1", "c1", "app/#"),
        subscription("s2", "c2", "sensors/room"),
    ];

    let (_, _, texts) = draw(&ctx, &mut state, &connections, &subscriptions);

    let local = place(&texts, "Local");
    let app = place(&texts, "app/#");
    let public = place(&texts, "Public");
    let sensors = place(&texts, "sensors/room");

    assert!(
        app.y > local.y,
        "the subscription of the first connection is not under it"
    );
    assert!(
        sensors.y > public.y,
        "the subscription of the second connection is not under it"
    );
    assert!(
        app.x > local.x && sensors.x > public.x,
        "the subscriptions are not indented"
    );
    // The subscription of one connection is not shown under the other one.
    assert!(
        app.y < public.y,
        "the subscription of the first connection is shown under the second one"
    );
}

/// The button on a connection adds a subscription to that connection.
#[test]
fn the_button_of_a_connection_subscribes_on_that_connection() {
    let ctx = Context::default();
    let mut state = UiState::default();
    let connections = vec![connection("c1", "Local"), connection("c2", "Public")];

    let (layout, _, _) = draw(&ctx, &mut state, &connections, &[]);
    let (_, rect) = layout
        .subscribe_buttons
        .iter()
        .find(|(id, _)| id == "c2")
        .expect("the second connection has a subscribe button");

    draw_with(&ctx, &mut state, &connections, &[], press_at(rect.center()));
    let (_, actions, _) = draw_with(
        &ctx,
        &mut state,
        &connections,
        &[],
        release_at(rect.center()),
    );

    assert_eq!(
        actions.commands,
        vec![AppCommand::NewSubscription("c2".to_string())],
        "the subscription was not made on the second connection"
    );
}
