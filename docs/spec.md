# MQTT Assistant V1.0 — 实现规格

> 本规格由 `docs/mqtt.prd` 推导而来，遵循 to-spec 模板。仓库当前为空工程（仅 `Cargo.toml` 与空的 `src/main.rs`），因此本规格同时定义整体架构。

## Problem Statement

MQTT 开发者与硬件联调人员目前只能依赖 `mosquitto_sub` / `mosquitto_pub` 这类命令行工具或浏览器端 WebSocket 客户端来排查 Broker 上的报文。命令行工具无法同时观察多条 Topic、无法并排比对 Text/HEX/JSON 三种视角、无法保留 MQTT 5.0 的属性信息，也没有历史检索能力；而基于浏览器的客户端在处理二进制负载时会把原始字节直接塞进文本编辑器，导致 `\x00` 之类的字节被 UTF-8 替换规则悄悄篡改，调试者看到的报文与 Broker 实际发出的字节并不一致。

用户需要一个原生、跨平台的桌面 MQTT 调试工具：能管理多个 Broker 连接、独立订阅、实时监控消息流、精确查看原始字节、方便地发布带 QoS/Retain 的报文，并且所有这些操作都不能因为消息洪流而卡死界面。

## Solution

交付 **MQTT Assistant V1.0**：一个纯 Rust 实现的原生桌面应用，基于 egui + eframe 构建三栏式界面，rumqttc + Tokio 承载全部 MQTT 通信。

核心形态：

- **左栏** 连接与订阅列表；**中栏** 实时消息监控表 + 消息详情；**右栏** 发布面板。底部状态栏显示连接、收/发计数与速率。
- **消息负载永远是原始字节**。所有展示格式（Text / HEX / JSON / Binary / Base64 / Auto）都是原始字节的视图，绝不回写、绝不破坏原始数据。
- **GUI 线程不碰网络**。egui 主线程只做绘制与交互，所有 MQTT 往返发生在 Tokio 后台运行时，经由 Command / Event 双向通道与 GUI 通信。
- **多连接并行**，每个 Broker 拥有独立的客户端、事件循环、状态与订阅空间。

## User Stories

### 连接管理

1. As a MQTT 调试工程师, I want to create a named broker connection, so that I can keep several environments (dev / staging / prod) side by side.
2. As a MQTT 调试工程师, I want to edit any saved connection, so that I can correct a host or port without recreating it.
3. As a MQTT 调试工程师, I want to delete a saved connection, so that my workspace does not accumulate stale entries.
4. As a MQTT 调试工程师, I want to connect to a broker, so that I can start exchanging messages.
5. As a MQTT 调试工程师, I want to disconnect from a broker, so that I can release resources and stop the traffic noise.
6. As a MQTT 调试工程师, I want to reconnect an existing connection, so that I do not have to re-enter credentials.
7. As a MQTT 调试工程师, I want to see a live connection status indicator (Disconnected / Connecting / Connected / Reconnecting / Error), so that I always know whether traffic can flow.
8. As a MQTT 调试工程师, I want the connection list to show the status of every connection independently, so that one broken broker does not hide a healthy one.
9. As a MQTT 调试工程师, I want to configure the MQTT protocol version (3.1.1 or 5.0) per connection, so that I can test both against the same broker.
10. As a MQTT 调试工程师, I want to set a client ID, so that I can control broker-side session identity and avoid client ID collisions.
11. As a MQTT 调试工程师, I want to configure a username and password, so that I can reach brokers that require authentication.
12. As a MQTT 调试工程师, I want to store passwords in the OS keychain rather than in a plaintext config file, so that my credentials are not exposed on disk.
13. As a MQTT 调试工程师, I want to configure keep-alive, so that I can match the broker's idle expectations.
14. As a MQTT 调试工程师, I want to toggle clean session (3.1.1) and clean start (5.0), so that I can test persistent session behaviour.
15. As a MQTT 调试工程师, I want to enable TLS with a CA certificate, so that I can talk to brokers over an encrypted channel.
16. As a MQTT 调试工程师, I want to present a client certificate and client key for mutual TLS, so that I can reach brokers that require mTLS.
17. As a MQTT 调试工程师, I want certificate verification to be on by default, so that I am not silently exposed to man-in-the-middle attacks.
18. As a MQTT 调试工程师, I want automatic reconnection with backoff, so that a flaky network does not end my session.
19. As a MQTT 调试工程师, I want to see the specific error text when a connection fails, so that I know whether it is a DNS, TLS, or authentication problem.
20. As a MQTT 调试工程师, I want a connection test that reports success or failure, so that I can validate a config before relying on it.
21. As a MQTT 调试工程师, I want connection configs to survive an app restart, so that I do not rebuild my workspace every morning.
22. As a MQTT 调试工程师, I want to cap the number of simultaneous connections, so that a misconfiguration cannot exhaust resources.

