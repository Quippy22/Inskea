//! Short tips shown under the wordmark on the opening screen.
//!
//! Add, remove, or reword entries here. The overlay picks a new one each time
//! a fresh document opens.

pub(crate) const TIPS: &[&str] = &[
    "Press A to pan, then drag the canvas.",
    "Shift-click adds shapes to your selection.",
    "Double-click a text element to edit it.",
    "Ctrl+G groups the current selection.",
    "Ctrl+Shift+G ungroups the selection.",
    "Ctrl+D duplicates the selection.",
    "Press 1-5 to pick a shape tool.",
    "Press F for the freehand tool.",
    "Shift-drag shows smart guides.",
    "Alt-drag snaps to the grid.",
    "Ctrl+Alt+Arrow aligns the selection.",
    "An infinite canvas for your ideas.",
];

/// Pick a random starting index for a session.
pub(crate) fn pick_index() -> usize {
    random_index(TIPS.len())
}

/// The tip shown for a document generation. Advancing the generation moves
/// to the next tip, so each fresh document gets a different line.
pub(crate) fn tip_for(generation: u32, start: usize) -> &'static str {
    TIPS[(start + generation as usize) % TIPS.len()]
}

fn random_index(len: usize) -> usize {
    #[cfg(target_arch = "wasm32")]
    let value = js_sys::Math::random();
    #[cfg(not(target_arch = "wasm32"))]
    let value = 0.5;
    ((value * len as f64) as usize).min(len - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successive_generations_advance_the_tip() {
        let start = pick_index();
        let first = tip_for(0, start);
        let second = tip_for(1, start);
        assert_ne!(first, second);
    }

    #[test]
    fn tips_have_no_blank_entries() {
        assert!(!TIPS.is_empty());
        assert!(TIPS.iter().all(|tip| !tip.trim().is_empty()));
    }
}
