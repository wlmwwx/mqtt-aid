//! The eframe application: it wires the UI, the engine and the storage together.

use eframe::egui;
use mqtt_core::models::{MqttSubscription, PublishRequest, SubscribeRequest};
use mqtt_engine::{MqttCommand, credentials};
use mqtt_ui::{CopyKind, UiActions, dialogs, panels};

use crate::app_state::AppState;
use mqtt_ui::state::AppCommand;

/// The MQTT Assistant window.
pub struct MqttApp {
    state: AppState,
    #[cfg(feature = "screenshot")]
    screenshot: crate::screenshot::Screenshot,
}

impl MqttApp {
    pub fn new() -> MqttApp {
        MqttApp {
            state: AppState::new(),
            #[cfg(feature = "screenshot")]
            screenshot: crate::screenshot::Screenshot::new(),
        }
    }
}

impl Default for MqttApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for MqttApp {
    /// Runs even when the window is hidden, which is what the screenshot
    /// helper relies on.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(not(feature = "screenshot"))]
        let _ = ctx;

        #[cfg(feature = "screenshot")]
        {
            // The picture has to be asked for from a painted frame: `logic`
            // runs even when the window has no surface, and a picture asked
            // for then is refused.
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaintWhileHidden);
            if self.screenshot.tick() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        #[cfg(feature = "screenshot")]
        {
            self.state.ui.open_menu = crate::screenshot::open_menu();
            self.screenshot.request(ui.ctx());
        }

        self.state.pump_events();

        let ctx = ui.ctx().clone();

        ctx.set_visuals(self.state.ui.theme.visuals());

        let mut actions = UiActions::default();
        let connections = self.state.connection_views();
        let subscriptions = self.state.visible_subscriptions();
        let messages = self.state.visible_messages();

        // The title bar and the ribbon sit above everything else.
        egui::Panel::top("app_bar")
            .exact_size(mqtt_ui::theme::spacing::APP_BAR_HEIGHT)
            .show(ui, |ui| {
                panels::app_bar::app_bar(ui, &mut self.state.ui, &connections, &mut actions);
            });

        egui::Panel::top("ribbon")
            .exact_size(
                mqtt_ui::theme::spacing::TAB_HEIGHT + mqtt_ui::theme::spacing::RIBBON_HEIGHT + 1.0,
            )
            .show(ui, |ui| {
                panels::ribbon::ribbon(ui, &mut self.state.ui, &mut actions);
            });

        if self.state.ui.show_status {
            egui::Panel::bottom("status_bar")
                .resizable(true)
                .show(ui, |ui| {
                    panels::status_bar::status_bar(
                        ui,
                        &mut self.state.ui,
                        &connections,
                        &mut actions,
                    );
                });
        }

        if self.state.ui.show_left {
            egui::Panel::left("left_panel")
                .default_size(self.state.ui.left_width)
                .resizable(true)
                .show(ui, |ui| {
                    panels::left_panel::left_panel(
                        ui,
                        &mut self.state.ui,
                        &connections,
                        &subscriptions,
                        &mut actions,
                    );
                });
        }

        if self.state.ui.show_right {
            egui::Panel::right("right_panel")
                .default_size(self.state.ui.right_width)
                .resizable(true)
                .show(ui, |ui| {
                    panels::publish_panel::publish_panel(
                        ui,
                        &mut self.state.ui,
                        &connections,
                        &mut actions,
                    );
                });
        }

        // The message monitor sits on the canvas colour, the way the SolidCraft
        // viewport sits on its own darker ground.
        egui::CentralPanel::default_margins()
            .frame(egui::Frame::new().fill(self.state.ui.theme.palette().canvas))
            .show(ui, |ui| {
                panels::message_panel::message_panel(
                    ui,
                    &mut self.state.ui,
                    &messages,
                    &connections,
                    &mut actions,
                );
            });

