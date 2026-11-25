//! The languages of the interface and the texts of every one of them.
//!
//! The source language is Chinese: the strings in the code are the Chinese
//! ones, and the other languages live in the catalogue below. A string that
//! is not in the catalogue is shown as it is written, which is why a test
//! walks through the sources and complains about every Chinese string that
//! the catalogue forgot.
//!
//! A string can mean two things — the menu of the connections and the
//! command that connects — so a text may carry a context, the way the
//! messages of gettext do: `tc(Some("ribbon"), "连接")` is the command and
//! `t("连接")` is the menu.
//!
//! The language of the window is a property of the window, not of the
//! widget: it is kept in a thread local, and the application sets it once
//! per frame. The interface is drawn on one thread, so the thread local is
//! the one place where the language can be read from anywhere without
//! threading it through every function.
//!
//! The user may also ask the interface to follow the language of the
//! system, which is what it does unless it is told otherwise. That choice
//! is a `LanguageChoice`, not a `Language`: the choice has to be kept as it
//! was made, because the settings window has to show which one is picked.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::OnceLock;

/// A language the interface speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    /// Chinese, the language the sources are written in.
    #[default]
    Chinese,
    /// English.
    English,
}

impl Language {
    /// The name of the language, in that language.
    pub fn label(self) -> &'static str {
        match self {
            Language::Chinese => "中文",
            Language::English => "English",
        }
    }

    /// The code the language is stored under.
    pub fn code(self) -> &'static str {
        match self {
            Language::Chinese => "zh",
            Language::English => "en",
        }
    }

    /// The language of a code, if there is one.
    pub fn from_code(code: &str) -> Option<Language> {
        match code.trim().to_lowercase().as_str() {
            "zh" | "zh-cn" | "chinese" | "中文" => Some(Language::Chinese),
            "en" | "en-us" | "english" => Some(Language::English),
            _ => None,
        }
    }
}

/// The language the user asked the interface to speak.
///
/// The choice is not always a language: it may also be "the one of the
/// system", which is what the interface does unless it is told otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LanguageChoice {
    /// The language of the system.
    #[default]
    System,
    /// Always Chinese.
    Chinese,
    /// Always English.
    English,
}

impl LanguageChoice {
    /// The name of the choice, in the language of the interface.
    ///
    /// The names of the languages are written in the languages themselves,
    /// but the name of the choice has to be read, so it is written in the
    /// language of the interface.
    pub fn label(self) -> &'static str {
        match self {
            LanguageChoice::System => t("跟随系统"),
            LanguageChoice::Chinese => Language::Chinese.label(),
            LanguageChoice::English => Language::English.label(),
        }
    }

    /// The code the choice is stored under.
    pub fn code(self) -> &'static str {
        match self {
            LanguageChoice::System => "system",
            LanguageChoice::Chinese => Language::Chinese.code(),
            LanguageChoice::English => Language::English.code(),
        }
    }

    /// The choice of a code, if there is one.
    pub fn from_code(code: &str) -> Option<LanguageChoice> {
        let code = code.trim().to_lowercase();
        match code.as_str() {
            "system" | "auto" => Some(LanguageChoice::System),
            _ => Language::from_code(&code).map(|language| match language {
                Language::Chinese => LanguageChoice::Chinese,
                Language::English => LanguageChoice::English,
            }),
        }
    }

    /// The language the choice means.
    ///
    /// The choice of the system is read once and then kept: the language of
    /// the system does not change while the program runs.
    pub fn resolve(self) -> Language {
        match self {
            LanguageChoice::System => system_language(),
            LanguageChoice::Chinese => Language::Chinese,
            LanguageChoice::English => Language::English,
        }
    }

    /// Every choice, in the order they are offered.
    pub fn all() -> &'static [LanguageChoice] {
        &[
            LanguageChoice::System,
            LanguageChoice::Chinese,
            LanguageChoice::English,
        ]
    }
}

/// The language of the system.
///
/// Everything that is not Chinese is English: the interface only speaks
/// these two, and a system that speaks a third language is served better
/// by English than by Chinese.
pub fn system_language() -> Language {
    static SYSTEM: OnceLock<Language> = OnceLock::new();
    *SYSTEM.get_or_init(|| {
        let locale = sys_locale::get_locale().unwrap_or_default();
        if locale.to_lowercase().starts_with("zh") {
            Language::Chinese
        } else {
            Language::English
        }
    })
}

thread_local! {
    /// The language of the window that is being drawn.
    static CURRENT: Cell<Language> = const { Cell::new(Language::Chinese) };
}

