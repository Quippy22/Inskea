pub mod components;
pub(crate) mod cursor;
pub mod dock;
pub(crate) mod export;
pub(crate) mod file_ops;
pub(crate) mod icon;
pub(crate) mod image_import;
pub(crate) mod notify;
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

pub use settings::SettingsPanel;
pub use status::CanvasStatus;
pub use theme_picker::ThemePicker;
pub use toolbar::ToolBar;
pub use unsaved_dialog::UnsavedChangesDialog;
pub use welcome::WelcomeOverlay;
