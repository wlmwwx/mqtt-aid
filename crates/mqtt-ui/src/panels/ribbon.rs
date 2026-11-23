//! The ribbon: a tab strip and the command groups of the active tab.
//!
//! It follows SolidCraft's layout: a row of category tabs with the command
//! search on the right, and below it the commands of the active category,
//! grouped by kind, each command drawn as an icon with a caption underneath,
//! with the group name below the group.

use egui::{Align, Layout, Rect, RichText, Stroke, Ui};

use crate::state::{AppCommand, CenterTab, ExportFormat, RibbonTab, UiActions, UiState};
use crate::theme::spacing;
use crate::widgets::icons::{self, Icon};

/// One command in the ribbon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    /// The caption under the icon.
    pub label: &'static str,
    /// The icon above the caption.
    pub icon: Icon,
    /// The name of the group the command belongs to.
    pub group: &'static str,
    /// What the command does.
    pub action: AppCommand,
}

/// The commands of a ribbon tab, in the order they are drawn.
pub fn commands_for(tab: RibbonTab) -> Vec<Command> {
    let command = |label, icon, group, action| Command {
        label,
        icon,
        group,
        action,
    };

    match tab {
        RibbonTab::Connection => vec![
            command("新建连接", Icon::Plus, "创建", AppCommand::NewConnection),
            command(
                "编辑",
                Icon::Pencil,
                "修改",
                AppCommand::EditConnection(String::new()),
            ),
            command(
                "删除",
                Icon::Trash,
                "修改",
                AppCommand::DeleteConnection(String::new()),
            ),
            command(
                "连接",
                Icon::Plug,
                "连接",
                AppCommand::Connect(String::new()),
            ),
            command(
                "断开",
                Icon::Power,
                "连接",
                AppCommand::Disconnect(String::new()),
            ),
        ],
        RibbonTab::Subscription => vec![
            command("新建订阅", Icon::Plus, "创建", AppCommand::NewSubscription),
            command(
                "取消订阅",
                Icon::Trash,
                "修改",
                AppCommand::Unsubscribe(String::new()),
            ),
            command(
                "暂停",
                Icon::Pause,
                "修改",
                AppCommand::ToggleSubscriptionPause(String::new()),
            ),
            command("筛选", Icon::Filter, "显示", AppCommand::FocusSearch),
        ],
        RibbonTab::Message => vec![
            command("清除", Icon::Trash, "消息", AppCommand::ClearMessages),
            command("暂停", Icon::Pause, "消息", AppCommand::TogglePause),
            command(
                "导出 JSON",
                Icon::Download,
                "导出",
                AppCommand::Export(ExportFormat::Json),
            ),
            command(
                "导出 CSV",
                Icon::Download,
                "导出",
                AppCommand::Export(ExportFormat::Csv),
            ),
            command(
                "导出 HEX",
                Icon::Download,
                "导出",
                AppCommand::Export(ExportFormat::Hex),
            ),
            command(
                "导出字节",
                Icon::Download,
                "导出",
                AppCommand::Export(ExportFormat::Binary),
            ),
        ],
        RibbonTab::Publish => vec![
            command("发布", Icon::Send, "发布", AppCommand::PublishNow),
            command("格式化 JSON", Icon::Layers, "工具", AppCommand::FormatJson),
            command("压缩 JSON", Icon::Layers, "工具", AppCommand::MinifyJson),
            command("保存模板", Icon::Copy, "模板", AppCommand::SaveTemplate),
        ],
        RibbonTab::View => vec![
            command(
                "数据面板",
                Icon::Chart,
                "面板",
                AppCommand::SetCenterTab(CenterTab::Data),
            ),
            command(
                "消息面板",
                Icon::Layers,
                "面板",
                AppCommand::SetCenterTab(CenterTab::Message),
            ),
            command("左栏", Icon::Layers, "面板", AppCommand::ToggleLeftPanel),
            command("右栏", Icon::Layers, "面板", AppCommand::ToggleRightPanel),
            command("状态栏", Icon::Layers, "面板", AppCommand::ToggleStatusBar),
            command("深色", Icon::Moon, "主题", AppCommand::ToggleTheme),
            command("浅色", Icon::Sun, "主题", AppCommand::ToggleTheme),
        ],
    }
}

