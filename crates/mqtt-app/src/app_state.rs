//! The application state: everything the window draws and the engine reports.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::time::Instant;

use mqtt_core::models::{
    ConnectionStatus, DataPoint, DataSeries, MqttConnection, MqttMessage, MqttSubscription,
    PayloadFormat, PublishRequest, PublishTemplate, SubscribeRequest,
};
use mqtt_engine::{MqttCommand, MqttEngine, MqttEvent};
use mqtt_storage::{
    ConnectionRepo, DataSeriesRepo, Database, SettingsRepo, SubscriptionRepo, TemplateRepo,
};
use mqtt_ui::{ConnectionView, ExportFormat, UiState};

/// How many messages are kept in memory.
pub const DEFAULT_CACHE_LIMIT: usize = 10_000;
/// How many events are drained from the engine per frame.
pub const EVENTS_PER_FRAME: usize = 512;
/// How many samples a data series keeps.
pub const SAMPLE_LIMIT: usize = 2_000;

/// The application state.
pub struct AppState {
    pub engine: MqttEngine,
    pub storage: Option<Database>,
    pub connections: Vec<MqttConnection>,
    pub subscriptions: Vec<MqttSubscription>,
    pub statuses: HashMap<String, ConnectionStatus>,
    pub messages: VecDeque<MqttMessage>,
    pub templates: Vec<PublishTemplate>,
    /// The data series the user watches.
    pub series: Vec<DataSeries>,
    /// The samples of every series, by series id.
    pub samples: HashMap<String, VecDeque<DataPoint>>,
    pub ui: UiState,
    pub paused: bool,
    pub received: u64,
    pub published: u64,
    pub cache_limit: usize,
    rate_window: Instant,
    rate_count: u64,
    pub rate: f64,
}

impl Default for AppState {
    fn default() -> Self {
        AppState::new()
    }
}

impl AppState {
    /// Build the state, loading whatever was persisted before.
    pub fn new() -> AppState {
        AppState::open_in(config_dir())
    }

    /// Build the state with the database in a specific directory.
    pub fn open_in(directory: PathBuf) -> AppState {
        let storage = open_storage_in(&directory);
        let mut state = AppState {
            engine: MqttEngine::new(),
            storage,
            connections: Vec::new(),
            subscriptions: Vec::new(),
            statuses: HashMap::new(),
            messages: VecDeque::new(),
            templates: Vec::new(),
            series: Vec::new(),
            samples: HashMap::new(),
            ui: UiState::default(),
            paused: false,
            received: 0,
            published: 0,
            cache_limit: DEFAULT_CACHE_LIMIT,
            rate_window: Instant::now(),
            rate_count: 0,
            rate: 0.0,
        };
        state.load();
        state
    }

    /// Load the persisted configuration.
    pub fn load(&mut self) {
        let Some(storage) = &self.storage else {
            return;
        };

        match ConnectionRepo::new(storage).list() {
            Ok(connections) => self.connections = connections,
            Err(error) => tracing::warn!("could not load the connections: {error}"),
        }
        match SubscriptionRepo::new(storage).list() {
            Ok(subscriptions) => self.subscriptions = subscriptions,
            Err(error) => tracing::warn!("could not load the subscriptions: {error}"),
        }
        match TemplateRepo::new(storage).list() {
            Ok(templates) => self.templates = templates,
            Err(error) => tracing::warn!("could not load the templates: {error}"),
        }
        match DataSeriesRepo::new(storage).list() {
            Ok(series) => self.series = series,
            Err(error) => tracing::warn!("could not load the data series: {error}"),
        }
        if let Ok(Some(code)) = SettingsRepo::new(storage).get("language")
            && let Some(language) = mqtt_ui::LanguageChoice::from_code(&code)
        {
            self.ui.language = language;
        }
        if let Ok(Some(limit)) = SettingsRepo::new(storage).get("cache_limit")
            && let Ok(limit) = limit.parse::<usize>()
        {
            self.cache_limit = limit;
        }

        self.ui.publish.templates = self.templates.clone();
        if self.ui.selected_connection.is_none() {
            self.ui.selected_connection = self.connections.first().map(|c| c.id.clone());
        }
        if self.ui.publish.connection_id.is_none() {
            self.ui.publish.connection_id = self.ui.selected_connection.clone();
        }
    }