### 订阅管理

23. As a MQTT 调试工程师, I want to subscribe to an exact topic, so that I receive exactly the messages I care about.
24. As a MQTT 调试工程师, I want to subscribe with the `+` single-level wildcard, so that I can watch a whole device family at once.
25. As a MQTT 调试工程师, I want to subscribe with the `#` multi-level wildcard, so that I can capture everything under a subtree.
26. As a MQTT 调试工程师, I want invalid topic filters to be rejected with an explanation, so that I do not silently subscribe to nothing.
27. As a MQTT 调试工程师, I want to unsubscribe, so that I can stop noise on a chatty topic.
28. As a MQTT 调试工程师, I want to change the QoS of an existing subscription, so that I can trade reliability against bandwidth.
29. As a MQTT 调试工程师, I want to pause the display of a subscription without unsubscribing, so that I can stop the scrolling but keep the session state.
30. As a MQTT 调试工程师, I want to resume a paused subscription, so that I can pick up where I left off.
31. As a MQTT 调试工程师, I want to give a subscription a friendly alias, so that raw topic strings do not dominate my workspace.
32. As a MQTT 调试工程师, I want a per-subscription default display format, so that a binary topic opens as HEX and a JSON topic opens as JSON.
33. As a MQTT 调试工程师, I want subscription configs to persist across restarts, so that my monitoring setup is reproducible.
34. As a MQTT 调试工程师, I want the subscription list scoped to the currently selected connection, so that I never mix two brokers' subscriptions.

### 消息发布

35. As a MQTT 调试工程师, I want to publish to any topic, so that I can drive the device under test.
36. As a MQTT 调试工程师, I want to publish plain UTF-8 text, so that the common case is one keystroke.
37. As a MQTT 调试工程师, I want to publish hexadecimal bytes with space-separated and unspaced input, so that I can send exact binary frames.
38. As a MQTT 调试工程师, I want live validation of my HEX input with a byte-count readout, so that I catch a typo before sending.
39. As a MQTT 调试工程师, I want to publish JSON with syntax checking, so that I do not send malformed payloads.
40. As a MQTT 调试工程师, I want to format and minify my JSON payload, so that I can produce both human- and machine-readable variants.
41. As a MQTT 调试工程师, I want to choose the indent width for JSON, so that the payload matches the device's expectation.
42. As a MQTT 调试工程师, I want to choose publish QoS 0, 1 or 2, so that I can test each delivery guarantee.
43. As a MQTT 调试工程师, I want a Retain toggle, so that I can test retained state on the device side.
44. As a MQTT 调试工程师, I want MQTT 5.0 publish properties (content type, payload format indicator, message expiry, response topic, correlation data, user properties), so that I can exercise 5.0-only features.
45. As a MQTT 调试工程师, I want to save publish templates, so that I can re-send a recurring command with one click.
46. As a MQTT 调试工程师, I want to load a template into the publish panel, so that I can tweak and resend it.
47. As a MQTT 调试工程师, I want to delete a template, so that my palette stays current.
48. As a MQTT 调试工程师, I want confirmation feedback that a publish succeeded, so that I know it actually left the app.

### 消息监控与详情

