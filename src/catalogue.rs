//! Saved selections ("snippets") and the data format shared by the local and
//! global catalogues. Local snippets live in the document's [`Scene`]; global
//! snippets are files in the app-data `catalogue` directory (or localStorage
//! in the browser).

use serde::{Deserialize, Serialize};

use crate::edit_ops;
use crate::model::elements::Bounds;
use crate::model::{Element, ElementId, Group, Point, Rect, Scene};
use leptos::*;
use std::rc::Rc;

/// Format header for a global snippet file.
pub const FORMAT_VERSION: u32 = 1;

/// A reusable group of elements, with the group structure they carried.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snippet {
    /// Stable identity within its catalogue. Used as the global file name.
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub starred: bool,
    pub elements: Vec<Element>,
    #[serde(default)]
    pub groups: Vec<Group>,
}

impl Snippet {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        elements: Vec<Element>,
        groups: Vec<Group>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            starred: false,
            elements,
            groups,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

/// A captured selection waiting for the user to choose local, global, or cancel.
#[derive(Clone, Debug)]
pub struct PendingSnippet {
    pub elements: Vec<Element>,
    pub groups: Vec<Group>,
}

impl PendingSnippet {
    pub fn into_snippet(self, id: String, name: String) -> Snippet {
        Snippet::new(id, name, self.elements, self.groups)
    }
}

#[derive(Serialize, Deserialize)]
struct SnippetFile {
    format_version: u32,
    snippet: Snippet,
}

/// Serialize a global snippet with its format header.
pub fn to_json(snippet: &Snippet) -> Result<String, String> {
    let file = SnippetFile {
        format_version: FORMAT_VERSION,
        snippet: snippet.clone(),
    };
    serde_json::to_string_pretty(&file).map_err(|e| format!("failed to serialize the snippet: {e}"))
}

/// Parse a global snippet file.
pub fn from_json(input: &str) -> Result<Snippet, String> {
    let raw: serde_json::Value =
        serde_json::from_str(input).map_err(|e| format!("failed to parse the snippet: {e}"))?;
    let version = raw
        .get("format_version")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as u32;
    if version == 0 || version > FORMAT_VERSION {
        return Err(format!(
            "unsupported snippet format version: {version} (expected 1 to {FORMAT_VERSION})"
        ));
    }
    let file: SnippetFile =
        serde_json::from_value(raw).map_err(|e| format!("failed to parse the snippet: {e}"))?;
    Ok(file.snippet)
}

/// Capture the current selection as a pending snippet. Complete groups are
/// included even when only one member was selected.
pub fn capture_selection(scene: &Scene, ids: &[ElementId]) -> Option<PendingSnippet> {
    if ids.is_empty() {
        return None;
    }
    let ids = scene.expand_to_groups(ids);
    let elements = scene.elements_for_ids(&ids);
    if elements.is_empty() {
        return None;
    }
    let groups = scene
        .groups
        .iter()
        .filter_map(|group| {
            let members = group
                .members
                .iter()
                .filter(|member| ids.contains(member))
                .copied()
                .collect::<Vec<_>>();
            (members.len() >= 2).then_some(Group {
                id: group.id,
                members,
            })
        })
        .collect();
    Some(PendingSnippet { elements, groups })
}

/// The union bounds of a snippet, if it has any elements.
pub fn bounds(snippet: &Snippet) -> Option<Rect> {
    snippet
        .elements
        .iter()
        .map(|element| element.bounds())
        .reduce(|left, right| left.union(&right))
}

/// Insert a snippet centred on `target`, keeping its group structure.
pub fn apply(
    snippet: &Snippet,
    scene: RwSignal<Scene>,
    selected_ids: RwSignal<Vec<ElementId>>,
    push_snapshot: &Rc<dyn Fn()>,
    target: Point,
) {
    if snippet.is_empty() {
        return;
    }
    let Some(bounds) = bounds(snippet) else {
        return;
    };
    let center = bounds.center();
    let dx = target.x - center.x;
    let dy = target.y - center.y;
    let mut new_ids = Vec::new();
    edit_ops::mutate_scene(scene, push_snapshot, |scene| {
        new_ids =
            edit_ops::insert_elements(scene, snippet.elements.clone(), &snippet.groups, dx, dy);
    });
    selected_ids.set(new_ids);
}

// ── Global catalogue storage ───────────────────────────────────────────────

fn browser_globals() -> Result<Vec<Snippet>, String> {
    let Some(content) = crate::storage::load_catalogue() else {
        return Ok(Vec::new());
    };
    serde_json::from_str(&content).map_err(|e| format!("failed to read the catalogue: {e}"))
}

fn save_browser_globals(snippets: &[Snippet]) -> Result<(), String> {
    let content = serde_json::to_string(snippets)
        .map_err(|e| format!("failed to serialize the catalogue: {e}"))?;
    if crate::storage::save_catalogue(&content) {
        Ok(())
    } else {
        Err("the browser refused to store the catalogue".into())
    }
}

/// Load every global snippet: app-data files on desktop, localStorage in the
/// browser. Individual malformed files are skipped.
pub async fn load_globals() -> Result<Vec<Snippet>, String> {
    if !crate::tauri_bridge::is_tauri() {
        return browser_globals();
    }
    let files = crate::tauri_bridge::list_catalogue_snippets().await?;
    Ok(files
        .iter()
        .filter_map(|content| from_json(content).ok())
        .collect())
}

/// Store one global snippet under its id.
pub async fn save_global(snippet: &Snippet) -> Result<(), String> {
    if !crate::tauri_bridge::is_tauri() {
        let mut snippets = browser_globals()?;
        snippets.retain(|existing| existing.id != snippet.id);
        snippets.push(snippet.clone());
        return save_browser_globals(&snippets);
    }
    let content = to_json(snippet)?;
    crate::tauri_bridge::save_catalogue_snippet(&snippet.id, &content).await
}

/// Remove one global snippet.
pub async fn delete_global(id: &str) -> Result<(), String> {
    if !crate::tauri_bridge::is_tauri() {
        let mut snippets = browser_globals()?;
        snippets.retain(|snippet| snippet.id != id);
        return save_browser_globals(&snippets);
    }
    crate::tauri_bridge::delete_catalogue_snippet(id).await
}

/// A filesystem-friendly base name for a snippet.
fn slug(name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let out = out.trim_matches('-');
    if out.is_empty() {
        "snippet".to_string()
    } else {
        out.to_string()
    }
}

/// A unique id derived from `name`, avoiding existing catalogue ids.
pub fn unique_id<'a>(name: &str, existing: impl IntoIterator<Item = &'a str>) -> String {
    let base = slug(name);
    let taken: Vec<&str> = existing.into_iter().collect();
    if !taken.contains(&base.as_str()) {
        return base;
    }
    for suffix in 2.. {
        let candidate = format!("{base}-{suffix}");
        if !taken.contains(&candidate.as_str()) {
            return candidate;
        }
    }
    unreachable!("an unbounded suffix always finds a free id")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ElementData, Rectangle};

    fn rectangle(id: ElementId) -> Element {
        let mut data = ElementData::new(id);
        data.width = 40.0;
        data.height = 30.0;
        Element::Rectangle(Rectangle { data })
    }

    #[test]
    fn capture_keeps_complete_groups_and_filters_loopback_groups() {
        let mut scene = Scene::new();
        scene.add_element(rectangle(0));
        scene.add_element(rectangle(0));
        scene.add_element(rectangle(0));
        scene.create_group(&[1, 2]);

        let pending = capture_selection(&scene, &[1]).unwrap();
        assert_eq!(pending.elements.len(), 2);
        assert_eq!(pending.groups.len(), 1);
        assert_eq!(pending.groups[0].members, vec![1, 2]);

        assert!(capture_selection(&scene, &[]).is_none());
        assert!(capture_selection(&scene, &[99]).is_none());
    }

    #[test]
    fn snippet_round_trips_through_json() {
        let mut scene = Scene::new();
        scene.add_element(rectangle(0));
        scene.add_element(rectangle(0));
        scene.create_group(&[1, 2]);
        let pending = capture_selection(&scene, &[1]).unwrap();
        let snippet = pending.into_snippet("pair".into(), "Pair".into());

        let loaded = from_json(&to_json(&snippet).unwrap()).unwrap();
        assert_eq!(loaded, snippet);
    }

    #[test]
    fn ids_are_slugged_and_made_unique() {
        assert_eq!(unique_id("Hello, World!", []), "hello-world");
        assert_eq!(unique_id("  ***  ", []), "snippet");
        assert_eq!(
            unique_id("Shape", ["shape", "shape-2"]),
            "shape-3".to_string()
        );
    }

    #[test]
    fn rejects_unknown_snippet_versions() {
        let bad = r#"{"format_version":2,"snippet":{"name":"x","elements":[]}}"#;
        assert!(from_json(bad).is_err());
        assert!(from_json(r#"{"snippet":{"name":"x","elements":[]}}"#).is_err());
    }
}
