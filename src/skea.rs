use serde::{Deserialize, Serialize};

use crate::model::Scene;

/// A `.skea` file is the `Scene` struct plus a version header. Inskea has not
/// shipped a public release yet, so only files written by the current format
/// load; older internal formats are rejected with a version error instead of
/// being migrated.
pub const FORMAT_VERSION: u32 = 6;

#[derive(Serialize, Deserialize)]
struct SkeaFile {
    format_version: u32,
    scene: Scene,
}

pub fn save_to_string(scene: &Scene) -> Result<String, String> {
    let file = SkeaFile {
        format_version: FORMAT_VERSION,
        scene: scene.clone(),
    };
    serde_json::to_string_pretty(&file).map_err(|e| format!("failed to serialize the drawing: {e}"))
}

pub fn load_from_str(input: &str) -> Result<Scene, String> {
    let raw: serde_json::Value =
        serde_json::from_str(input).map_err(|e| format!("failed to parse .skea file: {e}"))?;

    let version = raw
        .get("format_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    if version != FORMAT_VERSION {
        return Err(format!(
            "unsupported format version: {version} (expected {FORMAT_VERSION})"
        ));
    }

    let file: SkeaFile =
        serde_json::from_value(raw).map_err(|e| format!("failed to parse .skea file: {e}"))?;
    let mut scene = file.scene;
    scene.normalize_groups();
    Ok(scene)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::elements::path::CurveMode;
    use crate::model::elements::text::WrappedText;
    use crate::model::{
        Color, Element, ElementData, Ellipse, Freehand, Line, LineStyle, Point, Rectangle, Text,
        TextAlign,
    };

    fn make_scene() -> Scene {
        let mut s = Scene::new();
        let mut rd = ElementData::new(0);
        rd.world_point.set(10.0, 20.0);
        rd.width = 100.0;
        rd.height = 50.0;
        rd.style.stroke_color = Color::new(Color::BLUE);
        rd.style.fill_color = Some(Color::new(Color::YELLOW));
        s.add_element(Element::Rectangle(Rectangle { data: rd }));

        let mut ed = ElementData::new(0);
        ed.world_point.set(5.0, 5.0);
        ed.width = 60.0;
        ed.height = 60.0;
        ed.style.stroke_color = Color::new(Color::RED);
        s.add_element(Element::Ellipse(Ellipse { data: ed }));

        let mut ld = ElementData::new(0);
        ld.style.stroke_color = Color::new(Color::GREEN);
        ld.style.stroke_width = 3.0;
        s.add_element(Element::Line(Line {
            data: ld,
            points: vec![Point { x: 0.0, y: 0.0 }, Point { x: 100.0, y: 100.0 }],
            line_style: LineStyle {
                curve_mode: CurveMode::Straight,
                has_start_arrowhead: false,
                has_end_arrowhead: false,
            },
        }));

        let mut ad = ElementData::new(0);
        ad.style.stroke_color = Color::new(Color::RED);
        s.add_element(Element::Line(Line {
            data: ad,
            points: vec![Point { x: 10.0, y: 10.0 }, Point { x: 200.0, y: 50.0 }],
            line_style: LineStyle {
                curve_mode: CurveMode::Straight,
                has_start_arrowhead: false,
                has_end_arrowhead: true,
            },
        }));

        let mut td = ElementData::new(0);
        td.world_point.set(30.0, 40.0);
        td.style.fill_color = Some(Color::new(Color::WHITE));
        s.add_element(Element::Text(Text {
            data: td,
            wrapped: WrappedText::new("hello world"),
            align: crate::model::TextAlign::Left,
        }));

        let mut fd = ElementData::new(0);
        fd.style.stroke_color = Color::new(Color::GREEN);
        fd.style.stroke_width = 1.5;
        s.add_element(Element::Freehand(Freehand {
            data: fd,
            points: vec![
                Point { x: 1.0, y: 2.0 },
                Point { x: 3.0, y: 4.0 },
                Point { x: 5.0, y: 6.0 },
            ],
        }));

        s
    }

    #[test]
    fn round_trip_all_types() {
        let scene = make_scene();
        let saved = save_to_string(&scene).unwrap();
        let loaded = load_from_str(&saved).unwrap();
        assert_eq!(scene, loaded);
    }

    #[test]
    fn round_trip_text_with_newlines() {
        let mut s = Scene::new();
        let mut td = ElementData::new(0);
        td.world_point.set(10.0, 10.0);
        td.style.fill_color = Some(Color::new(Color::WHITE));
        s.add_element(Element::Text(Text {
            data: td,
            wrapped: WrappedText::new("line one\nline two\nline three"),
            align: crate::model::TextAlign::Left,
        }));
        let saved = save_to_string(&s).unwrap();
        let loaded = load_from_str(&saved).unwrap();
        assert_eq!(s, loaded);
    }

    #[test]
    fn round_trip_rotation_and_font_size() {
        let mut s = Scene::new();
        let mut rd = ElementData::new(0);
        rd.world_point.set(10.0, 10.0);
        rd.width = 100.0;
        rd.height = 50.0;
        rd.style.stroke_color = Color::new(Color::BLUE);
        rd.rotation = std::f64::consts::FRAC_PI_4; // ~45 degrees
        s.add_element(Element::Rectangle(Rectangle { data: rd }));

        let mut td = ElementData::new(0);
        td.world_point.set(50.0, 50.0);
        td.style.fill_color = Some(Color::new(Color::WHITE));
        td.style.font_size = 36.0;
        s.add_element(Element::Text(Text {
            data: td,
            wrapped: WrappedText::new("big text"),
            align: crate::model::TextAlign::Left,
        }));

        let saved = save_to_string(&s).unwrap();
        let loaded = load_from_str(&saved).unwrap();
        assert_eq!(s, loaded);
    }

    #[test]
    fn round_trip_text_alignment_and_old_scenes_default_to_left() {
        let mut center_scene = Scene::new();
        let mut td = ElementData::new(0);
        td.world_point.set(10.0, 10.0);
        td.style.fill_color = Some(Color::new(Color::WHITE));
        center_scene.add_element(Element::Text(Text {
            data: td,
            wrapped: WrappedText::new("aligned"),
            align: TextAlign::Center,
        }));
        let saved = save_to_string(&center_scene).unwrap();
        let loaded = load_from_str(&saved).unwrap();
        assert_eq!(center_scene, loaded);
        assert_eq!(save_to_string(&loaded).unwrap(), saved);

        // A scene saved before alignment existed has no `align` field and
        // loads with the Left default; re-saving restores the field.
        let mut left_scene = Scene::new();
        let ld = ElementData::new(0);
        left_scene.add_element(Element::Text(Text {
            data: ld,
            wrapped: WrappedText::new("aligned"),
            align: TextAlign::Left,
        }));
        let with_field = save_to_string(&left_scene).unwrap();
        let legacy = with_field.replace(r#","align":"left""#, "");
        let loaded = load_from_str(&legacy).unwrap();
        assert_eq!(left_scene, loaded);
        assert_eq!(save_to_string(&loaded).unwrap(), with_field);
        let Element::Text(t) = &loaded.elements()[0] else {
            panic!("expected text element");
        };
        assert_eq!(t.align, TextAlign::Left);
    }

    #[test]
    fn legacy_groups_receive_ids_when_loaded() {
        let mut scene = Scene::new();
        for _ in 0..2 {
            scene.add_element(Element::Rectangle(Rectangle {
                data: ElementData::new(0),
            }));
        }
        scene.create_group(&[1, 2]);

        let mut raw: serde_json::Value =
            serde_json::from_str(&save_to_string(&scene).unwrap()).unwrap();
        raw.pointer_mut("/scene/groups/0")
            .and_then(serde_json::Value::as_object_mut)
            .unwrap()
            .remove("id");
        raw.pointer_mut("/scene")
            .and_then(serde_json::Value::as_object_mut)
            .unwrap()
            .remove("next_group_id");

        let loaded = load_from_str(&serde_json::to_string(&raw).unwrap()).unwrap();
        assert_eq!(loaded.groups[0].id, 1);
        assert_eq!(loaded.next_group_id, 2);
    }

    #[test]
    fn malformed_input_returns_err() {
        let err = load_from_str("this is not json").unwrap_err();
        assert!(err.contains("failed to parse"));
    }

    #[test]
    fn unsupported_version_returns_err() {
        let input = r#"{"format_version":99,"scene":{"elements":[],"next_id":1}}"#;
        let err = load_from_str(input).unwrap_err();
        assert!(err.contains("unsupported format version"));
    }

    #[test]
    fn missing_version_header() {
        let input = r#"{"scene":{"elements":[],"next_id":1}}"#;
        let err = load_from_str(input).unwrap_err();
        assert!(err.contains("unsupported format version"));
    }
}