        if let Some(draft) = dialogs::connection_dialog(&ctx, &mut self.state.ui) {
            self.save_connection(draft);
        }
        if let Some(draft) = dialogs::subscribe_dialog(&mut self.state.ui, &ctx) {
            self.subscribe(draft);
        }
        if let Some(limit) =
            dialogs::settings::settings_window(&ctx, &mut self.state.ui, self.state.cache_limit)
        {
            actions.set_cache_limit = Some(limit);
        }
        dialogs::info::about_window(&ctx, &mut self.state.ui);
        dialogs::info::shortcuts_window(&ctx, &mut self.state.ui);

        self.apply(actions, &ctx);
    }

    fn on_exit(&mut self) {
        // Dropping the engine closes every connection and stops the runtime.
        self.state.engine.disconnect_all();
    }
}

impl MqttApp {
    /// Store a connection and remember its password in the keyring.
    fn save_connection(&mut self, draft: mqtt_ui::state::ConnectionDraft) {
        let mut config = draft.config;
        if !draft.password.is_empty() {
            config.credential_ref = Some(config.id.clone());
            if let Err(error) = credentials::store_password(&config.id, &draft.password) {
                tracing::warn!("could not store the password: {error}");
            }
        }
        self.state.save_connection(config);
    }

    /// Create a subscription and send it to the engine.
    fn subscribe(&mut self, draft: mqtt_ui::state::SubscribeDraft) {
        let topic = draft.topic.trim().to_string();
        if let Err(error) = mqtt_core::topic::validate_filter(&topic) {
            tracing::warn!("invalid topic filter: {error}");
            return;
        }

        let id = panels::left_panel::new_id();
        let subscription = MqttSubscription::new(
            id.clone(),
            draft.connection_id.clone(),
            topic.clone(),
            draft.qos,
        );
        self.state.save_subscription(subscription);

        let _ = self
            .state
            .engine
            .execute(MqttCommand::Subscribe(SubscribeRequest {
                connection_id: draft.connection_id,
                id,
                topic,
                qos: draft.qos,
            }));
    }

    /// Turn the intentions collected by the UI into engine commands.
    fn apply(&mut self, actions: UiActions, ctx: &egui::Context) {
        for id in &actions.connect {
            if let Some(connection) = self
                .state
                .connections
                .iter()
                .find(|connection| &connection.id == id)
                .cloned()
            {
                let _ = self.state.engine.execute(MqttCommand::Connect(connection));
            }
        }

        for id in &actions.disconnect {
            let _ = self
                .state
                .engine
                .execute(MqttCommand::Disconnect(id.clone()));
        }

        for id in &actions.delete_connection {
            let _ = self
                .state
                .engine
                .execute(MqttCommand::Disconnect(id.clone()));
            self.state.delete_connection(id);
        }

        for (connection_id, topic) in &actions.unsubscribe {
            let _ = self.state.engine.execute(MqttCommand::Unsubscribe {
                connection_id: connection_id.clone(),
                topic: topic.clone(),
            });
            if let Some(subscription) = self.state.subscriptions.iter().find(|subscription| {
                &subscription.topic == topic && &subscription.connection_id == connection_id
            }) {
                let id = subscription.id.clone();
                self.state.delete_subscription(&id);
            }
        }

        for (connection_id, topic, qos) in &actions.set_qos {
            let _ = self.state.engine.execute(MqttCommand::SetSubscriptionQos {
                connection_id: connection_id.clone(),
                topic: topic.clone(),
                qos: *qos,
            });
            if let Some(subscription) = self.state.subscriptions.iter_mut().find(|subscription| {
                &subscription.topic == topic && &subscription.connection_id == connection_id
            }) {
                subscription.qos = *qos;
                let updated = subscription.clone();
                self.state.save_subscription(updated);
            }
        }

        for (id, format) in &actions.set_subscription_format {
            if let Some(subscription) = self
                .state
                .subscriptions
                .iter_mut()
                .find(|subscription| &subscription.id == id)
            {
                subscription.format = *format;
                let updated = subscription.clone();
                self.state.save_subscription(updated);
            }
        }

        for id in &actions.toggle_subscription_pause {
            if let Some(subscription) = self
                .state
                .subscriptions
                .iter_mut()
                .find(|subscription| &subscription.id == id)
            {
                subscription.paused = !subscription.paused;
                let updated = subscription.clone();
                self.state.save_subscription(updated);
            }
        }

        for request in &actions.publish {
            let _ = self
                .state
                .engine
                .execute(MqttCommand::Publish(request.clone()));
        }

        for template in &actions.save_template {
            self.state.save_template(template.clone());
        }

        for id in &actions.load_template {
            self.state.load_template(id);
        }

        for id in &actions.delete_template {
            self.state.delete_template(id);
        }

        for request in &actions.copy {
            let text = match request.kind {
                CopyKind::Topic | CopyKind::Payload | CopyKind::Hex => request.value.clone(),
            };
            self.state.copy_to_clipboard(ctx, &text);
        }

        if let Some(format) = actions.export
            && let Err(error) = self.state.export(format)
        {
            tracing::warn!("export failed: {error}");
        }

        if actions.clear_messages {
            self.state.messages.clear();
            self.state.ui.selected_message = None;
        }

        if actions.toggle_pause {
            self.state.paused = !self.state.paused;
            self.state.ui.paused = self.state.paused;
        }

        if let Some(message) = actions.send_to_publisher {
            self.state.ui.publish.topic = message.topic.clone();
            self.state.ui.publish.payload = String::from_utf8_lossy(&message.payload).to_string();
            self.state.ui.publish.qos = message.qos;
            self.state.ui.publish.retain = message.retain;
            self.state.ui.publish.connection_id = Some(message.connection_id);
        }

        if let Some(limit) = actions.set_cache_limit {
            self.state.cache_limit = limit.clamp(100, 100_000);
            self.state.persist_cache_limit();
        }

        if actions.toggle_theme {
            self.state.ui.theme = self.state.ui.theme.toggled();
        }

        for command in actions.commands {
            self.apply_command(command, ctx);
        }
    }

