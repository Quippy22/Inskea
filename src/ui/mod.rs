pub mod components;
pub(crate) mod cursor;
pub mod dock;
pub(crate) mod export;
pub(crate) mod file_ops;
pub(crate) mod icon;
pub(crate) mod notify;
pub(crate) mod settings;
/// UI overlay components: toolbar, dock, settings, icons, Tailwind class constants,
/// and reusable UI building blocks.
pub(crate) mod styles;
mod toolbar;
pub(crate) mod unsaved_dialog;

pub use settings::SettingsPanel;
pub use toolbar::ToolBar;
pub use unsaved_dialog::UnsavedChangesDialog;