    /// Store the cache limit in the settings table.
    pub fn persist_cache_limit(&self) {
        if let Some(storage) = &self.storage
            && let Err(error) =
                SettingsRepo::new(storage).set("cache_limit", &self.cache_limit.to_string())
        {
            tracing::warn!("could not save the cache limit: {error}");
        }
    }

    /// Store the language of the interface.
    ///
    /// The language is a property of the application, not of the window: it
    /// is remembered, so that the interface comes back in the language the
    /// user reads.
    pub fn persist_language(&self) {
        if let Some(storage) = &self.storage
            && let Err(error) = SettingsRepo::new(storage).set("language", self.ui.language.code())
        {
            tracing::warn!("could not save the language: {error}");
        }
    }

    /// The connections as the UI sees them.
    pub fn connection_views(&self) -> Vec<ConnectionView> {
        self.connections
            .iter()
            .map(|config| ConnectionView {
                status: self
                    .statuses
                    .get(&config.id)
                    .copied()
                    .unwrap_or(ConnectionStatus::Disconnected),
                config: config.clone(),
            })
            .collect()
    }

    /// The messages that match the current search and filters.
    pub fn visible_messages(&self) -> Vec<MqttMessage> {
        let search = self.ui.search.trim().to_lowercase();
        self.messages
            .iter()
            .filter(|message| self.matches_filter(message, &search))
            .cloned()
            .collect()
    }

    fn matches_filter(&self, message: &MqttMessage, search: &str) -> bool {
        if let Some(qos) = self.ui.filter_qos
            && message.qos != qos
        {
            return false;
        }
        if let Some(retain) = self.ui.filter_retain
            && message.retain != retain
        {
            return false;
        }
        if search.is_empty() {
            return true;
        }
        message.topic.to_lowercase().contains(search)
            || String::from_utf8_lossy(&message.payload)
                .to_lowercase()
                .contains(search)
    }

    /// Drain the engine events into the state.
    pub fn pump_events(&mut self) {
        for event in self.engine.poll_events(EVENTS_PER_FRAME) {
            match event {
                MqttEvent::ConnectionStatusChanged {
                    connection_id,
                    status,
                } => {
                    let connected = status == ConnectionStatus::Connected;
                    self.statuses.insert(connection_id.clone(), status);
                    if connected {
                        // The broker forgets the subscriptions when the
                        // session ends, so a connection that comes back has
                        // to ask for them again.
                        self.restore_subscriptions(&connection_id);
                    }
                }
                MqttEvent::MessageReceived(message) => {
                    // The data series are read even while the display is
                    // paused: pausing hides the messages, it does not stop
                    // the charts.
                    self.collect(&message);
                    if !self.paused {
                        self.received += 1;
                        self.rate_count += 1;
                        self.messages.push_back(message);
                        while self.messages.len() > self.cache_limit {
                            self.messages.pop_front();
                        }
                    }
                }
                MqttEvent::PublishCompleted { .. } => {
                    self.published += 1;
                }
                MqttEvent::SubscriptionAcknowledged { .. } => {}
                MqttEvent::Error {
                    connection_id,
                    message,
                } => {
                    tracing::warn!("engine error: {message}");
                    if let Some(id) = connection_id {
                        self.statuses.insert(id, ConnectionStatus::Error);
                    }
                }
            }
        }

        let elapsed = self.rate_window.elapsed().as_secs_f64();
        if elapsed >= 1.0 {
            self.rate = self.rate_count as f64 / elapsed;
            self.rate_count = 0;
            self.rate_window = Instant::now();
        }

        self.ui.stats.received = self.received;
        self.ui.stats.published = self.published;
        self.ui.stats.messages_per_second = self.rate;
        self.ui.stats.cached = self.messages.len();
    }

    /// Subscribe again to everything a connection is subscribed to.
    ///
    /// A broker drops the subscriptions of a session when the session ends,
    /// so a connection that was disconnected and connected again — or that was
    /// opened after the application was restarted — receives nothing until it
    /// subscribes again. The subscriptions of the application are the truth
    /// here: they are persisted, and a subscription the user deleted is
    /// really gone.
    fn restore_subscriptions(&mut self, connection_id: &str) {
        let requests: Vec<SubscribeRequest> = self
            .subscriptions
            .iter()
            .filter(|subscription| subscription.connection_id == connection_id)
            .map(|subscription| SubscribeRequest {
                connection_id: connection_id.to_string(),
                id: subscription.id.clone(),
                topic: subscription.topic.clone(),
                qos: subscription.qos,
            })
            .collect();

        for request in requests {
            if let Err(error) = self.engine.execute(MqttCommand::Subscribe(request)) {
                tracing::warn!("could not subscribe again: {error}");
            }
        }
    }