/// The ribbon: the tab strip and the command groups.
///
/// Returns where each command was drawn, so that a test can click one.
pub fn ribbon(ui: &mut Ui, state: &mut UiState, actions: &mut UiActions) -> Vec<(String, Rect)> {
    tab_strip(ui, state);

    let placed = egui::Frame::new()
        .fill(ui.visuals().panel_fill)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .stroke(Stroke::NONE)
        .show(ui, |ui| {
            ui.set_min_height(spacing::RIBBON_HEIGHT);
            ribbon_body(ui, state, actions)
        })
        .inner;

    // The hairline that separates the ribbon from the panels below it.
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::ZERO,
            ui.visuals().widgets.noninteractive.fg_stroke.color,
        );
    }

    placed
}

/// The command groups of the active tab.
///
/// The groups are laid out by hand: a nested layout would give every group
/// the whole width of the ribbon, and the groups would run off the window.
fn ribbon_body(ui: &mut Ui, state: &mut UiState, actions: &mut UiActions) -> Vec<(String, Rect)> {
    let height = spacing::RIBBON_HEIGHT - 8.0;
    let cell = spacing::COMMAND_CELL;
    let painter = ui.painter().clone();
    let palette = state.theme.palette();
    // The icons carry the accent colour, the way SolidCraft's do.
    let icon_color = palette.icon;
    let icon_color_alt = palette.icon_alt;
    // The caption is bright: it names the command.
    let caption = palette.text;
    let hover = palette.accent_text;

    let groups = layout(state);
    let origin = ui.cursor().left_top();
    let placed = place(&groups, origin, height);

    // The rules between the groups.
    let mut x = origin.x + spacing::GAP;
    for (index, group) in groups.iter().enumerate() {
        if index > 0 {
            let separator_x = x + spacing::GAP / 2.0;
            painter.line_segment(
                [
                    egui::pos2(separator_x, origin.y),
                    egui::pos2(separator_x, origin.y + height - 12.0),
                ],
                egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.fg_stroke.color),
            );
            x += spacing::GAP;
        }
        x += group.commands.len() as f32 * cell + spacing::GAP;
    }

    for (index, (label, rect)) in placed.iter().enumerate() {
        let is_odd = index % 2 == 1;
        let command = groups
            .iter()
            .flat_map(|group| group.commands.iter())
            .find(|command| command.label == label)
            .expect("the command was placed");

        let icon_rect = egui::Rect::from_center_size(
            egui::pos2(rect.center().x, origin.y + spacing::ICON.y / 2.0 + 4.0),
            spacing::ICON,
        );

        let response = ui.interact(
            *rect,
            ui.scope_id().with(command.label),
            egui::Sense::click(),
        );
        let color = if response.hovered() {
            hover
        } else {
            icon_color
        };
        // Every second icon in a group takes the secondary colour, which gives
        // the ribbon the varied look it has in SolidCraft.
        let color = if is_odd {
            if response.hovered() {
                hover
            } else {
                icon_color_alt
            }
        } else {
            color
        };
        icons::draw(&painter, command.icon, icon_rect, color, 1.6);

        let galley = painter.layout_no_wrap(
            command.label.to_string(),
            egui::FontId::proportional(11.0),
            caption,
        );
        painter.galley(
            egui::pos2(
                rect.center().x - galley.size().x / 2.0,
                icon_rect.bottom() + 4.0,
            ),
            galley,
            caption,
        );

        if response.clicked() {
            actions.commands.push(command.action.clone());
        }
    }

    // The name of every group, centred under the group.
    for group in &groups {
        let group_width = group.commands.len() as f32 * cell;
        let galley = painter.layout_no_wrap(
            group.name.to_string(),
            egui::FontId::proportional(11.0),
            ui.visuals().weak_text_color(),
        );
        let group_center = group
            .commands
            .first()
            .map(|command| {
                placed
                    .iter()
                    .find(|(label, _)| *label == command.label)
                    .map(|(_, rect)| rect.center().x)
                    .unwrap_or(origin.x)
            })
            .unwrap_or(origin.x)
            + group_width / 2.0
            - cell / 2.0;

        painter.galley(
            egui::pos2(
                group_center - galley.size().x / 2.0,
                origin.y + height - galley.size().y,
            ),
            galley,
            palette.text_group,
        );
    }

    // Reserve the space the groups took.
    let used = placed
        .last()
        .map(|(_, rect)| rect.right() - origin.x + spacing::GAP)
        .unwrap_or(0.0);
    ui.allocate_space(egui::vec2(used, height));

    placed
}

