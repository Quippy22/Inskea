/// UI overlay components: toolbar, dock, settings, icons, Tailwind class constants,
/// and reusable UI building blocks.
pub(crate) mod classes;
pub mod components;
pub mod dock;
pub(crate) mod export;
pub(crate) mod icon;
pub(crate) mod settings;
mod toolbar;

pub use settings::SettingsPanel;
pub use toolbar::ToolBar;
