//! Vector icons.
//!
//! The icons are drawn with the painter instead of taken from an icon font:
//! that keeps them crisp at any size, lets them carry a colour, and avoids
//! depending on a font that may not have the glyph.

use egui::{Color32, CornerRadius, Painter, Rect, Shape, Stroke, StrokeKind, Vec2, vec2};

/// The icons the application uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Plus,
    Trash,
    Pencil,
    Plug,
    Power,
    Pause,
    Play,
    Download,
    Upload,
    Search,
    Gear,
    Moon,
    Sun,
    Filter,
    Copy,
    ChevronUp,
    ChevronDown,
    ChevronRight,
    Menu,
    Dots,
    Eye,
    EyeOff,
    Topic,
    Connection,
    Layers,
    Send,
    Grip,
    Ellipsis,
}

impl Icon {
    /// The icon as a character, for tooltips and tests.
    pub fn name(self) -> &'static str {
        match self {
            Icon::Plus => "plus",
            Icon::Trash => "trash",
            Icon::Pencil => "pencil",
            Icon::Plug => "plug",
            Icon::Power => "power",
            Icon::Pause => "pause",
            Icon::Play => "play",
            Icon::Download => "download",
            Icon::Upload => "upload",
            Icon::Search => "search",
            Icon::Gear => "gear",
            Icon::Moon => "moon",
            Icon::Sun => "sun",
            Icon::Filter => "filter",
            Icon::Copy => "copy",
            Icon::ChevronUp => "chevron up",
            Icon::ChevronDown => "chevron down",
            Icon::ChevronRight => "chevron right",
            Icon::Menu => "menu",
            Icon::Dots => "dots",
            Icon::Eye => "eye",
            Icon::EyeOff => "eye off",
            Icon::Topic => "topic",
            Icon::Connection => "connection",
            Icon::Layers => "layers",
            Icon::Send => "send",
            Icon::Grip => "grip",
            Icon::Ellipsis => "ellipsis",
        }
    }
}