/// Set the language of the interface.
///
/// The application calls this once per frame, before anything is drawn.
pub fn set_language(choice: LanguageChoice) {
    CURRENT.with(|current| current.set(choice.resolve()));
}

/// The language of the interface.
pub fn language() -> Language {
    CURRENT.with(|current| current.get())
}

/// The text of a string in the language of the interface.
///
/// A string that the catalogue does not know is shown as it is written.
pub fn t(msgid: &'static str) -> &'static str {
    tc(None, msgid)
}

/// The text of a string that has a context.
///
/// A context tells two strings that are written the same apart: the menu of
/// the connections and the command that connects.
pub fn tc(context: Option<&'static str>, msgid: &'static str) -> &'static str {
    match language() {
        // The sources are written in Chinese: the string is the text.
        Language::Chinese => msgid,
        Language::English => texts().get(&(context, msgid)).copied().unwrap_or(msgid),
    }
}

/// A text of the interface with its placeholders filled in.
///
/// The placeholders of a text are written `{}`, in the order they are filled
/// in, so that a translation can move them around: `连接：{}` becomes
/// `Connection: {}`.
pub fn fill(text: &'static str, args: &[&str]) -> String {
    let mut filled = text.to_string();
    for arg in args {
        filled = filled.replacen("{}", arg, 1);
    }
    filled
}

/// The texts of the interface in English, by string and context.
fn texts() -> &'static HashMap<(Option<&'static str>, &'static str), &'static str> {
    static TEXTS: OnceLock<HashMap<(Option<&'static str>, &'static str), &'static str>> =
        OnceLock::new();
    TEXTS.get_or_init(|| {
        CATALOGUE
            .iter()
            .map(|(context, msgid, english)| ((*context, *msgid), *english))
            .collect()
    })
}