49. As a MQTT 调试工程师, I want a live-updating message table, so that I see traffic as it happens.
50. As a MQTT 调试工程师, I want each row to show time, topic, QoS, retain flag, size and a content preview, so that I can triage at a glance.
51. As a MQTT 调试工程师, I want to select a message and see full details, so that I can inspect what really arrived.
52. As a MQTT 调试工程师, I want the detail view to show the exact payload length in bytes, so that I am not misled by character count.
53. As a MQTT 调试工程师, I want the detail view to show the MQTT version and MQTT 5.0 properties, so that I can verify broker behaviour.
54. As a MQTT 调试工程师, I want to copy a topic to the clipboard, so that I can paste it into a terminal.
55. As a MQTT 调试工程师, I want to copy the payload and separately copy its HEX form, so that I can compare textual and byte representations.
56. As a MQTT 调试工程师, I want to send a received message straight to the publish panel, so that I can echo or replay it.
57. As a MQTT 调试工程师, I want to switch the display format of a received message without losing the original bytes, so that I can pivot between views.
58. As a MQTT 调试工程师, I want to pause and resume the incoming stream, so that I can read the screen during a burst.
59. As a MQTT 调试工程师, I want to clear the message list, so that I can start a fresh capture.
60. As a MQTT 调试工程师, I want to search by topic, so that I can find a specific stream among thousands of rows.
61. As a MQTT 调试工程师, I want to search inside payload content, so that I can find a message by its value.
62. As a MQTT 调试工程师, I want to filter by QoS, retain flag, connection and time range, so that I can isolate a specific class of traffic.
63. As a MQTT 调试工程师, I want the filter and search controls to compose, so that I can narrow by topic and by time together.
64. As a MQTT 调试工程师, I want messages from different connections kept separate, so that two brokers never interleave in one table.

### 数据格式引擎

65. As a MQTT 调试工程师, I want to view a payload as Text, so that I read human-readable messages directly.
66. As a MQTT 调试工程师, I want a hex dump with offset, hex and ASCII columns, so that I can read binary frames in the familiar format.
67. As a MQTT 调试工程师, I want to see the total byte count of a payload, so that I know its true size.
68. As a MQTT 调试工程师, I want to view a payload as a JSON tree with expandable nodes, so that I can navigate a deeply nested document.
69. As a MQTT 调试工程师, I want to copy an individual JSON node's value, so that I can lift one field out.
70. As a MQTT 调试工程师, I want a precise error with location when JSON is invalid, so that I can fix it.
71. As a MQTT 调试工程师, I want Auto format to detect JSON, then text, then fall back to HEX, so that the common case needs no manual choice.
72. As a MQTT 调试工程师, I want Base64 and Binary views, so that I can transport or inspect payloads the way my tooling expects.
73. As a MQTT 调试工程师, I want a format codec interface that new formats can plug into, so that XML or CBOR support is additive.
74. As a MQTT 调试工程师, I want invalid UTF-8 bytes to be shown as HEX rather than mangled, so that the data I see is the data that was sent.

### 数据序列与图表

75. As a MQTT 调试工程师, I want to watch one field of the messages of a topic over time, so that I see the trend without reading the messages one by one.
76. As a MQTT 调试工程师, I want to pick the field with a path like `data.temperature`, so that I can read a value out of a nested JSON document.
77. As a MQTT 调试工程师, I want to watch a topic whose payload is a plain number, so that I can plot simple sensors too.
78. As a MQTT 调试工程师, I want several series on one chart, so that I can compare two values of the same device.
79. As a MQTT 调试工程师, I want the series to survive a restart, so that my dashboard is still there tomorrow.
80. As a MQTT 调试工程师, I want the values to keep coming while the message list is paused, so that pausing the table does not stop the charts.
81. As a MQTT 调试工程师, I want the chart to ignore messages that do not carry the field, so that one bad message does not break the line.

### 性能、稳定性与安全

