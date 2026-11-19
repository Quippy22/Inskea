mod catalogue;
mod chart_editor;
pub(crate) mod chart_import;
pub mod components;
pub(crate) mod cursor;
pub mod dock;
pub(crate) mod export;
pub(crate) mod file_ops;
pub(crate) mod frames;
pub(crate) mod icon;
pub(crate) mod image_import;
pub(crate) mod notify;
pub mod panel_zone;
pub(crate) mod settings;
mod status;
/// UI overlay components: toolbar, dock, settings, icons, Tailwind class constants,
/// and reusable UI building blocks.
pub(crate) mod styles;
mod theme_picker;
mod toolbar;
pub(crate) mod unsaved_dialog;
mod welcome;
mod welcome_tips;
mod whats_new;

pub use catalogue::{
    CatalogueContext, CatalogueContextMenu, CatalogueTab, SaveSnippetDialog, SelectionContextMenu,
};
pub use chart_editor::ChartDataEditor;
#[allow(unused_imports)]
pub use components::{LogoAnimation, OpeningScreen};
pub use frames::{FrameReaderOverlay, FramesContext, FramesContextMenu};
#[allow(unused_imports)]
pub use panel_zone::{
    animate_snap_transition, detect_snap_zone, grip_line_horizontal, grip_line_vertical,
    shapes_container_class, toolbar_container_class, tools_container_class, DraggedPanel,
    PanelSnapHighlight, PanelZone,
};
pub use settings::SettingsPanel;
pub use status::CanvasStatus;
pub use theme_picker::ThemePicker;
pub use toolbar::ToolBar;
pub use unsaved_dialog::UnsavedChangesDialog;
pub use welcome::WelcomeOverlay;
pub use whats_new::WhatsNewModal;

