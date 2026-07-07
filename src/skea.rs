use std::fmt::Write;

use crate::model::{
    Arrow, Element, ElementData, Ellipse, Freehand, Line, Point, Rectangle,
    Scene, ShapeColor, Text,
};

const FORMAT_VERSION: u32 = 1;

/// Serialise the scene to the .skea line-based format.
pub fn save_to_string(scene: &Scene) -> String {
    let mut out = String::new();
    _ = writeln!(out, "v {FORMAT_VERSION}");
    for el in &scene.elements {
        match el {
            Element::Rectangle(r) => {
                let d = &r.data;
                let fill = d
                    .fill_color
                    .map(|c| format!("{c}"))
                    .unwrap_or("none".into());
                _ = writeln!(
                    out,
                    "rect {} {} {} {} {} {} {fill}",
                    d.x, d.y, d.width, d.height, d.stroke_width, d.stroke_color
                );
            }
            Element::Ellipse(e) => {
                let d = &e.data;
                let fill = d
                    .fill_color
                    .map(|c| format!("{c}"))
                    .unwrap_or("none".into());
                _ = writeln!(
                    out,
                    "ellipse {} {} {} {} {} {} {fill}",
                    d.x, d.y, d.width, d.height, d.stroke_width, d.stroke_color
                );
            }
            Element::Line(l) => {
                _ = writeln!(
                    out,
                    "line {} {} {} {} {} {}",
                    l.a.x, l.a.y, l.b.x, l.b.y, l.data.stroke_width, l.data.stroke_color
                );
            }
            Element::Arrow(a) => {
                _ = writeln!(
                    out,
                    "arrow {} {} {} {} {} {}",
                    a.a.x, a.a.y, a.b.x, a.b.y, a.data.stroke_width, a.data.stroke_color
                );
            }
            Element::Text(t) => {
                let d = &t.data;
                let fill = d
                    .fill_color
                    .map(|c| format!("{c}"))
                    .unwrap_or("none".into());
                let content_len = t.content.len();
                _ = writeln!(
                    out,
                    "text {} {} {} {} {fill} {content_len}:{}",
                    d.x, d.y, d.stroke_width, d.stroke_color, t.content
                );
            }
            Element::Freehand(f) => {
                let points: String = f.points
                    .iter()
                    .map(|p| format!("{},{}", p.x, p.y))
                    .collect::<Vec<_>>()
                    .join(" ");
                _ = writeln!(
                    out,
                    "freehand {} {} {points}",
                    f.data.stroke_width, f.data.stroke_color
                );
            }
        }
    }
    out
}

