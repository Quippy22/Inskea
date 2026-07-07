/// UI overlay components: toolbar, dock, settings, icons, and Tailwind class constants.
pub(crate) mod classes;
pub mod dock;
pub(crate) mod icon;
pub(crate) mod settings;
mod toolbar;

pub use settings::SettingsPanel;
pub use toolbar::ToolBar;
