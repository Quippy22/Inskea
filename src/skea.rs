use serde::{Deserialize, Serialize};

use crate::model::Scene;

pub const FORMAT_VERSION: u32 = 2;

#[derive(Serialize, Deserialize)]
struct SkeaFile {
    format_version: u32,
    scene: Scene,
}

pub fn save_to_string(scene: &Scene) -> String {
    let file = SkeaFile {
        format_version: FORMAT_VERSION,
        scene: scene.clone(),
    };
    serde_json::to_string_pretty(&file).unwrap_or_else(|_| "{}".to_string())
}

pub fn load_from_str(input: &str) -> Result<Scene, String> {
    let mut raw: serde_json::Value =
        serde_json::from_str(input).map_err(|e| format!("failed to parse .skea file: {e}"))?;

    let version = raw
        .get("format_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    if version == 1 {
        // Migrate v1 → v2: convert Line and Arrow from {a, b} to {points, curve_mode}
        if let Some(elements) = raw.pointer_mut("/scene/elements") {
            if let Some(arr) = elements.as_array_mut() {
                for el in arr.iter_mut() {
                    let type_name = el.get("type").and_then(|v| v.as_str());
                    if (type_name == Some("Line") || type_name == Some("Arrow"))
                        && el.get("points").is_none() {
                            let a = el.get("a").and_then(|v| serde_json::to_value(v).ok());
                            let b = el.get("b").and_then(|v| serde_json::to_value(v).ok());
                            if let (Some(a_val), Some(b_val)) = (a, b) {
                                if let Some(obj) = el.as_object_mut() {
                                    obj.remove("a");
                                    obj.remove("b");
                                    let points = serde_json::json!([a_val, b_val]);
                                    obj.insert("points".to_string(), points);
                                    obj.insert(
                                        "curve_mode".to_string(),
                                        serde_json::json!("Straight"),
                                    );
                                }
                            }
                        }
                }
            }
        }
        if let Some(obj) = raw.as_object_mut() {
            obj.insert("format_version".to_string(), serde_json::json!(2));
        }
    }

    let version_after = raw
        .get("format_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    if version_after != FORMAT_VERSION {
        return Err(format!(
            "unsupported format version: {} (expected {FORMAT_VERSION})",
            version_after
        ));
    }

    let file: SkeaFile = serde_json::from_value(raw)
        .map_err(|e| format!("failed to parse .skea file after migration: {e}"))?;
    Ok(file.scene)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::elements::path::CurveMode;
    use crate::model::elements::text::WrappedText;
    use crate::model::{
        Arrow, Element, ElementData, Ellipse, Freehand, Line, Point, Rectangle, ShapeColor, Text,
    };

    fn make_scene() -> Scene {
        let mut s = Scene::new();
        let mut rd = ElementData::new(0);
        rd.x = 10.0;
        rd.y = 20.0;
        rd.width = 100.0;
        rd.height = 50.0;
        rd.stroke_color = ShapeColor::Blue;
        rd.fill_color = Some(ShapeColor::Cyan);
        s.add_element(Element::Rectangle(Rectangle { data: rd }));

        let mut ed = ElementData::new(0);
        ed.x = 5.0;
        ed.y = 5.0;
        ed.width = 60.0;
        ed.height = 60.0;
        ed.stroke_color = ShapeColor::Red;
        s.add_element(Element::Ellipse(Ellipse { data: ed }));

        let mut ld = ElementData::new(0);
        ld.stroke_color = ShapeColor::Green;
        ld.stroke_width = 3.0;
        s.add_element(Element::Line(Line {
            data: ld,
            points: vec![Point { x: 0.0, y: 0.0 }, Point { x: 100.0, y: 100.0 }],
            curve_mode: CurveMode::Straight,
        }));

        let mut ad = ElementData::new(0);
        ad.stroke_color = ShapeColor::Orange;
        s.add_element(Element::Arrow(Arrow {
            data: ad,
            points: vec![Point { x: 10.0, y: 10.0 }, Point { x: 200.0, y: 50.0 }],
            curve_mode: CurveMode::Straight,
        }));

        let mut td = ElementData::new(0);
        td.x = 30.0;
        td.y = 40.0;
        td.fill_color = Some(ShapeColor::White);
        s.add_element(Element::Text(Text {
            data: td,
            wrapped: WrappedText::new("hello world", 0.0, 24.0),
        }));

        let mut fd = ElementData::new(0);
        fd.stroke_color = ShapeColor::Purple;
        fd.stroke_width = 1.5;
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
        let saved = save_to_string(&scene);
        let loaded = load_from_str(&saved).unwrap();
        assert_eq!(scene, loaded);
    }

    #[test]
    fn round_trip_text_with_newlines() {
        let mut s = Scene::new();
        let mut td = ElementData::new(0);
        td.x = 10.0;
        td.y = 10.0;
        td.fill_color = Some(ShapeColor::White);
        s.add_element(Element::Text(Text {
            data: td,
            wrapped: WrappedText::new("line one\nline two\nline three", 200.0, 24.0),
        }));
        let saved = save_to_string(&s);
        let loaded = load_from_str(&saved).unwrap();
        assert_eq!(s, loaded);
    }

    #[test]
    fn round_trip_rotation_and_font_size() {
        let mut s = Scene::new();
        let mut rd = ElementData::new(0);
        rd.x = 10.0;
        rd.y = 10.0;
        rd.width = 100.0;
        rd.height = 50.0;
        rd.stroke_color = ShapeColor::Blue;
        rd.rotation = 0.785398; // ~45 degrees
        s.add_element(Element::Rectangle(Rectangle { data: rd }));

        let mut td = ElementData::new(0);
        td.x = 50.0;
        td.y = 50.0;
        td.fill_color = Some(ShapeColor::White);
        td.font_size = 36.0;
        s.add_element(Element::Text(Text {
            data: td,
            wrapped: WrappedText::new("big text", 200.0, 36.0),
        }));

        let saved = save_to_string(&s);
        let loaded = load_from_str(&saved).unwrap();
        assert_eq!(s, loaded);
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

    #[test]
    fn migrate_v1_line_arrow_to_v2() {
        // Hand-written v1 format with a/ b fields
        let v1_input = r#"{
            "format_version": 1,
            "scene": {
                "next_id": 10,
                "elements": [
                    {"type": "Rectangle", "data": {"id": 1, "x": 0.0, "y": 0.0, "width": 50.0, "height": 50.0, "rotation": 0.0, "font_size": 24.0, "stroke_color": "Blue", "fill_color": null, "stroke_width": 2.0}},
                    {"type": "Line", "data": {"id": 2, "x": 0.0, "y": 0.0, "width": 0.0, "height": 0.0, "rotation": 0.0, "font_size": 24.0, "stroke_color": "Green", "fill_color": null, "stroke_width": 3.0}, "a": {"x": 0.0, "y": 0.0}, "b": {"x": 100.0, "y": 100.0}},
                    {"type": "Arrow", "data": {"id": 3, "x": 0.0, "y": 0.0, "width": 0.0, "height": 0.0, "rotation": 0.0, "font_size": 24.0, "stroke_color": "Orange", "fill_color": null, "stroke_width": 2.0}, "a": {"x": 10.0, "y": 10.0}, "b": {"x": 200.0, "y": 50.0}}
                ]
            }
        }"#;
        let scene = load_from_str(v1_input).expect("v1 migration should succeed");
        assert_eq!(scene.elements.len(), 3);
        // Check the Line was migrated
        if let Element::Line(line) = &scene.elements[1] {
            assert_eq!(line.points.len(), 2);
            assert_eq!(line.points[0].x, 0.0);
            assert_eq!(line.points[0].y, 0.0);
            assert_eq!(line.points[1].x, 100.0);
            assert_eq!(line.points[1].y, 100.0);
            assert_eq!(line.curve_mode, CurveMode::Straight);
        } else {
            panic!("expected Line element at index 1");
        }
        // Check the Arrow was migrated
        if let Element::Arrow(arrow) = &scene.elements[2] {
            assert_eq!(arrow.points.len(), 2);
            assert_eq!(arrow.points[0].x, 10.0);
            assert_eq!(arrow.points[0].y, 10.0);
            assert_eq!(arrow.points[1].x, 200.0);
            assert_eq!(arrow.points[1].y, 50.0);
            assert_eq!(arrow.curve_mode, CurveMode::Straight);
        } else {
            panic!("expected Arrow element at index 2");
        }
    }
}
