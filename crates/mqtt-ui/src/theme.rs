//! The visual language of the application.
//!
//! The palette and the metrics follow SolidCraft: a very dark, slightly blue
//! tinted grey, one accent blue, hairline separators and generous rounding.
//! Everything is low saturation so that the data stays the loudest thing on
//! screen.

use egui::{Color32, Stroke, Visuals};

/// The two themes the application ships with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    pub fn toggled(self) -> Theme {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        }
    }

    /// The palette for this theme.
    ///
    /// The colours are sampled from a real SolidCraft screenshot, so the two
    /// applications look like they belong together.
    pub fn palette(self) -> Palette {
        match self {
            Theme::Dark => Palette {
                app_bar: Color32::from_rgb(0x1C, 0x1F, 0x26),
                surface: Color32::from_rgb(0x27, 0x2B, 0x34),
                canvas: Color32::from_rgb(0x2E, 0x34, 0x40),
                border: Color32::from_rgb(0x34, 0x3A, 0x46),
                border_strong: Color32::from_rgb(0x3F, 0x45, 0x52),
                text: Color32::from_rgb(0xDB, 0xE4, 0xEE),
                text_dim: Color32::from_rgb(0x8F, 0x96, 0xA2),
                text_group: Color32::from_rgb(0x8F, 0x96, 0xA2),
                text_faint: Color32::from_rgb(0x54, 0x59, 0x63),
                accent: Color32::from_rgb(0x3C, 0x64, 0xE6),
                accent_text: Color32::from_rgb(0x9B, 0xC6, 0xF4),
                // The fill of the selected ribbon tab.
                tab_active: Color32::from_rgb(0x2A, 0x46, 0x6E),
                // The colour of the ribbon icons.
                icon: Color32::from_rgb(0x4E, 0xA1, 0xDC),
                // The secondary colour of the ribbon icons.
                icon_alt: Color32::from_rgb(0x32, 0x87, 0xD1),
                success: Color32::from_rgb(0x3C, 0xAA, 0x3C),
                warning: Color32::from_rgb(0xC8, 0xA2, 0x4A),
                danger: Color32::from_rgb(0xFF, 0x5E, 0x5B),
            },
            Theme::Light => Palette {
                app_bar: Color32::from_rgb(0xF2, 0xF3, 0xF5),
                surface: Color32::from_rgb(0xFA, 0xFB, 0xFC),
                canvas: Color32::from_rgb(0xF0, 0xF2, 0xF5),
                border: Color32::from_rgb(0xD8, 0xDC, 0xE2),
                border_strong: Color32::from_rgb(0xC2, 0xC8, 0xD0),
                text: Color32::from_rgb(0x25, 0x29, 0x30),
                text_dim: Color32::from_rgb(0x5A, 0x61, 0x6B),
                text_group: Color32::from_rgb(0x6B, 0x73, 0x7E),
                text_faint: Color32::from_rgb(0x93, 0x99, 0xA3),
                accent: Color32::from_rgb(0x2F, 0x54, 0xC4),
                accent_text: Color32::from_rgb(0x1D, 0x4E, 0xA8),
                tab_active: Color32::from_rgb(0xD6, 0xE4, 0xF7),
                icon: Color32::from_rgb(0x2F, 0x7A, 0xC4),
                icon_alt: Color32::from_rgb(0x1D, 0x5E, 0xA0),
                success: Color32::from_rgb(0x2E, 0x7D, 0x32),
                warning: Color32::from_rgb(0x9A, 0x6B, 0x12),
                danger: Color32::from_rgb(0xC0, 0x36, 0x33),
            },
        }
    }

    /// Turn the theme into egui visuals.
    pub fn visuals(self) -> Visuals {
        let palette = self.palette();
        let mut visuals = if self == Theme::Dark {
            Visuals::dark()
        } else {
            Visuals::light()
        };
        visuals.panel_fill = palette.surface;
        visuals.window_fill = palette.surface;
        visuals.extreme_bg_color = palette.canvas;
        visuals.faint_bg_color = palette.surface;
        visuals.override_text_color = Some(palette.text);
        visuals.hyperlink_color = palette.accent_text;
        // The dim text of SolidCraft: the captions and the group names.
        visuals.weak_text_color = Some(palette.text_dim);
        visuals.error_fg_color = palette.danger;
        visuals.warn_fg_color = palette.warning;
        visuals.selection.bg_fill = palette.accent.gamma_multiply(0.35);
        visuals.selection.stroke = Stroke::new(1.0, palette.accent);

        visuals.widgets.noninteractive.bg_fill = palette.surface;
        visuals.widgets.noninteractive.weak_bg_fill = palette.surface;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text_faint);
        // The stroke of an inactive widget: the arrow of a drop-down and
        // the handle of a scroll bar are drawn with it.
        visuals.widgets.inactive.bg_fill = palette.surface;
        visuals.widgets.inactive.weak_bg_fill = palette.surface;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette.text_dim);
        visuals.widgets.hovered.bg_fill = palette.border;
        visuals.widgets.hovered.weak_bg_fill = palette.border;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, palette.accent_text);
        visuals.widgets.active.bg_fill = palette.accent;
        visuals.widgets.active.weak_bg_fill = palette.accent;
        // Strong text takes the colour of the active widget stroke, so this is
        // also the colour of every bold label.
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, palette.text);
        visuals.widgets.open.bg_fill = palette.surface;

        visuals
    }

    /// The style of the scroll bars.
    ///
    /// The scroll bars float above the content, so they do not take any room
    /// away from it. The handle stays faintly visible when the mouse is
    /// elsewhere: a scroll bar that only shows up while it is dragged is a
    /// scroll bar nobody notices.
    pub fn scroll_style() -> egui::style::ScrollStyle {
        let mut scroll = egui::style::ScrollStyle::floating();
        scroll.bar_width = 10.0;
        scroll.floating_width = 4.0;
        scroll.dormant_handle_opacity = 0.30;
        scroll.active_handle_opacity = 0.55;
        scroll.interact_handle_opacity = 0.95;
        scroll
    }
}