82. As a MQTT 调试工程师, I want the UI to stay responsive under a sustained 100 messages/second, so that I keep working while traffic flows.
83. As a MQTT 调试工程师, I want the in-memory cache capped at a configurable limit (default 10,000), so that memory stays bounded.
84. As a MQTT 调试工程师, I want the message list to be virtualized, so that a huge list scrolls without lag.
85. As a MQTT 调试工程师, I want previews truncated to a sensible length, so that one huge payload does not stall rendering.
86. As a MQTT 调试工程师, I want payloads displayed up to 1 MB, so that large-but-real payloads remain inspectable.
87. As a MQTT 调试工程师, I want the app to survive a broker that drops the connection, so that a network blip is not a crash.
88. As a MQTT 调试工程师, I want the app to release connections and shut the runtime down cleanly on exit, so that I can close it without leaving sockets behind.
89. As a MQTT 调试工程师, I want to export captured messages as JSON, CSV, HEX or raw binary, so that I can hand evidence to someone else.
90. As a MQTT 调试工程师, I want binary export to preserve the original bytes exactly, so that an exported file re-decodes identically.
91. As a MQTT 调试工程师, I want passwords never written to logs, so that my logs are safe to share.
92. As a MQTT 调试工程师, I want a dark and a light theme, so that the app matches my environment.
93. As a MQTT 调试工程师, I want resizable and collapsible side panels, so that I can give the message table the whole screen when needed.
94. As a MQTT 调试工程师, I want the app to respect display scaling, so that text is legible on a HiDPI screen.

## Implementation Decisions

### 分层与工作区结构

采用 Cargo workspace，五个 crate 严格分层，依赖方向单向向下：

- **mqtt-core** — 纯领域模型与格式处理。不依赖 egui，不依赖 Tokio，不依赖网络库。
- **mqtt-engine** — MQTT 通信与异步任务，依赖 mqtt-core。
- **mqtt-storage** — SQLite 持久化，依赖 mqtt-core。
- **mqtt-ui** — egui 界面，只依赖 mqtt-core（依赖方向向下的关键约束）。
- **mqtt-app** — 应用入口与协调，依赖全部四个。

依赖方向：`mqtt-app → {mqtt-ui, mqtt-engine, mqtt-storage} → mqtt-core`。禁止反向依赖与环。

### 领域模型（mqtt-core）

- `MqttVersion`：`V311` / `V5`。
- `ConnectionStatus`：`Disconnected` / `Connecting` / `Connected` / `Reconnecting` / `Error`（与 PRD 一致）。
- `PayloadFormat`：`Auto` / `Text` / `Hex` / `Json` / `Binary` / `Base64`。
- `MqttConnection`：PRD §10.1 的字段，加上 `clean_session` / `clean_start`、TLS 三件套路径、`keep_alive` 等；密码**不**存放于结构体内。
- `MqttSubscription`：PRD §10.2 字段，加上 `alias`。
- `MqttMessage`：PRD §10.3 字段，加上 `version`。
- `MqttProperties`：MQTT 5.0 属性的 core 侧表示（content type、payload format indicator、message expiry、response topic、correlation data、user properties），由 engine 在适配层从 rumqttc 转换而来——core 不认识 rumqttc。

### 凭据与连接配置分离

`MqttConnection` 只保存 `credential_ref: Option<String>`。密码写入系统 keyring（service 名固定为 `mqtt-assistant`，account 为 connection id）。engine 在建连时按需从 keyring 取密码，取不到则退化为无密码连接并向 GUI 报 Error 事件。

### 格式引擎（mqtt-core/format）

PRD §7.6 的 `PayloadCodec` trait 原样保留（`decode` / `encode`），实现 `TextCodec` / `HexCodec` / `JsonCodec` / `BinaryCodec` / `Base64Codec` 五个实现。`Auto` 不作为一个 codec 实现，而是作为一个**探测函数** `detect_format(&[u8]) -> PayloadFormat`，规则按 PRD §7.3：合法 JSON → Json；合法 UTF-8 → Text；否则 → Hex。

关键约束：`decode` 只产生视图，`encode` 只在发布路径上使用。任何 codec 都**不得**改写原始 `Vec<u8>`。

HEX 编解码：接受空格分隔与无空格混合输入，奇数长度或非法字符时报错并指出位置；输出统一为 `01 03 00 0A` 形式的大写 hex dump。JSON 编解码用 `serde_json`，提供格式化（可配缩进）、压缩、树节点提取。

### Topic 过滤（mqtt-core/topic）

`validate_filter(&str) -> Result<(), TopicError>` 实现 MQTT 3.1.1 §4.7.1 的规则：`#` 只能作为整层且必须在末尾；`+` 必须独占一层。`matches(filter, topic) -> bool` 实现标准匹配语义（`#` 匹配剩余所有层，`+` 匹配恰好一层）。该函数是订阅列表展示与本地过滤的基础。