/// Parse a .skea format string back into a Scene.
pub fn load_from_str(input: &str) -> Result<Scene, String> {
    let mut elements = Vec::new();
    let mut lines = input.lines().peekable();

    let header = lines.next().ok_or("empty file")?.trim();
    if !header.starts_with("v ") {
        return Err("missing format version header (expected `v 1`)".into());
    }
    let version: u32 = header[2..]
        .trim()
        .parse()
        .map_err(|_| "invalid format version".to_string())?;
    if version != 1 {
        return Err(format!("unsupported format version: {version}"));
    }

    let mut next_id: u64 = 1;

    for (lineno, line) in lines.enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let err = |msg: &str| format!("line {}: {msg}", lineno + 2);
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "rect" | "ellipse" => {
                if parts.len() < 8 {
                    return Err(err("expected: tag x y w h sw stroke fill"));
                }
                let x: f64 = parts[1].parse().map_err(|_| err("invalid x"))?;
                let y: f64 = parts[2].parse().map_err(|_| err("invalid y"))?;
                let w: f64 = parts[3].parse().map_err(|_| err("invalid width"))?;
                let h: f64 = parts[4].parse().map_err(|_| err("invalid height"))?;
                let sw: f64 = parts[5].parse().map_err(|_| err("invalid stroke_width"))?;
                let stroke =
                    ShapeColor::from_name(parts[6]).ok_or_else(|| err("invalid stroke color"))?;
                let fill = if parts[7] == "none" {
                    None
                } else {
                    Some(ShapeColor::from_name(parts[7]).ok_or_else(|| err("invalid fill color"))?)
                };
                let mut data = ElementData::new(next_id);
                data.x = x;
                data.y = y;
                data.width = w;
                data.height = h;
                data.stroke_width = sw;
                data.stroke_color = stroke;
                data.fill_color = fill;
                next_id += 1;
                elements.push(if parts[0] == "rect" {
                    Element::Rectangle(Rectangle { data })
                } else {
                    Element::Ellipse(Ellipse { data })
                });
            }
            "line" | "arrow" => {
                if parts.len() < 7 {
                    return Err(err("expected: tag x1 y1 x2 y2 sw stroke"));
                }
                let x1: f64 = parts[1].parse().map_err(|_| err("invalid x1"))?;
                let y1: f64 = parts[2].parse().map_err(|_| err("invalid y1"))?;
                let x2: f64 = parts[3].parse().map_err(|_| err("invalid x2"))?;
                let y2: f64 = parts[4].parse().map_err(|_| err("invalid y2"))?;
                let sw: f64 = parts[5].parse().map_err(|_| err("invalid stroke_width"))?;
                let stroke =
                    ShapeColor::from_name(parts[6]).ok_or_else(|| err("invalid stroke color"))?;
                let mut data = ElementData::new(next_id);
                data.stroke_width = sw;
                data.stroke_color = stroke;
                next_id += 1;
                let a = Point { x: x1, y: y1 };
                let b = Point { x: x2, y: y2 };
                elements.push(if parts[0] == "line" {
                    Element::Line(Line { data, a, b })
                } else {
                    Element::Arrow(Arrow { data, a, b })
                });
            }
            "text" => {
                if parts.len() < 6 {
                    return Err(err("expected: tag x y sw stroke fill <len>:<content>"));
                }
                let x: f64 = parts[1].parse().map_err(|_| err("invalid x"))?;
                let y: f64 = parts[2].parse().map_err(|_| err("invalid y"))?;
                let sw: f64 = parts[3].parse().map_err(|_| err("invalid stroke_width"))?;
                let stroke =
                    ShapeColor::from_name(parts[4]).ok_or_else(|| err("invalid stroke color"))?;
                let fill = if parts[5] == "none" {
                    None
                } else {
                    Some(ShapeColor::from_name(parts[5]).ok_or_else(|| err("invalid fill color"))?)
                };
                let content = if parts.len() > 6 {
                    let field_end: usize = parts[..6].iter().map(|p| p.len() + 1).sum();
                    let rest = &line[field_end..];
                    let (len_str, remaining) = rest.split_once(':')
                        .ok_or_else(|| err("expected len:content format"))?;
                    let content_len: usize = len_str.trim().parse()
                        .map_err(|_| err("invalid content length"))?;
                    let content_start = remaining.trim_start();
                    if content_start.len() < content_len {
                        return Err(err("content length exceeds line length"));
                    }
                    content_start[..content_len].to_string()
                } else {
                    String::new()
                };
                let mut data = ElementData::new(next_id);
                data.x = x;
                data.y = y;
                data.stroke_width = sw;
                data.stroke_color = stroke;
                data.fill_color = fill;
                next_id += 1;
                elements.push(Element::Text(Text { data, content }));
            }
            "freehand" => {
                if parts.len() < 4 {
                    return Err(err("expected: tag sw stroke x,y ..."));
                }
                let sw: f64 = parts[1].parse().map_err(|_| err("invalid stroke_width"))?;
                let stroke =
                    ShapeColor::from_name(parts[2]).ok_or_else(|| err("invalid stroke color"))?;
                let mut pts = Vec::new();
                for p in &parts[3..] {
                    let (xs, ys) = p
                        .split_once(',')
                        .ok_or_else(|| err("invalid point, expected x,y without spaces"))?;
                    let x: f64 = xs.parse().map_err(|_| err("invalid point x"))?;
                    let y: f64 = ys.parse().map_err(|_| err("invalid point y"))?;
                    pts.push(Point { x, y });
                }
                let mut data = ElementData::new(next_id);
                data.stroke_width = sw;
                data.stroke_color = stroke;
                next_id += 1;
                elements.push(Element::Freehand(Freehand { data, points: pts }));
            }
            other => return Err(err(&format!("unknown element type: {other}"))),
        }
    }

    Ok(Scene { elements, next_id })
}

