use crate::model::elements::path::{eval_cubic_bezier, segment_cubic_bezier};
use crate::model::elements::text::{
    measure_text_width, EmbeddedText, FONT_FAMILY, FONT_STACK, TEXT_ASCENT_RATIO,
};
use crate::model::elements::{CurveMode, Element, ElementData};
use crate::model::{Point, Rect};
use leptos::{view, IntoView};

const SHAPE_WIDTH_RATIO: f64 = 0.8;
const SHAPE_HEIGHT_RATIO: f64 = 0.8;
const ELLIPSE_WIDTH_RATIO: f64 = 0.68;
const ELLIPSE_HEIGHT_RATIO: f64 = 0.68;
const MIN_EMBEDDED_FONT_SIZE: f64 = 6.0;

/// Resolved position and wrapping for a label in the host element's current
/// geometry. Box labels rotate with their host; path labels remain horizontal.
#[derive(Clone, Debug)]
pub struct EmbeddedTextLayout {
    pub center: Point,
    pub width: f64,
    pub height: f64,
    pub font_size: f64,
    pub rotation: f64,
    pub lines: Vec<String>,
    pub path_bounds: Option<Rect>,
    pub mask_width: f64,
}

pub fn layout_for_element(element: &Element) -> Option<EmbeddedTextLayout> {
    let data = element.data();
    match element {
        Element::Rectangle(_) => layout_box(data, SHAPE_WIDTH_RATIO, SHAPE_HEIGHT_RATIO),
        Element::Ellipse(_) => layout_box(data, ELLIPSE_WIDTH_RATIO, ELLIPSE_HEIGHT_RATIO),
        Element::Line(line) => {
            layout_path(data, &sample_path(&line.points, line.line_style.curve_mode))
        }
        Element::Connector(connector) => layout_path(data, &connector.route_points()),
        _ => None,
    }
}

pub fn reference_size_for_element(element: &Element) -> Option<(f64, f64)> {
    match element {
        Element::Rectangle(_) | Element::Ellipse(_) => Some((
            element.data().width.abs().max(1.0),
            element.data().height.abs().max(1.0),
        )),
        Element::Line(line) => {
            let sampled = sample_path(&line.points, line.line_style.curve_mode);
            let length = polyline_length(&sampled).max(1.0);
            Some((length, length))
        }
        Element::Connector(connector) => {
            let length = polyline_length(&connector.route_points()).max(1.0);
            Some((length, length))
        }
        _ => None,
    }
}

pub fn layout_rectangle(data: &ElementData) -> Option<EmbeddedTextLayout> {
    layout_box(data, SHAPE_WIDTH_RATIO, SHAPE_HEIGHT_RATIO)
}

pub fn layout_ellipse(data: &ElementData) -> Option<EmbeddedTextLayout> {
    layout_box(data, ELLIPSE_WIDTH_RATIO, ELLIPSE_HEIGHT_RATIO)
}

pub fn layout_line(
    data: &ElementData,
    points: &[Point],
    mode: CurveMode,
) -> Option<EmbeddedTextLayout> {
    layout_path(data, &sample_path(points, mode))
}

pub fn layout_box(
    data: &ElementData,
    width_ratio: f64,
    height_ratio: f64,
) -> Option<EmbeddedTextLayout> {
    let label = data.embedded_text.as_ref()?;
    let width = (data.width.abs() * width_ratio).max(1.0);
    let available_height = (data.height.abs() * height_ratio).max(1.0);
    let mut font_size = scaled_font_size(label, data.width, data.height);
    let mut lines;
    for _ in 0..24 {
        lines = wrap_text(label.wrapped.raw.as_str(), width, font_size);
        let height = lines.len().max(1) as f64 * font_size * 1.2;
        if height <= available_height || font_size <= MIN_EMBEDDED_FONT_SIZE {
            break;
        }
        font_size = (font_size * 0.9).max(MIN_EMBEDDED_FONT_SIZE);
    }
    lines = wrap_text(label.wrapped.raw.as_str(), width, font_size);
    let height = lines.len().max(1) as f64 * font_size * 1.2;
    Some(EmbeddedTextLayout {
        center: Point::new(
            data.world_point.x + data.width / 2.0,
            data.world_point.y + data.height / 2.0,
        ),
        width,
        height,
        font_size,
        rotation: data.rotation,
        lines,
        path_bounds: None,
        mask_width: 0.0,
    })
}