    /// Act on a command coming from the menu bar or the ribbon.
    ///
    /// A command that carries an id acts on that id; an empty id means "the
    /// current selection".
    fn apply_command(&mut self, command: AppCommand, ctx: &egui::Context) {
        match command {
            AppCommand::NewConnection => {
                let id = panels::left_panel::new_id();
                self.state.ui.connection_draft = Some(mqtt_ui::state::ConnectionDraft {
                    config: mqtt_core::models::MqttConnection::new(id, "新连接", "127.0.0.1", 1883),
                    password: String::new(),
                    is_new: true,
                });
            }
            AppCommand::EditConnection(id) => {
                let id = self.resolve_connection(&id);
                let Some(config) = self
                    .state
                    .connections
                    .iter()
                    .find(|connection| connection.id == id)
                    .cloned()
                else {
                    return;
                };
                self.state.ui.connection_draft = Some(mqtt_ui::state::ConnectionDraft {
                    config,
                    password: String::new(),
                    is_new: false,
                });
            }
            AppCommand::DeleteConnection(id) => {
                let id = self.resolve_connection(&id);
                let _ = self
                    .state
                    .engine
                    .execute(MqttCommand::Disconnect(id.clone()));
                self.state.delete_connection(&id);
            }
            AppCommand::Connect(id) => {
                let id = self.resolve_connection(&id);
                if let Some(connection) = self
                    .state
                    .connections
                    .iter()
                    .find(|connection| connection.id == id)
                    .cloned()
                {
                    let _ = self.state.engine.execute(MqttCommand::Connect(connection));
                }
            }
            AppCommand::Disconnect(id) => {
                let id = self.resolve_connection(&id);
                let _ = self.state.engine.execute(MqttCommand::Disconnect(id));
            }
            AppCommand::NewSubscription => {
                let Some(connection_id) = self.state.ui.selected_connection.clone() else {
                    tracing::warn!("select a connection before subscribing");
                    return;
                };
                self.state.ui.subscribe_draft = Some(mqtt_ui::state::SubscribeDraft {
                    connection_id,
                    topic: String::new(),
                    qos: 0,
                });
            }
            AppCommand::Unsubscribe(id) => {
                let Some(subscription) = self.resolve_subscription(&id) else {
                    return;
                };
                let _ = self.state.engine.execute(MqttCommand::Unsubscribe {
                    connection_id: subscription.connection_id.clone(),
                    topic: subscription.topic.clone(),
                });
                self.state.delete_subscription(&subscription.id);
            }
            AppCommand::ToggleSubscriptionPause(id) => {
                let Some(subscription) = self.resolve_subscription(&id) else {
                    return;
                };
                let mut updated = subscription;
                updated.paused = !updated.paused;
                self.state.save_subscription(updated);
            }
            AppCommand::ClearMessages => {
                self.state.messages.clear();
                self.state.ui.selected_message = None;
            }
            AppCommand::TogglePause => {
                self.state.paused = !self.state.paused;
                self.state.ui.paused = self.state.paused;
            }
            AppCommand::Export(format) => {
                if let Err(error) = self.state.export(format) {
                    tracing::warn!("export failed: {error}");
                }
            }
            AppCommand::SetFormat(format) => {
                self.state.ui.message_format = format;
            }
            AppCommand::FormatJson => self.state.format_publish_json(true),
            AppCommand::MinifyJson => self.state.format_publish_json(false),
            AppCommand::PublishNow => {
                if let Some(request) = self.state.publish_request() {
                    let _ = self.state.engine.execute(MqttCommand::Publish(request));
                }
            }
            AppCommand::SaveTemplate => {
                if let Some(template) = self.state.publish_template() {
                    self.state.save_template(template);
                }
            }
            AppCommand::CopyTopic => {
                if let Some(message) = self.copy_target() {
                    self.state.copy_to_clipboard(ctx, &message.topic);
                }
            }
            AppCommand::CopyPayload => {
                if let Some(message) = self.copy_target() {
                    self.state
                        .copy_to_clipboard(ctx, &String::from_utf8_lossy(&message.payload));
                }
            }
            AppCommand::CopyHex => {
                if let Some(message) = self.copy_target() {
                    self.state
                        .copy_to_clipboard(ctx, &mqtt_core::format::encode_hex(&message.payload));
                }
            }
            AppCommand::SendToPublisher => {
                if let Some(message) = self.copy_target() {
                    self.state.ui.publish.topic = message.topic.clone();
                    self.state.ui.publish.payload =
                        String::from_utf8_lossy(&message.payload).to_string();
                    self.state.ui.publish.qos = message.qos;
                    self.state.ui.publish.retain = message.retain;
                    self.state.ui.publish.connection_id = Some(message.connection_id);
                }
            }
            AppCommand::ToggleTheme => {
                self.state.ui.theme = self.state.ui.theme.toggled();
            }
            AppCommand::ToggleLeftPanel => {
                self.state.ui.show_left = !self.state.ui.show_left;
            }
            AppCommand::ToggleRightPanel => {
                self.state.ui.show_right = !self.state.ui.show_right;
            }
            AppCommand::ToggleStatusBar => {
                self.state.ui.show_status = !self.state.ui.show_status;
            }
            AppCommand::FocusSearch => {
                self.state.ui.focus_search = true;
            }
            AppCommand::About => {
                self.state.ui.show_about = true;
            }
            AppCommand::Shortcuts => {
                self.state.ui.show_shortcuts = true;
            }
            AppCommand::Quit => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    /// The connection a command acts on: the given id, or the selected one.
    fn resolve_connection(&self, id: &str) -> String {
        if !id.is_empty() {
            return id.to_string();
        }
        self.state
            .ui
            .selected_connection
            .clone()
            .unwrap_or_default()
    }

    /// The subscription a command acts on: the given id, or the selected one.
    fn resolve_subscription(&self, id: &str) -> Option<mqtt_core::models::MqttSubscription> {
        let wanted = if id.is_empty() {
            self.state.ui.selected_subscription.clone()?
        } else {
            id.to_string()
        };
        self.state
            .subscriptions
            .iter()
            .find(|subscription| subscription.id == wanted)
            .cloned()
    }

    /// The message the copy commands act on.
    fn copy_target(&self) -> Option<mqtt_core::models::MqttMessage> {
        if let Some(message) = &self.state.ui.copy_target {
            return Some(message.clone());
        }
        let index = self.state.ui.selected_message?;
        self.state.visible_messages().into_iter().nth(index)
    }
}

/// The publish request type, re-exported for the tests.
pub type AppPublishRequest = PublishRequest;