    /// Read the value of every series out of a message.
    ///
    /// A series only reads the messages of the connection and the topics it
    /// watches, and only the ones that hold the value it is after.
    fn collect(&mut self, message: &MqttMessage) {
        let watching: Vec<(String, String)> = self
            .series
            .iter()
            .filter(|series| {
                series.connection_id == message.connection_id
                    && mqtt_core::topic::matches(&series.topic, &message.topic)
            })
            .map(|series| (series.id.clone(), series.path.clone()))
            .collect();

        for (id, path) in watching {
            let Some(value) = mqtt_core::extract::extract_value(&message.payload, &path) else {
                continue;
            };
            let points = self.samples.entry(id).or_default();
            points.push_back(DataPoint {
                at: message.received_at,
                value,
            });
            while points.len() > SAMPLE_LIMIT {
                points.pop_front();
            }
        }
    }

    /// Watch a new data series.
    ///
    /// The messages of the topic have to reach the application, so the
    /// topic is subscribed when no subscription covers it yet.
    pub fn add_series(&mut self, series: DataSeries) {
        match self.series.iter_mut().find(|entry| entry.id == series.id) {
            Some(existing) => *existing = series.clone(),
            None => self.series.push(series.clone()),
        }
        self.persist_series(&series);
        self.subscribe_topic(&series);
    }

    /// Make sure the messages of the topic of a series arrive.
    ///
    /// A subscription that already covers the topic is left alone: two
    /// subscriptions of the same topic would deliver every message twice.
    fn subscribe_topic(&mut self, series: &DataSeries) {
        let topic = series.topic.trim();
        if topic.is_empty() || series.connection_id.is_empty() {
            return;
        }

        let covered = self.subscriptions.iter().any(|subscription| {
            subscription.connection_id == series.connection_id
                && mqtt_core::topic::matches(&subscription.topic, topic)
        });
        if covered {
            return;
        }

        let id = mqtt_ui::panels::left_panel::new_id();
        let subscription = MqttSubscription::new(
            id.clone(),
            series.connection_id.clone(),
            topic.to_string(),
            0,
        );
        self.save_subscription(subscription);

        let _ = self
            .engine
            .execute(MqttCommand::Subscribe(SubscribeRequest {
                connection_id: series.connection_id.clone(),
                id,
                topic: topic.to_string(),
                qos: 0,
            }));
    }

    /// Change a data series.
    pub fn update_series(&mut self, series: DataSeries) {
        self.add_series(series);
    }

    /// Stop watching a data series.
    pub fn remove_series(&mut self, id: &str) {
        self.series.retain(|series| series.id != id);
        self.samples.remove(id);
        if let Some(storage) = &self.storage
            && let Err(error) = DataSeriesRepo::new(storage).delete(id)
        {
            tracing::warn!("could not delete the data series: {error}");
        }
    }

    /// Store a data series.
    fn persist_series(&self, series: &DataSeries) {
        if let Some(storage) = &self.storage
            && let Err(error) = DataSeriesRepo::new(storage).save(series)
        {
            tracing::warn!("could not save the data series: {error}");
        }
    }

    /// Persist a connection.
    pub fn save_connection(&mut self, connection: MqttConnection) {
        match self.connections.iter_mut().find(|c| c.id == connection.id) {
            Some(existing) => *existing = connection.clone(),
            None => self.connections.push(connection.clone()),
        }
        if let Some(storage) = &self.storage
            && let Err(error) = ConnectionRepo::new(storage).save(&connection)
        {
            tracing::warn!("could not save the connection: {error}");
        }
        if self.ui.selected_connection.is_none() {
            self.ui.selected_connection = Some(connection.id.clone());
        }
    }

