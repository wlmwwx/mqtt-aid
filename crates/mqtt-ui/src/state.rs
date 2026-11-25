//! The state the UI owns, and the intentions it hands back to the application.

use mqtt_core::models::{
    ConnectionStatus, DataSeries, MqttConnection, MqttMessage, PayloadFormat, PublishRequest,
    PublishTemplate, SubscribeRequest,
};

use crate::i18n::LanguageChoice;
use crate::theme::Theme;

/// A connection as the UI sees it: the configuration plus its live status.
#[derive(Debug, Clone)]
pub struct ConnectionView {
    pub config: MqttConnection,
    pub status: ConnectionStatus,
}

/// The name of a connection state, in the language of the interface.
pub fn connection_status_label(status: ConnectionStatus) -> &'static str {
    match status {
        ConnectionStatus::Connected => "已连接",
        ConnectionStatus::Connecting => "连接中",
        ConnectionStatus::Reconnecting => "重连中",
        ConnectionStatus::Error => "错误",
        ConnectionStatus::Disconnected => "未连接",
    }
}

/// The name of a payload format, in the language of the interface.
///
/// The names of the formats are technical, but two of them are worth
/// translating: "自动" and "文本" are words, the rest are names.
pub fn format_label(format: PayloadFormat) -> &'static str {
    match format {
        PayloadFormat::Auto => crate::i18n::t("自动"),
        PayloadFormat::Text => crate::i18n::t("文本"),
        PayloadFormat::Hex => "HEX",
        PayloadFormat::Json => "JSON",
        PayloadFormat::Base64 => "Base64",
        PayloadFormat::Binary => crate::i18n::t("二进制"),
    }
}

/// What should end up on the clipboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyKind {
    Topic,
    Payload,
    Hex,
}

/// A clipboard request.
#[derive(Debug, Clone)]
pub struct CopyRequest {
    pub kind: CopyKind,
    pub value: String,
}

/// A command the menu bar or the ribbon can trigger.
///
/// The menu bar and the ribbon share one vocabulary, so a command added to one
/// is available in the other. An empty id means "whatever is selected right
/// now".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppCommand {
    NewConnection,
    EditConnection(String),
    DeleteConnection(String),
    Connect(String),
    Disconnect(String),
    NewSubscription(String),
    Unsubscribe(String),
    ToggleSubscriptionPause(String),
    ClearMessages,
    TogglePause,
    Export(ExportFormat),
    SetFormat(PayloadFormat),
    FormatJson,
    MinifyJson,
    PublishNow,
    SaveTemplate,
    CopyTopic,
    CopyPayload,
    CopyHex,
    SendToPublisher,
    ToggleTheme,
    ToggleLeftPanel,
    ToggleRightPanel,
    ToggleStatusBar,
    SetCenterTab(CenterTab),
    FocusSearch,
    About,
    Shortcuts,
    Quit,
}

/// The tabs of the centre panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CenterTab {
    /// The table of the captured messages.
    #[default]
    Message,
    /// The charts of the data series.
    Data,
}

impl CenterTab {
    pub fn label(self) -> &'static str {
        match self {
            CenterTab::Message => "消息",
            CenterTab::Data => "数据",
        }
    }

    pub fn all() -> &'static [CenterTab] {
        &[CenterTab::Message, CenterTab::Data]
    }
}

/// What should happen to a data series.
#[derive(Debug, Clone, PartialEq)]
pub enum SeriesAction {
    Add(DataSeries),
    Update(DataSeries),
    Remove(String),
}

/// The categories of the ribbon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RibbonTab {
    #[default]
    Connection,
    Subscription,
    Message,
    Publish,
    View,
}

impl RibbonTab {
    pub fn label(self) -> &'static str {
        match self {
            RibbonTab::Connection => "连接",
            RibbonTab::Subscription => "订阅",
            RibbonTab::Message => "消息",
            RibbonTab::Publish => "发布",
            RibbonTab::View => "视图",
        }
    }

    pub fn all() -> &'static [RibbonTab] {
        &[
            RibbonTab::Connection,
            RibbonTab::Subscription,
            RibbonTab::Message,
            RibbonTab::Publish,
            RibbonTab::View,
        ]
    }
}

/// The format a message list is exported in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Csv,
    Hex,
    Binary,
}

/// The publish form.
#[derive(Debug, Clone)]
pub struct PublishPanelState {
    pub connection_id: Option<String>,
    pub topic: String,
    pub payload: String,
    pub format: PayloadFormat,
    pub qos: u8,
    pub retain: bool,
    pub validation_error: Option<String>,
    pub templates: Vec<PublishTemplate>,
    /// How many spaces a JSON indent is worth.
    pub json_indent: usize,
}