#[cfg(test)]
mod tests {
    use super::*;

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
            content: "hello world".into(),
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
    fn round_trip() {
        let scene = make_scene();
        let saved = save_to_string(&scene);
        println!("{saved}");
        let loaded = load_from_str(&saved).unwrap();
        assert_eq!(scene.elements.len(), loaded.elements.len());
        for (a, b) in scene.elements.iter().zip(loaded.elements.iter()) {
            match (a, b) {
                (Element::Rectangle(ra), Element::Rectangle(rb)) => {
                    let da = &ra.data;
                    let db = &rb.data;
                    assert_eq!(da.x, db.x);
                    assert_eq!(da.y, db.y);
                    assert_eq!(da.width, db.width);
                    assert_eq!(da.height, db.height);
                    assert_eq!(da.stroke_width, db.stroke_width);
                    assert_eq!(da.stroke_color, db.stroke_color);
                    assert_eq!(da.fill_color, db.fill_color);
                }
                (Element::Ellipse(ea), Element::Ellipse(eb)) => {
                    let da = &ea.data;
                    let db = &eb.data;
                    assert_eq!(da.x, db.x);
                    assert_eq!(da.y, db.y);
                    assert_eq!(da.width, db.width);
                    assert_eq!(da.height, db.height);
                    assert_eq!(da.stroke_width, db.stroke_width);
                    assert_eq!(da.stroke_color, db.stroke_color);
                    assert_eq!(da.fill_color, db.fill_color);
                }
                (Element::Line(la), Element::Line(lb)) => {
                    assert_eq!(la.data.stroke_width, lb.data.stroke_width);
                    assert_eq!(la.data.stroke_color, lb.data.stroke_color);
                    assert_eq!(la.a.x, lb.a.x);
                    assert_eq!(la.a.y, lb.a.y);
                    assert_eq!(la.b.x, lb.b.x);
                    assert_eq!(la.b.y, lb.b.y);
                }
                (Element::Arrow(aa), Element::Arrow(ab)) => {
                    assert_eq!(aa.data.stroke_width, ab.data.stroke_width);
                    assert_eq!(aa.data.stroke_color, ab.data.stroke_color);
                    assert_eq!(aa.a.x, ab.a.x);
                    assert_eq!(aa.a.y, ab.a.y);
                    assert_eq!(aa.b.x, ab.b.x);
                    assert_eq!(aa.b.y, ab.b.y);
                }
                (Element::Text(ta), Element::Text(tb)) => {
                    assert_eq!(ta.data.x, tb.data.x);
                    assert_eq!(ta.data.y, tb.data.y);
                    assert_eq!(ta.data.fill_color, tb.data.fill_color);
                    assert_eq!(ta.content, tb.content);
                }
                (Element::Freehand(fa), Element::Freehand(fb)) => {
                    assert_eq!(fa.data.stroke_width, fb.data.stroke_width);
                    assert_eq!(fa.data.stroke_color, fb.data.stroke_color);
                    assert_eq!(fa.points.len(), fb.points.len());
                    for (pa, pb) in fa.points.iter().zip(fb.points.iter()) {
                        assert_eq!(pa.x, pb.x);
                        assert_eq!(pa.y, pb.y);
                    }
                }
                _ => panic!("element type mismatch"),
            }
        }
    }

    #[test]
    fn missing_version_header() {
        let err = load_from_str("rect 0 0 10 10 2 White none").unwrap_err();
        assert!(err.contains("version header"));
    }

    #[test]
    fn unsupported_version() {
        let err = load_from_str("v 99").unwrap_err();
        assert!(err.contains("unsupported"));
    }
}