### 引擎与线程模型（mqtt-engine）

- 引擎持有一个多线程 Tokio runtime（在 `std::thread` 中独立启动，GUI 线程不参与）。
- 对外暴露 `MqttCommand`（PRD §12.1）与 `MqttEvent`（PRD §12.2）两个枚举，二者构成 GUI 与引擎的唯一契约。
- 命令通过无界 mpsc channel 发往引擎；事件通过无界 mpsc channel 发回 GUI。
- **每个连接一个独立任务**，任务内持有该连接的 `AsyncClient` + `EventLoop`，用 `tokio::select!` 在「接收连接级命令」与「轮询网络事件」之间选择。
- 连接级命令：`Subscribe` / `Unsubscribe` / `Publish` / `Disconnect` / `SetSubscriptionQos`。
- **自动重连**：事件循环返回错误且连接配置开启 `auto_reconnect` 时，状态置为 `Reconnecting`，以指数退避（1s 起，上限 30s）重建事件循环；重建期间连接级命令继续排队不丢。
- **重连后重新订阅**：Broker 在会话结束时丢弃该会话的订阅，所以连接一旦重新连上（手动断开再连、自动重连、或者应用重启后重新连接），应用必须再订阅一次。订阅列表以应用持久化的数据为准（`mqtt-app` 在收到 `Connected` 状态时按存储的订阅重新下发 `Subscribe`），这样被用户删掉的订阅不会复活，而重启前的订阅会自动恢复。
- 状态传播：连接任务直接更新共享的状态表并发出 `MqttEvent::ConnectionStatusChanged`。
- 订阅确认：`SubscriptionAcknowledged` 在收到 Broker 的 **SUBACK** 时才发出，而不是在请求入队时——这样它可以作为「订阅已生效」的可靠屏障。
- GUI 侧非阻塞消费：`try_recv` 循环 + 每帧最多消费的条数上限，避免单帧卡死。
- **协议版本由两套实现承担**：`rumqttc` 0.25 把 3.1.1 与 5.0 拆成两个互不兼容的客户端类型，因此引擎为每个版本各有一套会话任务（`run_v311` / `run_v5`），共用同一套命令与事件语义。

### 消息缓存与过滤（mqtt-app）

消息缓存属于**应用层**而不是引擎：引擎只负责投递消息，缓存、限长、过滤与统计都在 mqtt-app 中完成。

- 内存缓存：`VecDeque<MqttMessage>`，容量上限可配置（默认 10,000），超出时从头部弹出。
- 过滤：在缓存之上建立惰性过滤视图，条件包括 topic 包含、payload 包含、QoS 集合、retain 标志、连接 id、时间区间。过滤只影响展示，不影响缓存。
- 消息列表用 `egui::ScrollArea::show_rows` 实现虚拟列表。

### 数据序列与图表（mqtt-core / mqtt-app / mqtt-ui）

「显示特定主题收到消息里特定数据」的最小实现：一条**数据序列** = 一个连接 + 一个主题过滤器 + 一个字段路径。收到消息时按序列取一个数值，攒成时间序列，画成折线图。

- **取值规则在 core**（`mqtt_core::extract::extract_value`）：路径是点分隔的字段路径，数组写作 `[0]`（如 `sensors[0].value`）；空路径表示整个负载，此时负载必须本身是一个数字。JSON 里的数字直接取值，布尔当 1/0，字符串能解析成数字就取。负载不是 JSON 时没有字段，只有空路径能取到东西。
- **序列属于一个连接**：同一个主题名可能存在于多个 Broker，所以序列带 `connection_id`，取值时连接与主题都要对上。
- **消息要先到得了**：序列的主题如果还没有被任何订阅覆盖，应用会补一条订阅（QoS 0），否则这条序列永远没有数据。已经被覆盖的主题不重复订阅——同一主题订阅两次会把每条消息送两遍。补上的这条订阅是用户的所有物：删除序列不会删除它，删除连接会连同序列一起删除。
- **样本只存在内存里**：每条序列保留最近 2000 个样本，超出从头部丢弃。序列的**配置**（名称、连接、主题、路径）持久化在 SQLite 的 `data_series` 表里，重启后还在。
- **图表在中栏的「数据」页**：中栏顶部是 `消息 / 数据` 两个标签页；数据页上半部分是序列列表（色点、名称、连接、主题、路径、样本数、最新值），下半部分是折线图。列表本身就是图例，所以图上不再画图例。横轴是相对最早样本的秒数，纵轴是数值。
- **取值与绘制分离**：core 决定「一个负载里有没有我想要的数」，app 决定「哪些消息归哪条序列」，ui 只负责画。三层各自可测。