    /// Persist a subscription.
    pub fn save_subscription(&mut self, subscription: MqttSubscription) {
        match self
            .subscriptions
            .iter_mut()
            .find(|s| s.id == subscription.id)
        {
            Some(existing) => *existing = subscription.clone(),
            None => self.subscriptions.push(subscription.clone()),
        }
        if let Some(storage) = &self.storage
            && let Err(error) = SubscriptionRepo::new(storage).save(&subscription)
        {
            tracing::warn!("could not save the subscription: {error}");
        }
    }

    /// Remove a subscription.
    pub fn delete_subscription(&mut self, id: &str) {
        self.subscriptions
            .retain(|subscription| subscription.id != id);
        if let Some(storage) = &self.storage
            && let Err(error) = SubscriptionRepo::new(storage).delete(id)
        {
            tracing::warn!("could not delete the subscription: {error}");
        }
    }

    /// Remove a connection and everything that belongs to it.
    pub fn delete_connection(&mut self, id: &str) {
        self.connections.retain(|connection| connection.id != id);
        self.subscriptions
            .retain(|subscription| subscription.connection_id != id);
        // The data series of the connection go with it.
        let gone: Vec<String> = self
            .series
            .iter()
            .filter(|series| series.connection_id == id)
            .map(|series| series.id.clone())
            .collect();
        for series_id in gone {
            self.remove_series(&series_id);
        }
        self.statuses.remove(id);
        if let Some(storage) = &self.storage {
            let repo = ConnectionRepo::new(storage);
            if let Err(error) = repo.delete(id) {
                tracing::warn!("could not delete the connection: {error}");
            }
        }
        if self.ui.selected_connection.as_deref() == Some(id) {
            self.ui.selected_connection = self.connections.first().map(|c| c.id.clone());
        }
    }

    /// Persist a publish template.
    pub fn save_template(&mut self, template: PublishTemplate) {
        match self.templates.iter_mut().find(|t| t.id == template.id) {
            Some(existing) => *existing = template.clone(),
            None => self.templates.push(template.clone()),
        }
        self.ui.publish.templates = self.templates.clone();
        if let Some(storage) = &self.storage
            && let Err(error) = TemplateRepo::new(storage).save(&template)
        {
            tracing::warn!("could not save the template: {error}");
        }
    }

    /// Remove a publish template.
    pub fn delete_template(&mut self, id: &str) {
        self.templates.retain(|template| template.id != id);
        self.ui.publish.templates = self.templates.clone();
        if let Some(storage) = &self.storage
            && let Err(error) = TemplateRepo::new(storage).delete(id)
        {
            tracing::warn!("could not delete the template: {error}");
        }
    }

    /// Load a template into the publish form.
    pub fn load_template(&mut self, id: &str) {
        let Some(template) = self.templates.iter().find(|t| t.id == id).cloned() else {
            return;
        };
        let text = match template.format {
            PayloadFormat::Hex => mqtt_core::format::encode_hex(&template.payload),
            PayloadFormat::Base64 => mqtt_core::format::codec_for(PayloadFormat::Base64)
                .decode(&template.payload)
                .unwrap_or_default(),
            PayloadFormat::Binary => mqtt_core::format::codec_for(PayloadFormat::Binary)
                .decode(&template.payload)
                .unwrap_or_default(),
            _ => String::from_utf8_lossy(&template.payload).to_string(),
        };
        self.ui.publish.topic = template.topic.clone();
        self.ui.publish.payload = text;
        self.ui.publish.format = template.format;
        self.ui.publish.qos = template.qos;
        self.ui.publish.retain = template.retain;
    }

    /// Copy something to the clipboard.
    pub fn copy_to_clipboard(&self, ctx: &egui::Context, text: &str) {
        ctx.copy_text(text.to_string());
    }

    /// Reformat the JSON payload of the publish form.
    ///
    /// `pretty` formats the payload with the configured indent, otherwise it
    /// is compacted onto a single line. An invalid document leaves the payload
    /// alone and shows the error.
    pub fn format_publish_json(&mut self, pretty: bool) {
        let indent = self.ui.publish.json_indent;
        let payload = self.ui.publish.payload.clone();
        let result = if pretty {
            mqtt_core::format::format_json(&payload, indent)
        } else {
            mqtt_core::format::minify_json(&payload)
        };
        match result {
            Ok(text) => {
                self.ui.publish.payload = text;
                self.ui.publish.validation_error = None;
            }
            Err(error) => {
                self.ui.publish.validation_error = Some(error.to_string());
            }
        }
    }