/// Draw an icon inside `rect`.
pub fn draw(painter: &Painter, icon: Icon, rect: Rect, color: Color32, width: f32) {
    let stroke = Stroke::new(width, color);
    let center = rect.center();
    let size = rect.height().min(rect.width());
    let half = size / 2.0;

    match icon {
        Icon::Plus => {
            painter.line_segment(
                [
                    center + Vec2::new(-half, 0.0),
                    center + Vec2::new(half, 0.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(0.0, -half),
                    center + Vec2::new(0.0, half),
                ],
                stroke,
            );
        }
        Icon::Trash => {
            let body = Rect::from_center_size(
                center + Vec2::new(0.0, size * 0.12),
                vec2(size * 0.6, size * 0.62),
            );
            painter.rect_stroke(body, CornerRadius::same(2), stroke, StrokeKind::Middle);
            painter.line_segment(
                [
                    center + Vec2::new(-size * 0.4, -size * 0.34),
                    center + Vec2::new(size * 0.4, -size * 0.34),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(-size * 0.18, -size * 0.34),
                    center + Vec2::new(-size * 0.18, -size * 0.48),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(size * 0.18, -size * 0.34),
                    center + Vec2::new(size * 0.18, -size * 0.48),
                ],
                stroke,
            );
        }
        Icon::Pencil => {
            painter.line_segment(
                [
                    center + Vec2::new(-half * 0.8, half * 0.8),
                    center + Vec2::new(half * 0.7, -half * 0.9),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(half * 0.45, -half * 0.65),
                    center + Vec2::new(half * 0.8, -half * 0.3),
                ],
                stroke,
            );
        }
        Icon::Plug => {
            // Two prongs and a body: the classic plug.
            painter.rect_stroke(
                Rect::from_center_size(
                    center + Vec2::new(0.0, size * 0.18),
                    vec2(size * 0.5, size * 0.45),
                ),
                CornerRadius::same(2),
                stroke,
                StrokeKind::Middle,
            );
            painter.line_segment(
                [
                    center + Vec2::new(-size * 0.15, -size * 0.05),
                    center + Vec2::new(-size * 0.15, -half),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(size * 0.15, -size * 0.05),
                    center + Vec2::new(size * 0.15, -half),
                ],
                stroke,
            );
        }
        Icon::Power => {
            // A circle with a gap at the top and a vertical bar.
            let radius = half * 0.8;
            let mut points = Vec::new();
            for step in 0..=24 {
                let angle = (step as f32 / 24.0) * std::f32::consts::TAU;
                let (sin, cos) = angle.sin_cos();
                // Leave a gap around the top.
                if cos > 0.82 {
                    continue;
                }
                points.push(center + Vec2::new(cos, sin) * radius);
            }
            if points.len() > 1 {
                painter.add(Shape::line(points, stroke));
            }
            painter.line_segment(
                [
                    center + Vec2::new(0.0, -half),
                    center + Vec2::new(0.0, -size * 0.05),
                ],
                stroke,
            );
        }
        Icon::Pause => {
            let bar = vec2(size * 0.16, size * 0.7);
            painter.rect_filled(
                Rect::from_center_size(center + Vec2::new(-size * 0.16, 0.0), bar),
                CornerRadius::same(1),
                color,
            );
            painter.rect_filled(
                Rect::from_center_size(center + Vec2::new(size * 0.16, 0.0), bar),
                CornerRadius::same(1),
                color,
            );
        }
        Icon::Play => {
            painter.add(Shape::convex_polygon(
                vec![
                    center + Vec2::new(-size * 0.28, -size * 0.36),
                    center + Vec2::new(size * 0.34, 0.0),
                    center + Vec2::new(-size * 0.28, size * 0.36),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Download => {
            painter.line_segment(
                [
                    center + Vec2::new(0.0, -half),
                    center + Vec2::new(0.0, half * 0.4),
                ],
                stroke,
            );
            painter.add(Shape::convex_polygon(
                vec![
                    center + Vec2::new(-size * 0.26, size * 0.1),
                    center + Vec2::new(size * 0.26, size * 0.1),
                    center + Vec2::new(0.0, half * 0.7),
                ],
                color,
                Stroke::NONE,
            ));
            painter.line_segment(
                [
                    center + Vec2::new(-size * 0.34, half),
                    center + Vec2::new(size * 0.34, half),
                ],
                stroke,
            );
        }
        Icon::Upload => {
            painter.line_segment(
                [
                    center + Vec2::new(0.0, half),
                    center + Vec2::new(0.0, -half * 0.4),
                ],
                stroke,
            );
            painter.add(Shape::convex_polygon(
                vec![
                    center + Vec2::new(-size * 0.26, -size * 0.1),
                    center + Vec2::new(size * 0.26, -size * 0.1),
                    center + Vec2::new(0.0, -half * 0.7),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Search => {
            let radius = half * 0.55;
            painter.circle_stroke(
                center + Vec2::new(-size * 0.08, -size * 0.08),
                radius,
                stroke,
            );
            painter.line_segment(
                [
                    center + Vec2::new(size * 0.18, size * 0.18),
                    center + Vec2::new(half * 0.9, half * 0.9),
                ],
                stroke,
            );
        }
        Icon::Gear => {
            let radius = half * 0.62;
            painter.circle_stroke(center, radius, stroke);
            painter.circle_stroke(center, radius * 0.4, stroke);
            for step in 0..8 {
                let angle = (step as f32 / 8.0) * std::f32::consts::TAU;
                let (sin, cos) = angle.sin_cos();
                painter.line_segment(
                    [
                        center + Vec2::new(cos, sin) * radius * 0.8,
                        center + Vec2::new(cos, sin) * half,
                    ],
                    stroke,
                );
            }
        }
        Icon::Sun => {
            let radius = half * 0.4;
            painter.circle_stroke(center, radius, stroke);
            for step in 0..8 {
                let angle = (step as f32 / 8.0) * std::f32::consts::TAU;
                let (sin, cos) = angle.sin_cos();
                painter.line_segment(
                    [
                        center + Vec2::new(cos, sin) * radius * 1.35,
                        center + Vec2::new(cos, sin) * half,
                    ],
                    stroke,
                );
            }
        }
        Icon::Moon => {
            // A crescent: the outer arc of a disc plus the inner arc of a
            // smaller disc offset to the right.
            let radius = half * 0.85;
            let mut points = Vec::new();
            for step in 0..=16 {
                let angle = (step as f32 / 16.0) * std::f32::consts::PI * 0.75 - 0.4;
                let (sin, cos) = angle.sin_cos();
                points.push(center + Vec2::new(cos, sin) * radius);
            }
            for step in (0..=16).rev() {
                let angle = (step as f32 / 16.0) * std::f32::consts::PI * 0.75 - 0.4;
                let (sin, cos) = angle.sin_cos();
                points.push(
                    center + Vec2::new(cos, sin) * radius * 0.72 + Vec2::new(radius * 0.42, 0.0),
                );
            }
            painter.add(Shape::line(points, stroke));
        }
        Icon::Filter => {
            painter.add(Shape::convex_polygon(
                vec![
                    center + Vec2::new(-half * 0.9, -half * 0.6),
                    center + Vec2::new(half * 0.9, -half * 0.6),
                    center + Vec2::new(size * 0.12, size * 0.05),
                    center + Vec2::new(size * 0.12, half * 0.8),
                    center + Vec2::new(-size * 0.12, half * 0.5),
                    center + Vec2::new(-size * 0.12, size * 0.05),
                ],
                Color32::TRANSPARENT,
                stroke,
            ));
        }
        Icon::Copy => {
            let back = Rect::from_center_size(
                center + Vec2::new(size * 0.18, -size * 0.18),
                vec2(size * 0.55, size * 0.55),
            );
            let front = Rect::from_center_size(
                center + Vec2::new(-size * 0.18, size * 0.18),
                vec2(size * 0.55, size * 0.55),
            );
            painter.rect_stroke(back, CornerRadius::same(2), stroke, StrokeKind::Middle);
            painter.rect_stroke(front, CornerRadius::same(2), stroke, StrokeKind::Middle);
        }
        Icon::ChevronUp => {
            painter.add(Shape::convex_polygon(
                vec![
                    center + Vec2::new(-size * 0.3, size * 0.15),
                    center + Vec2::new(0.0, -size * 0.25),
                    center + Vec2::new(size * 0.3, size * 0.15),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::ChevronDown => {
            painter.add(Shape::convex_polygon(
                vec![
                    center + Vec2::new(-size * 0.3, -size * 0.15),
                    center + Vec2::new(size * 0.3, -size * 0.15),
                    center + Vec2::new(0.0, size * 0.25),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::ChevronRight => {
            painter.add(Shape::convex_polygon(
                vec![
                    center + Vec2::new(-size * 0.15, -size * 0.3),
                    center + Vec2::new(size * 0.25, 0.0),
                    center + Vec2::new(-size * 0.15, size * 0.3),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Menu => {
            for offset in [-0.5, 0.0, 0.5] {
                painter.line_segment(
                    [
                        center + Vec2::new(-half * 0.8, size * offset),
                        center + Vec2::new(half * 0.8, size * offset),
                    ],
                    stroke,
                );
            }
        }
        Icon::Dots => {
            for offset in [-0.5, 0.0, 0.5] {
                painter.circle_filled(center + Vec2::new(0.0, size * offset), size * 0.1, color);
            }
        }
        Icon::Eye => {
            let width = half * 1.6;
            let height = half * 0.85;
            let mut points = Vec::new();
            for step in 0..=20 {
                let t = step as f32 / 20.0;
                let x = -width / 2.0 + width * t;
                let y = height * (1.0 - (2.0 * t - 1.0).powi(2)).sqrt();
                points.push(center + Vec2::new(x, -y));
            }
            for step in (0..=20).rev() {
                let t = step as f32 / 20.0;
                let x = -width / 2.0 + width * t;
                let y = height * (1.0 - (2.0 * t - 1.0).powi(2)).sqrt();
                points.push(center + Vec2::new(x, y));
            }
            painter.add(Shape::convex_polygon(points, Color32::TRANSPARENT, stroke));
            painter.circle_filled(center, size * 0.16, color);
        }
        Icon::EyeOff => {
            painter.line_segment(
                [
                    center + Vec2::new(-half * 0.8, half * 0.5),
                    center + Vec2::new(half * 0.8, -half * 0.5),
                ],
                stroke,
            );
        }
        Icon::Topic => {
            // A topic is a path: three dots joined by lines.
            let points = [
                center + Vec2::new(-half * 0.8, half * 0.5),
                center + Vec2::new(-half * 0.2, -half * 0.3),
                center + Vec2::new(half * 0.3, half * 0.2),
                center + Vec2::new(half * 0.85, -half * 0.5),
            ];
            painter.add(Shape::line(points.to_vec(), stroke));
            for point in points {
                painter.circle_filled(point, size * 0.11, color);
            }
        }
        Icon::Connection => {
            painter.circle_stroke(center + Vec2::new(-half * 0.45, 0.0), half * 0.5, stroke);
            painter.circle_stroke(center + Vec2::new(half * 0.45, 0.0), half * 0.5, stroke);
            painter.line_segment(
                [
                    center + Vec2::new(-half * 0.05, 0.0),
                    center + Vec2::new(half * 0.05, 0.0),
                ],
                stroke,
            );
        }
        Icon::Layers => {
            for offset in [-0.25, 0.0, 0.25] {
                let y = size * offset;
                painter.add(Shape::convex_polygon(
                    vec![
                        center + Vec2::new(0.0, y - size * 0.18),
                        center + Vec2::new(size * 0.8, y),
                        center + Vec2::new(0.0, y + size * 0.18),
                        center + Vec2::new(-size * 0.8, y),
                    ],
                    Color32::TRANSPARENT,
                    stroke,
                ));
            }
        }
        Icon::Send => {
            painter.add(Shape::convex_polygon(
                vec![
                    center + Vec2::new(-half * 0.9, -half * 0.9),
                    center + Vec2::new(half * 0.9, 0.0),
                    center + Vec2::new(-half * 0.9, half * 0.9),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Grip => {
            // Six dots in two columns: the drag handle of a dock panel.
            for column in [-1.0, 1.0] {
                for row in [-1.0, 0.0, 1.0] {
                    painter.circle_filled(
                        center + Vec2::new(column * size * 0.16, row * size * 0.24),
                        size * 0.08,
                        color,
                    );
                }
            }
        }
        Icon::Ellipsis => {
            for offset in [-0.4, 0.0, 0.4] {
                painter.circle_filled(center + Vec2::new(size * offset, 0.0), size * 0.09, color);
            }
        }
    }
}

/// A small button that shows an icon.
pub fn icon_button(ui: &mut egui::Ui, icon: Icon, tooltip: &str) -> bool {
    let size = egui::vec2(20.0, 20.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let color = if response.hovered() {
            ui.visuals().text_color()
        } else {
            ui.visuals().weak_text_color()
        };
        draw(ui.painter(), icon, rect, color, 1.4);
    }
    response.on_hover_text(tooltip).clicked()
}

/// A toggle button that shows an icon.
pub fn icon_toggle(ui: &mut egui::Ui, icon: Icon, active: bool, tooltip: &str) -> bool {
    let size = egui::vec2(20.0, 20.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let color = if active {
            ui.visuals().selection.bg_fill.gamma_multiply(1.8)
        } else if response.hovered() {
            ui.visuals().text_color()
        } else {
            ui.visuals().weak_text_color()
        };
        draw(ui.painter(), icon, rect, color, 1.4);
    }
    response.on_hover_text(tooltip).clicked()
}
