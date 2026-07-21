pub mod components;
pub mod dock;
pub(crate) mod export;
pub(crate) mod file_ops;
pub(crate) mod icon;
pub(crate) mod settings;
/// UI overlay components: toolbar, dock, settings, icons, Tailwind class constants,
/// and reusable UI building blocks.
pub(crate) mod styles;
mod toolbar;

pub use settings::SettingsPanel;
pub use toolbar::ToolBar;