### TLS 约束

`TlsConfiguration::Simple` 只接受显式的 CA 链，因此**客户端证书认证要求同时配置 CA 证书**；没有 CA 时回退到系统根证书（仅在未使用客户端证书时）。这条约束在连接编辑界面中被显式提示。

### 存储（mqtt-storage）

SQLite 单文件，位于平台配置目录下。表：`connections`、`subscriptions`、`publish_templates`、`data_series`、`app_settings`（`messages` 表建好但 V1.0 不启用持久化，与 PRD §9.1 一致）。

- 每个 repo 暴露 `save` / `list` / `delete`（及 `update`）四个基本操作，序列化用 `serde_json` 存结构化字段。
- 数据库连接不可 `Sync`，因此 `Database` 内部用 `Mutex<Connection>` 串行化访问。
- 密码不入库。

### UI（mqtt-ui）

界面参照 SolidCraft 的桌面 CAD 布局：**顶部应用栏 + 菜单栏 + 功能区（ribbon）**，下方是**左栏 / 中栏 / 右栏**三栏，底部是状态栏。

- **应用栏**（`Panel::top`，固定 38px）：应用图标与名称、菜单栏入口、右侧的连接状态胶囊、版本号、主题与设置图标。
- **菜单栏**：`文件` / `编辑` / `视图` / `连接` / `订阅` / `工具` / `帮助` 七个下拉菜单，跟 SolidCraft 一样放在标题栏里。
- **功能区（ribbon）**：一排分类标签（连接 / 订阅 / 消息 / 发布 / 视图）+ 当前分类的命令组，每组以「图标 + 说明」呈现，组间有竖直分隔线，组名下沉在底部；标签行右侧是命令搜索框。命令组的位置由代码自行排布（不用嵌套布局），避免每组都占满整个宽度。
- **左栏 / 右栏 / 底部状态栏**：都有 SolidCraft 风格的面板头——左侧是拖拽握把，中间是面板名，右侧是「…」菜单与折叠箭头。左栏是连接与订阅树，右栏是发布面板，中栏是消息监控，底部状态栏显示连接状态与收发计数。
- **中栏分两页**：顶部是 `消息 / 数据` 两个标签页。消息页是消息表与详情，数据页是数据序列的列表与折线图。

- mqtt-ui 只接受 `&mut Ui` + 数据 + 一个 `UiActions` 意图结构体。UI 不直接调用 engine，也不持有 engine 句柄；它把用户操作写入 `UiActions`，由 mqtt-app 翻译成 `MqttCommand`。
- **菜单栏与功能区共享同一套命令词汇**（`AppCommand`）：菜单项和 ribbon 命令都产出 `AppCommand`，由 mqtt-app 统一翻译。命令可以带一个目标 id；空 id 表示「作用于当前选中项」。
- 主题：`dark` / `light` 两种预设，通过 `egui::Visuals` 切换。
- 消息详情按当前选中格式渲染 HEX dump / JSON tree / 文本；JSON tree 支持节点展开折叠与节点值复制。
- 中栏里有两个滚动区域：消息表与 JSON 树。egui 的滚动区域 id 默认是「父 ui + `scroll_area`」，两个在同一 ui 里的滚动区域会撞 id，egui 会在窗口上画一条 `Second use of ScrollArea ID` 的错误。所以两者各自带 `id_salt`（`message_table` / `json_payload`）。消息表的高度留出详情区的高度，详情区因此始终可见。
- **滚动条**：egui 的滚动区域默认会缩到内容的宽度，滚动条于是停在面板中间，看着像是右边那块面板的。要让它待在面板的边上，滚动区域得写 `auto_shrink([false, true])`：宽度铺满，高度照旧。滚动条的样式走 `Style::spacing.scroll`：悬浮在内容之上，不占地方；手柄闲置时也留一点（30%）的可见度，不然暗色主题下根本看不见，只有拖动的时候才冒出来。
- HEX / JSON 编辑器复用同一个文本编辑控件 + 实时校验，避免引入 Monaco 之类的 Web 依赖（PRD §6.2 已明确）。