pub fn layout_path(data: &ElementData, points: &[Point]) -> Option<EmbeddedTextLayout> {
    let label = data.embedded_text.as_ref()?;
    let route_length = polyline_length(points);
    if route_length <= f64::EPSILON || points.len() < 2 {
        return None;
    }
    let width = (route_length * 0.5).max(1.0);
    let font_size = scaled_font_size(label, route_length, route_length);
    let lines = wrap_text(label.wrapped.raw.as_str(), width, font_size);
    let height = lines.len().max(1) as f64 * font_size * 1.2;
    let widest = lines
        .iter()
        .map(|line| measure_text_width(line, font_size, FONT_FAMILY))
        .fold(0.0, f64::max);
    let mask_width = (widest + font_size * 0.5).min(width).max(1.0);
    let center = point_at_distance(points, route_length / 2.0)?;
    let path_bounds = bounds_of_points(points);
    Some(EmbeddedTextLayout {
        center,
        width,
        height,
        font_size,
        rotation: 0.0,
        lines,
        path_bounds: Some(path_bounds),
        mask_width,
    })
}

fn wrap_text(raw: &str, max_width: f64, font_size: f64) -> Vec<String> {
    raw.split('\n')
        .flat_map(|line| wrap_line(line, max_width, font_size))
        .collect()
}

fn wrap_line(line: &str, max_width: f64, font_size: f64) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }
    let mut result = Vec::new();
    let mut current = String::new();
    for word in line.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if measure_text_width(&candidate, font_size, FONT_FAMILY) <= max_width {
            current = candidate;
            continue;
        }
        if !current.is_empty() {
            result.push(std::mem::take(&mut current));
        }
        let mut part = String::new();
        for character in word.chars() {
            let mut candidate = part.clone();
            candidate.push(character);
            if !part.is_empty()
                && measure_text_width(&candidate, font_size, FONT_FAMILY) > max_width
            {
                result.push(std::mem::take(&mut part));
            }
            part.push(character);
        }
        current = part;
    }
    if !current.is_empty() {
        result.push(current);
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}

fn scaled_font_size(label: &EmbeddedText, width: f64, height: f64) -> f64 {
    let sx = width.abs() / label.reference_width.max(1.0);
    let sy = height.abs() / label.reference_height.max(1.0);
    (label.font_size * sx.min(sy).clamp(0.2, 4.0)).max(MIN_EMBEDDED_FONT_SIZE)
}

fn sample_path(points: &[Point], mode: CurveMode) -> Vec<Point> {
    if points.len() <= 2 || mode == CurveMode::Straight {
        return points.to_vec();
    }
    let mut sampled = Vec::new();
    for i in 0..points.len() - 1 {
        if let Some((p0, p1, p2, p3)) = segment_cubic_bezier(points, i) {
            for step in 0..16 {
                sampled.push(eval_cubic_bezier(&p0, &p1, &p2, &p3, step as f64 / 16.0));
            }
        }
    }
    if let Some(last) = points.last() {
        sampled.push(*last);
    }
    sampled
}

fn polyline_length(points: &[Point]) -> f64 {
    points
        .windows(2)
        .map(|segment| (segment[1].x - segment[0].x).hypot(segment[1].y - segment[0].y))
        .sum()
}