/// Every string of the interface, in Chinese and in English.
///
/// The first field is the context, the second the Chinese string and the
/// third its English.
const CATALOGUE: &[(Option<&str>, &str, &str)] = &[
    // The menus of the title bar.
    (None, "文件", "File"),
    (None, "编辑", "Edit"),
    (None, "视图", "View"),
    (None, "连接", "Connections"),
    (Some("label"), "连接", "Connection"),
    (None, "订阅", "Subscription"),
    (None, "工具", "Tools"),
    (None, "帮助", "Help"),
    (None, "新建连接…", "New connection…"),
    (
        None,
        "创建一个新的 Broker 连接",
        "Create a new broker connection",
    ),
    (None, "退出", "Quit"),
    (None, "清除消息列表", "Clear the message list"),
    (None, "复制 Topic", "Copy topic"),
    (None, "复制 Payload", "Copy payload"),
    (None, "复制 HEX", "Copy hex"),
    (None, "消息面板", "Message panel"),
    (None, "消息格式", "Message format"),
    (None, "更多", "More"),
    (None, "数据面板", "Data panel"),
    (None, "深色主题", "Dark theme"),
    (None, "浅色主题", "Light theme"),
    (None, "断开", "Disconnect"),
    (None, "新建订阅…", "New subscription…"),
    (None, "取消订阅…", "Unsubscribe…"),
    (None, "暂停 / 恢复", "Pause / resume"),
    (None, "导出消息…", "Export the messages…"),
    (None, "查找…", "Find…"),
    (None, "格式化 JSON", "Format the JSON"),
    (None, "压缩 JSON", "Minify the JSON"),
    (None, "关于 MQTT Assistant", "About MQTT Assistant"),
    (None, "快捷键", "Shortcuts"),
    (None, "发送到发布面板", "Send to the publisher"),
    // The commands of the ribbon.
    (Some("ribbon"), "新建连接", "New connection"),
    (Some("ribbon"), "编辑", "Edit"),
    (Some("ribbon"), "删除", "Delete"),
    (Some("ribbon"), "连接", "Connect"),
    (Some("ribbon"), "断开", "Disconnect"),
    (Some("ribbon"), "新建订阅", "New subscription"),
    (Some("ribbon"), "取消订阅", "Unsubscribe"),
    (Some("ribbon"), "暂停", "Pause"),
    (Some("ribbon"), "筛选", "Filter"),
    (Some("ribbon"), "清除", "Clear"),
    (Some("ribbon"), "导出 JSON", "Export as JSON"),
    (Some("ribbon"), "导出 CSV", "Export as CSV"),
    (Some("ribbon"), "导出 HEX", "Export as hex"),
    (Some("ribbon"), "导出字节", "Export as bytes"),
    (Some("ribbon"), "发布", "Publish"),
    (Some("ribbon"), "保存模板", "Save the template"),
    (Some("ribbon"), "数据面板", "Data panel"),
    (Some("ribbon"), "消息面板", "Message panel"),
    (Some("ribbon"), "左栏", "Left panel"),
    (Some("ribbon"), "右栏", "Right panel"),
    (Some("ribbon"), "状态栏", "Status bar"),
    (Some("ribbon"), "深色", "Dark"),
    (Some("ribbon"), "浅色", "Light"),
    // The names of the groups of the ribbon.
    (Some("group"), "创建", "Create"),
    (Some("group"), "修改", "Change"),
    (Some("group"), "连接", "Connect"),
    (Some("group"), "显示", "Show"),
    (Some("group"), "消息", "Messages"),
    (Some("group"), "导出", "Export"),
    (Some("group"), "工具", "Tools"),
    (Some("group"), "模板", "Templates"),
    (Some("group"), "面板", "Panels"),
    (Some("group"), "主题", "Theme"),
    // The browser.
    (None, "浏览器", "Browser"),
    (None, "还没有连接", "No connections yet"),
    (None, "新建", "New"),
    (None, "选择连接", "Select the connection"),
    (None, "编辑连接", "Edit the connection"),
    (None, "删除连接", "Delete the connection"),
    (None, "新建订阅", "New subscription"),
    (None, "取消订阅", "Unsubscribe"),
    (None, "暂停显示", "Pause the display"),
    (None, "恢复显示", "Resume the display"),
    (None, "选择订阅", "Select the subscription"),
    (None, "· {} 订阅", "· {} subscriptions"),
    (None, "（连接已删除）", "(the connection is gone)"),
    // The title bar.
    (None, "设置", "Settings"),
    (None, "切换主题", "Switch the theme"),
    // The status bar.
    (None, "状态", "Status"),
    (None, "连接：{}", "Connection: {}"),
    (None, "状态：{}", "Status: {}"),
    (None, "接收：{}", "Received: {}"),
    (None, "发布：{}", "Published: {}"),
    (None, "{} 条/秒", "{} msg/s"),
    (None, "缓存：{}", "Cached: {}"),
    (None, "⏸ 已暂停", "⏸ Paused"),
    // The message monitor.
    (
        None,
        "搜索 topic 或 payload…",
        "Search a topic or a payload…",
    ),
    (None, "任意 QoS", "Any QoS"),
    (None, "全部", "All"),
    (None, "全部连接", "All connections"),
    (None, "导出…", "Export…"),
    (None, "导出为 JSON", "Export as JSON"),
    (None, "导出为 CSV", "Export as CSV"),
    (None, "导出为 HEX", "Export as hex"),
    (None, "导出为原始字节", "Export as raw bytes"),
    (None, "暂停接收", "Pause the reception"),
    (None, "继续接收", "Resume the reception"),
    (Some("column"), "保留", "Retained"),
    (None, "实时", "Live"),
    (None, "还没有捕获到消息", "No messages yet"),
    (
        None,
        "订阅一个主题，消息就会出现在这里",
        "Subscribe to a topic and the messages will show up here",
    ),
    (None, "时间", "Time"),
    (None, "主题", "Topic"),
    (None, "大小", "Size"),
    (None, "负载", "Payload"),
    // The publish panel.
    (None, "清空表单", "Clear the form"),
    (None, "没有连接", "No connection"),
    (None, "格式", "Format"),
    (None, "自动", "Auto"),
    (None, "文本", "Text"),
    (None, "二进制", "Binary"),
    (None, "格式：", "Format:"),
    (None, "复制主题", "Copy topic"),
    (None, "复制负载", "Copy payload"),
    (None, "MQTT 5.0 属性", "MQTT 5.0 properties"),
    (None, "内容类型：{}", "Content type: {}"),
    (None, "负载格式标志：{}", "Payload format indicator: {}"),
    (None, "消息过期时间：{} 秒", "Message expiry interval: {} s"),
    (None, "响应主题：{}", "Response topic: {}"),
    (None, "关联数据：{}", "Correlation data: {}"),
    (None, "用户属性 {} = {}", "User property {} = {}"),
    (None, "缩进", "Indent"),
    (None, "{} 字节", "{} bytes"),
    (None, "服务质量", "QoS"),
    (Some("checkbox"), "保留", "Retain"),
    (None, "还没有保存模板", "No templates yet"),
    (None, "保存当前为模板", "Save as a template"),
    (None, "删除模板", "Delete the template"),
    // The data panel.
    (None, "+ 添加序列", "+ Add a series"),
    (
        None,
        "从消息里取值，画成曲线",
        "Read a value out of the messages and draw it as a curve",
    ),
    (None, "还没有数据序列", "No data series yet"),
    (None, "编辑序列", "Edit the series"),
    (None, "删除序列", "Delete the series"),
    (None, "{} 点", "{} points"),
    (None, "最新 {:.3}", "Latest {:.3}"),
    (None, "新序列", "New series"),
    (None, "整个负载", "The whole payload"),
    (
        None,
        "字段路径用点分隔，数组写作 [0]；留空表示整个负载（负载要是一个数字）",
        "The field path is separated by dots and an array is written [0]; empty means the whole payload (the payload has to be a number)",
    ),
    (
        None,
        "MQTT 3.1.1 / 5.0 桌面调试工具",
        "The MQTT 3.1.1 / 5.0 desktop debugging tool",
    ),
    (
        None,
        "连接 · 订阅 · 发布 · 实时监控",
        "Connections · Subscriptions · Publishing · Live monitoring",
    ),
    (
        None,
        "消息负载永远是原始字节：Text / HEX / JSON / Binary / Base64 都只是视图。",
        "The payload of a message is always raw bytes: Text / HEX / JSON / Binary / Base64 are only views.",
    ),
    (None, "导出消息", "Export the messages"),
    (None, "搜索命令", "Search the commands"),
    (None, "数值", "Value"),
    (None, "断开连接", "Disconnect"),
    (None, "暂停 / 继续接收", "Pause / resume the reception"),
    (None, "温度", "Temperature"),
    (None, "秒", "Seconds"),
    (None, "自定义工具栏", "Customise the toolbar"),
    // The panel headers.
    (None, "面板菜单", "Panel menu"),
    (None, "折叠面板", "Collapse the panel"),
    // The dialogs.
    (None, "新建连接", "New connection"),
    (Some("dialog"), "编辑连接", "Edit connection"),
    (None, "名称", "Name"),
    (None, "主机", "Host"),
    (None, "端口", "Port"),
    (None, "协议", "Protocol"),
    (None, "客户端 ID", "Client ID"),
    (None, "用户名", "Username"),
    (
        None,
        "密码（保存在系统钥匙串里）",
        "Password (kept in the system keyring)",
    ),
    (None, "保活（秒）", "Keep alive (seconds)"),
    (None, "清除会话 / 干净启动", "Clean session / clean start"),
    (None, "自动重连", "Automatic reconnect"),
    (None, "使用 TLS", "Use TLS"),
    (None, "CA 证书", "CA certificate"),
    (None, "客户端证书", "Client certificate"),
    (None, "客户端密钥", "Client key"),
    (None, "保存", "Save"),
    (None, "取消", "Cancel"),
    (Some("button"), "订阅", "Subscribe"),
    (None, "主题过滤器", "Topic filter"),
    (
        None,
        "`+` 匹配一层，`#` 匹配剩下的树",
        "`+` matches one level, `#` matches the rest of the tree",
    ),
    (None, "新数据序列", "New data series"),
    (None, "编辑数据序列", "Edit the data series"),
    (None, "字段路径", "Field path"),
    (None, "外观", "Appearance"),
    (None, "深色", "Dark"),
    (None, "浅色", "Light"),
    (None, "消息", "Messages"),
    (None, "发布", "Publish"),
    (None, "模板", "Templates"),
    (None, "保留", "Retain"),
    (None, "数据", "Data"),
    (None, "缓存的消息", "Cached messages"),
    (None, "关闭", "Close"),
    (None, "语言", "Language"),
    (None, "跟随系统", "Follow the system"),
    (None, "当前系统语言：{}", "The system language is {}"),
    // The windows of information.
    (None, "关于", "About"),
    (
        None,
        "消息负载永远是原始字节：Text、Hex、JSON、Base64 与二进制都不会被改写，只在显示的时候换个样子。",
        "The payload of a message is always raw bytes: Text, Hex, JSON, Base64 and binary are never rewritten, they only look different when they are shown.",
    ),
    (None, "聚焦搜索框", "Focus the search box"),
    // The states of a connection.
    (None, "已连接", "Connected"),
    (None, "连接中", "Connecting"),
    (None, "重连中", "Reconnecting"),
    (None, "错误", "Error"),
    (None, "未连接", "Disconnected"),
];

/// How many texts the catalogue holds.
pub fn texts_count() -> usize {
    CATALOGUE.len()
}

/// Whether the catalogue knows a string.
pub fn knows(context: Option<&str>, msgid: &str) -> bool {
    CATALOGUE
        .iter()
        .any(|(entry_context, entry_msgid, _)| *entry_context == context && *entry_msgid == msgid)
}

/// Whether the catalogue knows a string, whatever its context.
pub fn knows_any(msgid: &str) -> bool {
    CATALOGUE
        .iter()
        .any(|(_, entry_msgid, _)| *entry_msgid == msgid)
}