### 主题与图标

低饱和度配色，圆角与间距统一走 egui 的 `CornerRadius` / `Spacing` 常量，集中定义在 mqtt-ui 的 theme 模块中。

配色不是估的，而是**从 SolidCraft 实际截图逐像素采样**出来的，因此两个应用放在一起是同一套视觉语言：

| 用途 | 颜色 |
|---|---|
| 标题栏 | `#1C1F26` |
| 面板与功能区 | `#272B34` |
| 画布 | `#2E3440` |
| 分隔线 | `#3F4552` |
| 主文字 | `#DBE4EE` |
| 次要文字 | `#8F96A2` |
| 强调色 | `#3C64E6` |
| 强调文字（选中标签） | `#9BC6F4` |
| 选中标签底色 | `#2A466E` |
| 功能区图标 | `#4EA1DC` / `#3287D1` |
| 成功 / 危险 | `#3CAA3C` / `#FF5E5B` |

SolidCraft 的功能区图标是彩色的，命令说明是亮色，只有组名和提示才是暗色；选中的分类用一块蓝底加亮蓝字，而不是一条下划线。这些细节一并照搬。

图标不用字体而是用 `Painter` 现场绘制（`widgets/icons.rs`）：这样在任何缩放比下都清晰、能带颜色，也不依赖字体里有没有那个字形。

### 截图辅助（开发工具）

`mqtt-app` 带有一个可选的 `screenshot` 特性：设置 `MQTT_ASSISTANT_SCREENSHOT` 后，应用渲染若干帧、把窗口写成 PNG 并退出；`MQTT_ASSISTANT_OPEN_MENU` 可以让某一个菜单在截图时保持展开；`MQTT_ASSISTANT_CENTER_TAB`（`message` / `data`）决定中栏显示哪一页。

截图请求必须发生在**真正绘制了的一帧**里：窗口没有有效表面时 egui 会拒绝截图。因此辅助工具拆成两步——`App::logic` 负责计帧并决定何时退出，`App::ui`（只在窗口被绘制时调用）负责发起截图请求。

### 依赖选型

`rumqttc`（启用 `use-rustls`）、`tokio`（rt-multi-thread / sync / time / macros）、`eframe` + `egui` + `egui_plot`（折线图）、`rusqlite`（bundled）、`serde` + `serde_json`、`thiserror`、`tracing` + `tracing-subscriber`、`keyring`、`rfd`、`base64`。

## Testing Decisions

### 什么是好测试

只断言**外部可观察行为**：发出去的字节、收到的事件、持久化后的数据、渲染出的文本。不测私有字段、不测内部函数名、不 mock 掉被测行为本身。断言尽量落在公开 API 边界上。

### 唯一测试接缝：引擎事件通道

整个项目只保留**一个**测试接缝——`MqttEngine` 的 Command/Event 通道。测试以「应用」的方式使用引擎：

1. 启动一个真实 broker（`rumqttd` 作为 dev-dependency，嵌入测试进程，监听随机端口）。
2. `engine.execute(MqttCommand::Connect(config))`。
3. 循环 `engine.poll_events()` 收事件，直到期望的 `MqttEvent::MessageReceived` 出现或超时。
4. 对 topic、payload 字节、QoS、retain 做断言。

这个接缝同时覆盖了连接、订阅、发布、QoS、retain、自动重连、协议版本切换——比在 core 里做纯单元测试覆盖的真实路径多得多，因为 core 单元测试永远验证不了「订阅真的收到了消息」。

**例外**：payload 格式引擎有大量与网络无关的边界（HEX 奇数长度、非法 UTF-8、JSON 截断、Auto 探测），这些仍然在 core 层做直接单元测试——但它们走的是 core 的公开 API（`PayloadCodec` trait、`detect_format`），不新增测试接缝，只是同一接缝下更细的断言点。