    /// The publish request the publish form currently describes.
    ///
    /// `None` when the form is not ready to be sent: no connection, no topic
    /// or a payload that does not encode.
    pub fn publish_request(&self) -> Option<PublishRequest> {
        let publish = &self.ui.publish;
        let connection_id = publish.connection_id.clone()?;
        if publish.topic.trim().is_empty() {
            return None;
        }
        let payload =
            mqtt_ui::panels::publish_panel::encode_payload(publish.format, &publish.payload)
                .ok()?;
        Some(PublishRequest {
            connection_id,
            topic: publish.topic.trim().to_string(),
            payload,
            qos: publish.qos,
            retain: publish.retain,
            properties: None,
        })
    }

    /// The publish template the publish form currently describes.
    pub fn publish_template(&self) -> Option<PublishTemplate> {
        let publish = &self.ui.publish;
        let connection_id = publish.connection_id.clone()?;
        let payload =
            mqtt_ui::panels::publish_panel::encode_payload(publish.format, &publish.payload)
                .ok()?;
        Some(PublishTemplate {
            id: mqtt_ui::panels::left_panel::new_id(),
            name: format!("{} @ {}", publish.topic, connection_id),
            topic: publish.topic.trim().to_string(),
            payload,
            format: publish.format,
            qos: publish.qos,
            retain: publish.retain,
        })
    }

    /// Render the captured messages in the given export format.
    pub fn export_bytes(&self, format: ExportFormat) -> Vec<u8> {
        let messages = self.visible_messages();
        match format {
            ExportFormat::Json => serde_json::to_vec_pretty(
                &messages
                    .iter()
                    .map(|message| {
                        serde_json::json!({
                            "topic": message.topic,
                            "qos": message.qos,
                            "retain": message.retain,
                            "received_at": message.received_at,
                            "version": message.version.label(),
                            "payload": String::from_utf8_lossy(&message.payload),
                            "payload_hex": mqtt_core::format::encode_hex(&message.payload),
                        })
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_else(|error| format!("{{\"error\": \"{error}\"}}").into_bytes()),
            ExportFormat::Csv => {
                let mut out = String::from("time,topic,qos,retain,size,payload_hex\n");
                for message in &messages {
                    out.push_str(&format!(
                        "{},{},{},{},{},{}\n",
                        message.received_at,
                        message.topic,
                        message.qos,
                        message.retain,
                        message.payload.len(),
                        mqtt_core::format::encode_hex(&message.payload)
                    ));
                }
                out.into_bytes()
            }
            ExportFormat::Hex => {
                let mut out = String::new();
                for message in &messages {
                    out.push_str(&format!(
                        "{} {}\n{}\n",
                        message.received_at,
                        message.topic,
                        mqtt_core::format::encode_hex(&message.payload)
                    ));
                }
                out.into_bytes()
            }
            ExportFormat::Binary => {
                let mut out = Vec::new();
                for message in &messages {
                    out.extend_from_slice(&message.payload);
                    out.push(b'\n');
                }
                out
            }
        }
    }

    /// Export the captured messages to a file.
    pub fn export(&self, format: ExportFormat) -> Result<PathBuf, String> {
        let Some(path) = rfd::FileDialog::new()
            .set_file_name("mqtt-messages")
            .save_file()
        else {
            return Err("the export was cancelled".to_string());
        };

        let bytes = self.export_bytes(format);
        std::fs::write(&path, bytes).map_err(|error| error.to_string())?;
        Ok(path)
    }
}

/// Where the application keeps its database.
fn open_storage_in(directory: &std::path::Path) -> Option<Database> {
    let path = directory.join("mqtt-assistant.db");
    match Database::open(path) {
        Ok(database) => Some(database),
        Err(error) => {
            tracing::warn!("could not open the database: {error}");
            None
        }
    }
}

/// The platform configuration directory.
///
/// `MQTT_ASSISTANT_CONFIG_DIR` overrides the location, which is what the tests
/// use to keep their database out of the user's home directory.
fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("MQTT_ASSISTANT_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(dir).join("mqtt-assistant");
    }
    if let Ok(dir) = std::env::var("APPDATA") {
        return PathBuf::from(dir).join("mqtt-assistant");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config").join("mqtt-assistant");
    }
    PathBuf::from(".")
}