/// The colours of a theme.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    /// The window title bar.
    pub app_bar: Color32,
    /// Panels, the ribbon and the tree.
    pub surface: Color32,
    /// The canvas the messages are drawn on.
    pub canvas: Color32,
    /// Hairline separators.
    pub border: Color32,
    /// A more visible separator.
    pub border_strong: Color32,
    pub text: Color32,
    /// The colour of secondary text: captions and hints.
    pub text_dim: Color32,
    /// The colour of the ribbon group names, a step brighter than `text_dim`.
    pub text_group: Color32,
    /// The colour of the least important text: the grip, the chevron.
    pub text_faint: Color32,
    pub accent: Color32,
    /// The colour of the accent text: the selected tab, the links.
    pub accent_text: Color32,
    /// The fill of the selected ribbon tab.
    pub tab_active: Color32,
    /// The colour of the ribbon icons.
    pub icon: Color32,
    /// The secondary colour of the ribbon icons.
    pub icon_alt: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
}

/// Shared spacing constants.
pub mod spacing {
    use egui::{Color32, CornerRadius, Margin, Stroke, Vec2};

    /// The rounding of cards, panels and buttons.
    pub const CORNER: CornerRadius = CornerRadius::same(6);
    /// The rounding of the ribbon commands.
    pub const CORNER_SMALL: CornerRadius = CornerRadius::same(4);
    /// The inner padding of a panel.
    pub const PADDING: Margin = Margin::symmetric(10, 10);
    /// The gap between groups.
    pub const GAP: f32 = 8.0;
    /// The height of a row in the message list.
    pub const ROW_HEIGHT: f32 = 22.0;
    /// The height the detail view of the selected message takes.
    pub const DETAILS_HEIGHT: f32 = 280.0;
    /// The height of the JSON tree of the detail view.
    pub const JSON_VIEW_HEIGHT: f32 = 180.0;
    /// The height of the window title bar.
    pub const APP_BAR_HEIGHT: f32 = 38.0;
    /// The height of the ribbon tab strip.
    pub const TAB_HEIGHT: f32 = 30.0;
    /// The height of the ribbon body.
    pub const RIBBON_HEIGHT: f32 = 96.0;
    /// The size of a ribbon icon.
    pub const ICON: Vec2 = Vec2::new(26.0, 26.0);
    /// The width of one ribbon command.
    pub const COMMAND_CELL: f32 = 58.0;
    /// A hairline separator.
    pub fn separator() -> Stroke {
        Stroke::new(1.0, Color32::from_rgb(0x34, 0x3A, 0x46))
    }
}
