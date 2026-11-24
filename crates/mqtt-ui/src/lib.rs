//! The egui user interface of MQTT Assistant.
//!
//! The UI crate only depends on the domain model: it never talks to the MQTT
//! engine itself. Everything the user does is expressed as an [`UiActions`]
//! value that the application layer translates into engine commands.

pub mod dialogs;
pub mod i18n;
pub mod panels;
pub mod state;
pub mod theme;
pub mod widgets;

pub use i18n::Language;
pub use state::{
    AppCommand, ConnectionView, CopyKind, CopyRequest, ExportFormat, PublishPanelState, RibbonTab,
    Stats, UiActions, UiState,
};
pub use theme::{Palette, Theme};