fn point_at_distance(points: &[Point], distance: f64) -> Option<Point> {
    let mut remaining = distance;
    for segment in points.windows(2) {
        let a = segment[0];
        let b = segment[1];
        let length = (b.x - a.x).hypot(b.y - a.y);
        if remaining <= length {
            let t = if length <= f64::EPSILON {
                0.0
            } else {
                remaining / length
            };
            return Some(Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
        }
        remaining -= length;
    }
    points.last().copied()
}

fn bounds_of_points(points: &[Point]) -> Rect {
    let (min_x, max_x, min_y, max_y) = points.iter().fold(
        (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ),
        |(min_x, max_x, min_y, max_y), point| {
            (
                min_x.min(point.x),
                max_x.max(point.x),
                min_y.min(point.y),
                max_y.max(point.y),
            )
        },
    );
    Rect::new(
        min_x,
        min_y,
        (max_x - min_x).max(1.0),
        (max_y - min_y).max(1.0),
    )
}

pub fn mask_id(data: &ElementData) -> String {
    format!("embedded-text-mask-{}", data.id)
}

pub fn render_mask(data: &ElementData, layout: &EmbeddedTextLayout) -> leptos::View {
    if data
        .embedded_text
        .as_ref()
        .is_none_or(|label| label.wrapped.raw.is_empty())
    {
        return ().into_view();
    }
    let Some(bounds) = layout.path_bounds else {
        return ().into_view();
    };
    let id = mask_id(data);
    let margin = 16.0;
    let x = bounds.x - margin;
    let y = bounds.y - margin;
    let width = bounds.w + margin * 2.0;
    let height = bounds.h + margin * 2.0;
    let label_x = layout.center.x - layout.mask_width / 2.0;
    let label_y = layout.center.y - layout.height / 2.0;
    let label_h = layout.height + layout.font_size * 0.35;
    view! {
        <defs>
            <mask id=id maskUnits="userSpaceOnUse" x=x y=y width=width height=height>
                <rect x=x y=y width=width height=height fill="white" />
                <rect
                    x=label_x
                    y=label_y
                    width=layout.mask_width
                    height=label_h
                    fill="black"
                />
            </mask>
        </defs>
    }
    .into_view()
}

pub fn render_label(
    label: &EmbeddedText,
    layout: &EmbeddedTextLayout,
    opacity: f64,
) -> leptos::View {
    let x = layout.center.x;
    let baseline = layout.center.y - layout.height / 2.0 + layout.font_size * TEXT_ASCENT_RATIO;
    let font_size = layout.font_size;
    let fill = label.color.to_hex();
    let font_family = FONT_STACK;
    let lines = layout.lines.clone();
    let text = view! {
        <text
            x=x
            y=baseline
            fill=fill
            font-size=font_size
            font-family=font_family
            text-anchor="middle"
            pointer-events="none"
            style="user-select: none; -webkit-user-select: none;"
            opacity=opacity
        >
            {lines
                .iter()
                .enumerate()
                .map(|(index, line)| {
                    let dy = if index == 0 { "0" } else { "1.2em" };
                    view! { <tspan x=x dy=dy>{line.clone()}</tspan> }
                })
                .collect::<Vec<_>>()}
        </text>
    }
    .into_view();
    if layout.rotation == 0.0 {
        text
    } else {
        let deg = layout.rotation.to_degrees();
        let cx = layout.center.x;
        let cy = layout.center.y;
        view! { <g transform=format!("rotate({deg} {cx} {cy})")>{text}</g> }.into_view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Color;

    fn path_data(raw: &str) -> ElementData {
        let mut data = ElementData::new(1);
        let mut label = EmbeddedText::new(Color::new(Color::RED), 18.0, 300.0, 300.0);
        label.set_content(raw);
        data.embedded_text = Some(label);
        data
    }

    #[test]
    fn path_label_sits_halfway_along_a_multi_segment_route() {
        let data = path_data("route label");
        let points = [
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 200.0),
        ];
        let layout = layout_path(&data, &points).unwrap();

        assert_eq!(layout.center, Point::new(100.0, 50.0));
        assert_eq!(layout.width, 150.0);
    }

    #[test]
    fn long_words_wrap_within_the_half_route_limit() {
        let data = path_data("averylongunbrokenword");
        let points = [Point::new(0.0, 0.0), Point::new(100.0, 0.0)];
        let layout = layout_path(&data, &points).unwrap();

        assert!(layout.lines.len() > 1);
        assert!(layout.lines.iter().all(|line| measure_text_width(
            line,
            layout.font_size,
            FONT_FAMILY
        ) <= layout.width));
    }
}
