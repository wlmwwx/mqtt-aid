//! The centre panel: the tabs and what they show.

use egui::{Rect, RichText, Ui};

use crate::i18n;
use crate::state::{CenterTab, UiState};
use crate::widgets::tabs::tab_button;

/// The tabs of the centre panel.
///
/// Returns where every tab was drawn, so that a test can click one.
pub fn center_tabs(ui: &mut Ui, state: &mut UiState) -> Vec<(CenterTab, Rect)> {
    let palette = state.theme.palette();
    let mut placed = Vec::new();

    ui.horizontal(|ui| {
        for tab in CenterTab::all() {
            let active = state.center_tab == *tab;
            let label = i18n::t(tab.label());
            let text = if active {
                RichText::new(label).color(palette.accent_text).strong()
            } else {
                RichText::new(label).color(palette.text)
            };

            let response = tab_button(ui, text, active, palette.tab_active);
            if response.clicked() {
                state.center_tab = *tab;
            }
            placed.push((*tab, response.rect));
        }
    });

    placed
}