### 分层

- **core 单元测试**（mqtt-core 内联 `#[cfg(test)]`）：hex 编解码、JSON 编解码与格式化、base64、Auto 探测、topic filter 校验与匹配、模型序列化往返。
- **引擎集成测试**（mqtt-engine/tests/）：以 rumqttd 为 broker，跑通连接、订阅（含 `+` / `#` 通配符）、发布、QoS 0/1/2、retain、断开、错误处理、MQTT 5.0 连接与属性。
- **存储测试**（mqtt-storage/tests/）：连接、订阅、模板、设置在 SQLite 中的往返。
- **应用层端到端测试**（mqtt-app/tests/）：`AppState` + 真实 broker，验证「连接 → 订阅 → 收到消息 → 进入缓存」的完整链路，以及过滤、导出与持久化。
- **界面测试**（mqtt-ui/tests/）：用 egui 的无头模式真的去点菜单和 ribbon——`Context::run_ui` 跑一帧，按下并松开指针，断言菜单项与 ribbon 命令真的产出了对应的 `AppCommand`。这保证「菜单看得见但点了没反应」这类问题不会溜过去。

应用层测试复用同一条接缝（引擎的命令/事件通道），只是多了一层状态与存储，因此**没有新增接缝**。

### 测试隔离

每个集成测试启动自己的 rumqttd 实例并绑定 `127.0.0.1:0`（由操作系统分配端口），测试之间互不干扰，也不需要外部 broker 在运行。测试用 `#[tokio::test]` 或 `#[test]` + 内部 runtime，并在超时后强制断开，避免测试挂死。

### 先例

仓库为空，无既有测试可参照；约定：集成测试文件按能力域命名（`mqtt_connection.rs`、`mqtt_subscribe.rs`、`mqtt_publish.rs`、`mqtt_protocol.rs`），core 单元测试与被测模块同文件内联。

## Out of Scope

- **数据 Dashboard / 图表**（PRD §16.1）——温度曲线、湿度曲线等数值提取与 egui 绘图。
- **自动化测试套件**（PRD §16.2）——定时发布、批量发布、响应验证、测试结果统计。
- **自定义二进制协议解析**（PRD §16.3）——用户自定义 Offset 字段规则并解析为结构化数据。
- **CLI**（PRD §16.4）——mqtt-assistant-cli 及其共享 core/engine 的设计。
- **消息持久化**（PRD §9.1 第二阶段）——`messages` 表建好但不写入；历史消息仅存在于内存缓存与导出文件中。
- **MQTT 5.0 之外的新协议**——不支持 MQTT 3.1（仅 3.1.1 与 5.0）。
- **WebSocket 传输**——rumqttc 支持的 ws/wss 连接方式不在 V1.0 范围内，仅 TCP + TLS。
- **多用户 / 远程协作**——纯本地单用户桌面应用。
- **消息人工标记、收藏、批注**——PRD 未定义的消息整理能力。

## Further Notes

- PRD 的目录结构把 `tests/` 放在 workspace 根。Rust 的集成测试必须位于某个 crate 的 `tests/` 目录下才会被 cargo 收集，因此集成测试实际落在各自的 crate 内；根目录不创建 `tests/`。
- V1.0 的「连接测试」按钮没有单独实现：连接动作本身就承担了这个职责——它要么进入 `Connected`，要么给出具体的错误原因（DNS / TLS / 认证）。
- GUI 的输入过滤不覆盖时间区间筛选；当前提供 topic、payload、QoS、retain 与连接维度。
- PRD §12.3 提到「GUI 使用定时批量消费消息」。实现上采用每帧 `try_recv` 批量取走（上限可配），比定时器更平滑，且在消息稀疏时不产生无谓唤醒。
- 性能验收（AC17，100 条/秒）通过引擎集成测试用批量发布 1000 条并断言全部到达来近似验证；GUI 帧率不纳入自动化测试范畴。
- 断线重连采用固定的指数退避（1 秒起，上限 30 秒）；缓存上限可在设置面板调整，默认 10,000 条，与 PRD 一致。
