// ── Tailwind class constants ──────────────────────────────────────────────
//
// Convention:
//   PANEL_*        Floating panels (frosted glass, border, shadow)
//   BTN_*          Button base classes (use _ACTIVE / _INACTIVE suffix for states)
//   CONTAINER_*    Outer layout wrappers (fixed positioning, etc.)
//   SEP_*          Separator / divider
//   MENU_*         Menu dropdown
// ──────────────────────────────────────────────────────────────────────────

/// Frosted-glass floating panel.
pub const PANEL: &str =
    "rounded-lg bg-panel/80 backdrop-blur-sm border border-border shadow-lg pointer-events-auto";

/// Toolbar inner container (horizontal button row with glow).
pub const TBAR_INNER: &str = "flex items-center gap-0.5 rounded-lg bg-panel/80 backdrop-blur-sm border border-border shadow-[0_6px_12px_-4px_rgba(122,162,247,0.35)] pointer-events-auto p-0.5";

/// Dock wrapper: fixed left-centre column.
pub const CONTAINER_DOCK: &str =
    "fixed left-4 max-sm:left-2 top-1/2 -translate-y-1/2 z-40 flex flex-col items-center gap-0.5";

/// Status bar wrapper: fixed top-centre row.
pub const CONTAINER_STATUSBAR: &str =
    "fixed max-sm:top-2 top-4 inset-x-0 flex justify-center pointer-events-none z-50";

/// Active state for a dock category / eraser button.
pub const BTN_CAT_ACTIVE: &str = "flex items-center justify-center h-9 w-9 transition-colors text-accent bg-accent/10 border-l-2 border-accent";

/// Inactive state for a dock category / eraser button.
pub const BTN_CAT_INACTIVE: &str = "flex items-center justify-center h-9 w-9 transition-colors text-subtle hover:text-fg hover:bg-surface/50 border-l-2 border-transparent";

/// Active drawing-tool button in the expanded panel.
pub const BTN_TOOL_ACTIVE: &str = "flex items-center justify-center h-9 w-9 rounded-md transition-colors text-accent bg-accent/10";

/// Inactive drawing-tool button.
pub const BTN_TOOL_INACTIVE: &str = "flex items-center justify-center h-9 w-9 rounded-md transition-colors text-subtle hover:text-fg hover:bg-surface/50";

/// Active toolbar button (hand / select / draw).
pub const BTN_TBAR_ACTIVE: &str = "flex items-center justify-center h-8 w-8 rounded-md transition-colors text-accent bg-accent/10";

/// Inactive toolbar button.
pub const BTN_TBAR_INACTIVE: &str = "flex items-center justify-center h-8 w-8 rounded-md transition-colors text-subtle hover:text-fg hover:bg-surface/50";

/// Ghost toolbar button (home, undo, redo, menu — no toggling).
pub const BTN_GHOST: &str = "flex items-center justify-center h-8 w-8 rounded-md text-subtle hover:text-fg hover:bg-surface/50 transition-colors";

/// Collapse / expand button inside the dock.
pub const BTN_COLLAPSE: &str = "flex items-center justify-center h-9 w-9 text-subtle hover:text-fg hover:bg-surface/50 transition-colors";

/// Colour swatch with selection ring.
pub const BTN_SWATCH_SEL: &str = "w-7 h-7 rounded-md border transition-transform hover:scale-110 border-accent ring-2 ring-accent/50";

/// Colour swatch (no selection).
pub const BTN_SWATCH_OFF: &str =
    "w-7 h-7 rounded-md border transition-transform hover:scale-110 border-border";

/// Separator line (vertical).
pub const SEP_V: &str = "w-px h-5 bg-border mx-1";

/// Menu dropdown container (frosted panel anchored below the menu button).
pub const MENU_DROPDOWN: &str = "absolute top-full right-0 mt-1 z-50 min-w-[160px] rounded-lg bg-panel/95 backdrop-blur-sm border border-border shadow-xl py-1 pointer-events-auto";

/// Menu dropdown item.
pub const MENU_ITEM: &str = "flex items-center justify-between w-full text-left px-4 py-1.5 text-sm text-fg hover:bg-accent/10 transition-colors";

/// Settings panel window (dropdown anchored below gear button).
pub const SETTINGS_WINDOW: &str = "absolute top-full right-0 mt-2 z-50 w-64 rounded-lg bg-panel/95 backdrop-blur-sm border border-border shadow-xl py-3 px-4 pointer-events-auto";

/// Settings section label.
pub const SETTINGS_LABEL: &str = "text-xs text-subtle shrink-0";

/// Segmented control button — active state.
pub const SEG_BTN_ACTIVE: &str = "px-2.5 py-1 text-xs transition-colors bg-accent/10 text-accent";

/// Segmented control button — inactive state.
pub const SEG_BTN_INACTIVE: &str = "px-2.5 py-1 text-xs transition-colors text-subtle hover:text-fg hover:bg-surface/50";
