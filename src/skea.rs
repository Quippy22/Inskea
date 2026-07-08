use serde::{Deserialize, Serialize};

use crate::model::Scene;

pub const FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct SkeaFile {
    format_version: u32,
    scene: Scene,
}

pub fn save_to_string(scene: &Scene) -> String {
    let file = SkeaFile { format_version: FORMAT_VERSION, scene: scene.clone() };
    serde_json::to_string_pretty(&file).unwrap_or_else(|_| "{}".to_string())
}

pub fn load_from_str(input: &str) -> Result<Scene, String> {
    let file: SkeaFile = serde_json::from_str(input)
        .map_err(|e| format!("failed to parse .skea file: {e}"))?;
    if file.format_version != FORMAT_VERSION {
        return Err(format!(
            "unsupported format version: {} (expected {FORMAT_VERSION})",
            file.format_version
        ));
    }
    Ok(file.scene)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Arrow, Element, ElementData, Ellipse, Freehand, Line, Point, Rectangle, ShapeColor, Text,
    };
    use crate::model::elements::text::WrappedText;

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
            a: Point { x: 0.0, y: 0.0 },
            b: Point { x: 100.0, y: 100.0 },
        }));

        let mut ad = ElementData::new(0);
        ad.stroke_color = ShapeColor::Orange;
        s.add_element(Element::Arrow(Arrow {
            data: ad,
            a: Point { x: 10.0, y: 10.0 },
            b: Point { x: 200.0, y: 50.0 },
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
        assert!(err.contains("failed to parse"));
    }
}
