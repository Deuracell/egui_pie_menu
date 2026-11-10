//! Drawing of the directional highlight (arc, wedge and/or dot) at the centre indicator.

use crate::settings::HighlightShape;
use egui::{Color32, Pos2, Stroke, epaint::Shape, vec2};

/// Everything needed to draw one highlight.
pub(crate) struct Highlight {
    pub shape: HighlightShape,
    pub center: Pos2,
    /// Radius of the arc/wedge, and the dot's distance from `center`.
    pub radius: f32,
    /// Start and end angle of the arc/wedge, in radians.
    pub angle_range: std::ops::Range<f32>,
    /// Direction of the dot, in radians.
    pub dot_angle: f32,
    pub dot_radius: f32,
    /// Stroke of the arc and of the dot's outline.
    pub stroke: Stroke,
    /// Fill of the wedge and the dot.
    pub fill: Color32,
}

/// Points per logical pixel of arc length.
const ARC_RESOLUTION: f32 = 10.0;

/// Points along the arc of `radius` around `center` over `angle_range`.
///
/// Degenerate input (non-positive or NaN radius, non-finite angles) yields no points
/// instead of NaN positions.
pub(crate) fn arc_points(center: Pos2, radius: f32, angle_range: std::ops::Range<f32>) -> Vec<Pos2> {
    let finite = angle_range.start.is_finite() && angle_range.end.is_finite();
    if radius.is_nan() || radius <= 0.0 || !finite {
        return Vec::new();
    }
    let width = angle_range.end - angle_range.start;
    let n_points = ((width.abs() * radius * ARC_RESOLUTION).ceil() as usize).max(1);
    let step = width / n_points as f32;
    (0..=n_points)
        .map(|i| {
            let angle = angle_range.start + i as f32 * step;
            center + vec2(angle.cos(), angle.sin()) * radius
        })
        .collect()
}

/// Paints the components of `h.shape`: wedge first, then the arc, then the dot.
pub(crate) fn paint(painter: &egui::Painter, h: Highlight) {
    let shape = h.shape;
    if shape.arc || shape.slice {
        let points = arc_points(h.center, h.radius, h.angle_range);
        if points.len() >= 2 {
            if shape.slice {
                let mut wedge = Vec::with_capacity(points.len() + 1);
                wedge.push(h.center);
                wedge.extend_from_slice(&points);
                painter.add(Shape::convex_polygon(wedge, h.fill, Stroke::NONE));
            }
            if shape.arc && h.stroke.width > 0.0 && h.stroke.color != Color32::TRANSPARENT {
                painter.add(Shape::line(points, h.stroke));
            }
        }
    }
    if shape.circle {
        let center = h.center + h.radius * vec2(h.dot_angle.cos(), h.dot_angle.sin());
        let visible = h.stroke.color != Color32::TRANSPARENT || h.fill != Color32::TRANSPARENT;
        if h.dot_radius > 0.0 && center.is_finite() && visible {
            painter.circle(center, h.dot_radius, h.fill, h.stroke);
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use super::*;
    use crate::{button::*, highlight_shapes::*, mnemonic::*, *};
    use egui::{Color32, FontId, Key, Pos2, Rect, Stroke, Vec2};

    mod highlight_tests {
        use super::*;

        #[test]
        fn arc_points_are_finite_for_degenerate_input() {
            use crate::highlight_shapes::arc_points;
            let c = Pos2::ZERO;
            assert!(arc_points(c, f32::NAN, 0.0..1.0).is_empty());
            assert!(arc_points(c, 15.0, 0.0..f32::NAN).is_empty());
            for p in arc_points(c, 15.0, 1.0..1.0) {
                assert!(p.x.is_finite() && p.y.is_finite());
            }
        }
    }
}