impl Default for PublishPanelState {
    fn default() -> Self {
        Self {
            connection_id: None,
            topic: String::new(),
            payload: String::new(),
            format: PayloadFormat::Auto,
            qos: 0,
            retain: false,
            validation_error: None,
            templates: Vec::new(),
            json_indent: 2,
        }
    }
}

/// The connection editor.
#[derive(Debug, Clone)]
pub struct ConnectionDraft {
    pub config: MqttConnection,
    pub password: String,
    pub is_new: bool,
}

/// The subscribe form.
#[derive(Debug, Clone)]
pub struct SubscribeDraft {
    pub connection_id: String,
    pub topic: String,
    pub qos: u8,
}

/// The editor of a data series.
#[derive(Debug, Clone)]
pub struct SeriesDraft {
    pub series: DataSeries,
    pub is_new: bool,
}

/// Counters shown in the status bar.
#[derive(Debug, Clone, Copy, Default)]
pub struct Stats {
    pub received: u64,
    pub published: u64,
    pub messages_per_second: f64,
    pub cached: usize,
}

/// Everything the UI needs to draw itself.
#[derive(Debug, Clone)]
pub struct UiState {
    pub theme: Theme,
    /// The language the interface speaks.
    pub language: LanguageChoice,
    pub selected_connection: Option<String>,
    pub selected_subscription: Option<String>,
    pub selected_message: Option<usize>,
    pub search: String,
    pub filter_qos: Option<u8>,
    pub filter_retain: Option<bool>,
    pub filter_connection: Option<String>,
    pub message_format: PayloadFormat,
    pub publish: PublishPanelState,
    pub connection_draft: Option<ConnectionDraft>,
    pub subscribe_draft: Option<SubscribeDraft>,
    /// The editor of a data series.
    pub series_draft: Option<SeriesDraft>,
    /// Which tab of the centre panel is shown.
    pub center_tab: CenterTab,
    pub left_width: f32,
    pub right_width: f32,
    pub show_left: bool,
    pub show_right: bool,
    pub show_settings: bool,
    pub show_status: bool,
    /// The collapsed state of the three dockable panels.
    pub left_collapsed: bool,
    pub right_collapsed: bool,
    pub status_collapsed: bool,
    pub ribbon_tab: RibbonTab,
    pub command_search: String,
    /// The menu that should be open, for the screenshot helper.
    pub open_menu: Option<String>,
    /// The message the copy commands act on.
    pub copy_target: Option<MqttMessage>,
    /// The search box should take the keyboard focus.
    pub focus_search: bool,
    /// The about and the shortcut window are open.
    pub show_about: bool,
    pub show_shortcuts: bool,
    pub paused: bool,
    pub stats: Stats,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            language: LanguageChoice::default(),
            selected_connection: None,
            selected_subscription: None,
            selected_message: None,
            search: String::new(),
            filter_qos: None,
            filter_retain: None,
            filter_connection: None,
            message_format: PayloadFormat::Auto,
            publish: PublishPanelState::default(),
            connection_draft: None,
            subscribe_draft: None,
            series_draft: None,
            center_tab: CenterTab::default(),
            left_width: 260.0,
            right_width: 320.0,
            show_left: true,
            show_right: true,
            show_settings: false,
            show_status: true,
            left_collapsed: false,
            right_collapsed: false,
            status_collapsed: false,
            ribbon_tab: RibbonTab::default(),
            command_search: String::new(),
            open_menu: None,
            copy_target: None,
            focus_search: false,
            show_about: false,
            show_shortcuts: false,
            paused: false,
            stats: Stats::default(),
        }
    }
}

/// Everything the user did that the application has to act on.
#[derive(Debug, Default)]
pub struct UiActions {
    pub connect: Vec<String>,
    pub disconnect: Vec<String>,
    pub delete_connection: Vec<String>,
    pub save_connection: Vec<MqttConnection>,
    pub save_password: Vec<(String, String)>,
    pub subscribe: Vec<SubscribeRequest>,
    pub unsubscribe: Vec<(String, String)>,
    pub set_qos: Vec<(String, String, u8)>,
    pub toggle_subscription_pause: Vec<String>,
    pub set_subscription_format: Vec<(String, PayloadFormat)>,
    pub publish: Vec<PublishRequest>,
    pub save_template: Vec<PublishTemplate>,
    pub load_template: Vec<String>,
    pub delete_template: Vec<String>,
    pub copy: Vec<CopyRequest>,
    pub export: Option<ExportFormat>,
    pub clear_messages: bool,
    pub toggle_pause: bool,
    pub send_to_publisher: Option<MqttMessage>,
    pub toggle_theme: bool,
    pub set_cache_limit: Option<usize>,
    /// What should happen to the data series.
    pub series: Vec<SeriesAction>,
    /// Commands issued by the menu bar and the ribbon.
    pub commands: Vec<AppCommand>,
}