/// One group of commands in the ribbon.
pub struct Group {
    pub name: &'static str,
    pub commands: Vec<Command>,
}

/// The groups of the ribbon, after the command search was applied.
pub fn layout(state: &UiState) -> Vec<Group> {
    let search = state.command_search.trim().to_lowercase();
    let mut groups: Vec<Group> = Vec::new();
    for command in commands_for(state.ribbon_tab) {
        if !search.is_empty() && !command.label.to_lowercase().contains(&search) {
            continue;
        }
        match groups.iter_mut().find(|group| group.name == command.group) {
            Some(group) => group.commands.push(command),
            None => groups.push(Group {
                name: command.group,
                commands: vec![command],
            }),
        }
    }
    groups
}

/// Where every command of the ribbon is drawn.
///
/// The ribbon draws its commands itself rather than with a layout, so this
/// function is the single source of truth about where a command lives.
pub fn place(groups: &[Group], origin: egui::Pos2, height: f32) -> Vec<(String, egui::Rect)> {
    let cell = spacing::COMMAND_CELL;
    let mut x = origin.x + spacing::GAP;
    let top = origin.y;

    let mut placed = Vec::new();
    for (index, group) in groups.iter().enumerate() {
        if index > 0 {
            x += spacing::GAP;
        }
        for command in &group.commands {
            placed.push((
                command.label.to_string(),
                egui::Rect::from_min_size(egui::pos2(x, top), egui::vec2(cell, height)),
            ));
            x += cell;
        }
        x += spacing::GAP;
    }
    placed
}

/// The position of a command of a tab, for a ribbon that starts at `origin`.
pub fn command_position(tab: RibbonTab, origin: egui::Pos2, label: &str) -> Option<egui::Pos2> {
    let state = UiState {
        ribbon_tab: tab,
        ..Default::default()
    };
    let placed = place(&layout(&state), origin, spacing::RIBBON_HEIGHT - 8.0);
    placed
        .iter()
        .find(|(name, _)| name == label)
        .map(|(_, rect)| rect.center())
}
/// The row of category tabs, with the command search on the right.
fn tab_strip(ui: &mut Ui, state: &mut UiState) {
    let palette = state.theme.palette();

    egui::Frame::new()
        .fill(ui.visuals().panel_fill)
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for tab in RibbonTab::all() {
                    let active = state.ribbon_tab == *tab;
                    let text = if active {
                        RichText::new(tab.label())
                            .color(palette.accent_text)
                            .strong()
                    } else {
                        // The tabs that are not selected are still bright: they
                        // are the way to the other commands.
                        RichText::new(tab.label()).color(palette.text)
                    };

                    // The selected tab gets a blue plate, the way SolidCraft
                    // marks the active category.
                    let response =
                        crate::widgets::tabs::tab_button(ui, text, active, palette.tab_active);
                    if response.clicked() {
                        state.ribbon_tab = *tab;
                    }
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let response = ui.add(
                        egui::Button::new(RichText::new("+").color(palette.text_dim))
                            .fill(egui::Color32::TRANSPARENT),
                    );
                    response.on_hover_text("自定义工具栏");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.command_search)
                            .hint_text("搜索命令")
                            .desired_width(160.0),
                    );
                });
            });
        });
}

/// A thin separator used between the ribbon groups.
pub fn group_separator(ui: &mut Ui) {
    ui.add(egui::Separator::default().vertical().spacing(spacing::GAP));
}
