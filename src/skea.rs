use serde::{Deserialize, Serialize};

use crate::model::Scene;

pub const FORMAT_VERSION: u32 = 4;

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
                    // Convert flat data.{x,y} to data.world_point.{x,y}
                    if let Some(data) = el.get_mut("data") {
                        if let Some(data_obj) = data.as_object_mut() {
                            let has_x = data_obj.contains_key("x");
                            let has_y = data_obj.contains_key("y");
                            if has_x || has_y {
                                let old_x = data_obj.remove("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                let old_y = data_obj.remove("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                data_obj.insert(
                                    "world_point".to_string(),
                                    serde_json::json!({"x": old_x, "y": old_y}),
                                );
                            }
                        }
                    }

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

    let version = raw
        .get("format_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    if version == 2 {
        // Migrate v2 → v3: convert Arrow to Line with has_arrowhead
        if let Some(elements) = raw.pointer_mut("/scene/elements") {
            if let Some(arr) = elements.as_array_mut() {
                for el in arr.iter_mut() {
                    let type_name = el.get("type").and_then(|v| v.as_str());
                    if type_name == Some("Arrow") {
                        if let Some(obj) = el.as_object_mut() {
                            obj.insert("type".to_string(), serde_json::json!("Line"));
                            obj.insert("has_arrowhead".to_string(), serde_json::json!(true));
                        }
                    }
                }
            }
        }
        if let Some(obj) = raw.as_object_mut() {
            obj.insert("format_version".to_string(), serde_json::json!(3));
        }
    }

    let version = raw
        .get("format_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    if version == 3 {
        // Migrate v3 → v4: wrap flat style fields into ElementStyle/LineStyle
        if let Some(elements) = raw.pointer_mut("/scene/elements") {
            if let Some(arr) = elements.as_array_mut() {
                for el in arr.iter_mut() {
                    // data.* → data.style.*
                    if let Some(data) = el.get_mut("data") {
                        if let Some(data_obj) = data.as_object_mut() {
                            let style_fields =
                                ["stroke_color", "fill_color", "stroke_width", "font_size", "stroke_style", "edge_style"];
                            let has_any_style = style_fields.iter().any(|f| data_obj.contains_key(*f));
                            if has_any_style {
                                let mut style = serde_json::Map::new();
                                for field in &style_fields {
                                    if let Some(val) = data_obj.remove(*field) {
                                        style.insert(field.to_string(), val);
                                    }
                                }
                                data_obj.insert("style".to_string(), serde_json::Value::Object(style));
                            }
                        }
                    }
                    // Line.{curve_mode,has_arrowhead} → Line.line_style
                    let type_name = el.get("type").and_then(|v| v.as_str());
                    if type_name == Some("Line") {
                        if let Some(obj) = el.as_object_mut() {
                            if obj.contains_key("curve_mode") || obj.contains_key("has_arrowhead") {
                                let mut line_style = serde_json::Map::new();
                                if let Some(curve) = obj.remove("curve_mode") {
                                    line_style.insert("curve_mode".to_string(), curve);
                                }
                                if let Some(arrow) = obj.remove("has_arrowhead") {
                                    line_style.insert("has_arrowhead".to_string(), arrow);
                                }
                                obj.insert("line_style".to_string(), serde_json::Value::Object(line_style));
                            }
                        }
                    }
                }
            }
        }
        if let Some(obj) = raw.as_object_mut() {
            obj.insert("format_version".to_string(), serde_json::json!(4));
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
        Element, ElementData, Ellipse, Freehand, Line, LineStyle, Point, Rectangle, ShapeColor,
        Text,
    };

    fn make_scene() -> Scene {
        let mut s = Scene::new();
        let mut rd = ElementData::new(0);
        rd.world_point.set(10.0, 20.0);
        rd.width = 100.0;
        rd.height = 50.0;
        rd.style.stroke_color = ShapeColor::Blue;
        rd.style.fill_color = Some(ShapeColor::Cyan);
        s.add_element(Element::Rectangle(Rectangle { data: rd }));

        let mut ed = ElementData::new(0);
        ed.world_point.set(5.0, 5.0);
        ed.width = 60.0;
        ed.height = 60.0;
        ed.style.stroke_color = ShapeColor::Red;
        s.add_element(Element::Ellipse(Ellipse { data: ed }));

        let mut ld = ElementData::new(0);
        ld.style.stroke_color = ShapeColor::Green;
        ld.style.stroke_width = 3.0;
        s.add_element(Element::Line(Line {
            data: ld,
            points: vec![Point { x: 0.0, y: 0.0 }, Point { x: 100.0, y: 100.0 }],
            line_style: LineStyle {
                curve_mode: CurveMode::Straight,
                has_arrowhead: false,
            },
        }));

        let mut ad = ElementData::new(0);
        ad.style.stroke_color = ShapeColor::Orange;
        s.add_element(Element::Line(Line {
            data: ad,
            points: vec![Point { x: 10.0, y: 10.0 }, Point { x: 200.0, y: 50.0 }],
            line_style: LineStyle {
                curve_mode: CurveMode::Straight,
                has_arrowhead: true,
            },
        }));

        let mut td = ElementData::new(0);
        td.world_point.set(30.0, 40.0);
        td.style.fill_color = Some(ShapeColor::White);
        s.add_element(Element::Text(Text {
            data: td,
            wrapped: WrappedText::new("hello world", 0.0, 24.0),
        }));

        let mut fd = ElementData::new(0);
        fd.style.stroke_color = ShapeColor::Purple;
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
        let saved = save_to_string(&scene);
        let loaded = load_from_str(&saved).unwrap();
        assert_eq!(scene, loaded);
    }

    #[test]
    fn round_trip_text_with_newlines() {
        let mut s = Scene::new();
        let mut td = ElementData::new(0);
        td.world_point.set(10.0, 10.0);
        td.style.fill_color = Some(ShapeColor::White);
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
        rd.world_point.set(10.0, 10.0);
        rd.width = 100.0;
        rd.height = 50.0;
        rd.style.stroke_color = ShapeColor::Blue;
        rd.rotation = 0.785398; // ~45 degrees
        s.add_element(Element::Rectangle(Rectangle { data: rd }));

        let mut td = ElementData::new(0);
        td.world_point.set(50.0, 50.0);
        td.style.fill_color = Some(ShapeColor::White);
        td.style.font_size = 36.0;
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
        assert_eq!(scene.elements().len(), 3);
        // Check the Line was migrated
        if let Element::Line(line) = &scene.elements()[1] {
            assert_eq!(line.points.len(), 2);
            assert_eq!(line.points[0].x, 0.0);
            assert_eq!(line.points[0].y, 0.0);
            assert_eq!(line.points[1].x, 100.0);
            assert_eq!(line.points[1].y, 100.0);
            assert_eq!(line.line_style.curve_mode, CurveMode::Straight);
        } else {
            panic!("expected Line element at index 1");
        }
        // Check the Arrow was migrated (through v2 and v3)
        if let Element::Line(line) = &scene.elements()[2] {
            assert_eq!(line.points.len(), 2);
            assert_eq!(line.points[0].x, 10.0);
            assert_eq!(line.points[0].y, 10.0);
            assert_eq!(line.points[1].x, 200.0);
            assert_eq!(line.points[1].y, 50.0);
            assert_eq!(line.line_style.curve_mode, CurveMode::Straight);
            assert!(line.line_style.has_arrowhead);
        } else {
            panic!("expected Line (was Arrow) element at index 2");
        }
    }

    #[test]
    fn migrate_v2_arrow_to_v3() {
        let v2_input = r#"{
            "format_version": 2,
            "scene": {
                "next_id": 10,
                "elements": [
                    {"type": "Rectangle", "data": {"id": 1, "world_point": {"x": 0.0, "y": 0.0}, "width": 50.0, "height": 50.0, "rotation": 0.0, "font_size": 24.0, "stroke_color": "Blue", "fill_color": null, "stroke_width": 2.0}},
                    {"type": "Line", "data": {"id": 2, "world_point": {"x": 0.0, "y": 0.0}, "width": 0.0, "height": 0.0, "rotation": 0.0, "font_size": 24.0, "stroke_color": "Green", "fill_color": null, "stroke_width": 3.0}, "points": [{"x": 0.0, "y": 0.0}, {"x": 100.0, "y": 100.0}], "curve_mode": "Straight"},
                    {"type": "Arrow", "data": {"id": 3, "world_point": {"x": 0.0, "y": 0.0}, "width": 0.0, "height": 0.0, "rotation": 0.0, "font_size": 24.0, "stroke_color": "Orange", "fill_color": null, "stroke_width": 2.0}, "points": [{"x": 10.0, "y": 10.0}, {"x": 200.0, "y": 50.0}], "curve_mode": "Straight"}
                ]
            }
        }"#;
        let scene = load_from_str(v2_input).expect("v2→v3 migration should succeed");
        assert_eq!(scene.elements().len(), 3);
        // Line unchanged
        if let Element::Line(line) = &scene.elements()[1] {
            assert!(!line.line_style.has_arrowhead);
        } else {
            panic!("expected Line element at index 1");
        }
        // Arrow converted to Line
        if let Element::Line(line) = &scene.elements()[2] {
            assert_eq!(line.points[0].x, 10.0);
            assert_eq!(line.points[0].y, 10.0);
            assert_eq!(line.points[1].x, 200.0);
            assert_eq!(line.points[1].y, 50.0);
            assert!(line.line_style.has_arrowhead);
        } else {
            panic!("expected Line (was Arrow) element at index 2");
        }
    }
}
