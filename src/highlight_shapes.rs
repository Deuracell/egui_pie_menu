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

/// How far a chord of the drawn arc may stray from the true arc, in physical pixels.
const ARC_TOLERANCE: f32 = 0.1;

/// Points along the arc of `radius` around `center` over `angle_range`, as few as keep
/// every chord within [`ARC_TOLERANCE`] of the arc at `pixels_per_point`.
///
/// Degenerate input (non-positive or NaN radius, non-finite angles) yields no points
/// instead of NaN positions.
pub(crate) fn arc_points(center: Pos2, radius: f32, angle_range: std::ops::Range<f32>, pixels_per_point: f32) -> Vec<Pos2> {
    let finite = angle_range.start.is_finite() && angle_range.end.is_finite();
    if radius.is_nan() || radius <= 0.0 || !finite {
        return Vec::new();
    }
    let width = angle_range.end - angle_range.start;
    // A chord spanning the angle `a` strays r·(1 − cos(a/2)) from its arc.
    let ppp = if pixels_per_point.is_finite() && pixels_per_point > 0.0 { pixels_per_point } else { 1.0 };
    let max_step = 2.0 * (1.0 - (ARC_TOLERANCE / ppp / radius).min(1.0)).acos();
    let n_points = ((width.abs() / max_step).ceil() as usize).clamp(2, 4096);
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
        let width = (h.angle_range.end - h.angle_range.start).abs();
        let points = arc_points(h.center, h.radius, h.angle_range, painter.pixels_per_point());
        if points.len() >= 2 {
            if shape.slice {
                // A wedge wider than half a turn is not convex: fill it in convex pieces.
                let pieces = ((width / std::f32::consts::PI).ceil() as usize).max(1);
                let segments = points.len() - 1;
                let per_piece = segments.div_ceil(pieces);
                for piece in points.chunks(per_piece).enumerate().map(|(i, _)| i * per_piece) {
                    let end = (piece + per_piece).min(segments);
                    let mut wedge = Vec::with_capacity(end - piece + 2);
                    wedge.push(h.center);
                    wedge.extend_from_slice(&points[piece..=end]);
                    if wedge.len() >= 3 {
                        painter.add(Shape::convex_polygon(wedge, h.fill, Stroke::NONE));
                    }
                }
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
            assert!(arc_points(c, f32::NAN, 0.0..1.0, 1.0).is_empty());
            assert!(arc_points(c, 15.0, 0.0..f32::NAN, 1.0).is_empty());
            for ppp in [1.0, 0.0, f32::NAN] {
                for p in arc_points(c, 15.0, 1.0..1.0, ppp) {
                    assert!(p.x.is_finite() && p.y.is_finite());
                }
            }
            for p in arc_points(c, 0.01, 0.0..7.0, 1.0) {
                assert!(p.x.is_finite() && p.y.is_finite());
            }
        }

        #[test]
        fn arc_has_as_few_points_as_stay_within_the_tolerance() {
            use crate::highlight_shapes::{ARC_TOLERANCE, arc_points};
            for (radius, ppp) in [(15.0f32, 1.0f32), (15.0, 2.0), (200.0, 1.0)] {
                let points = arc_points(Pos2::ZERO, radius, 0.0..std::f32::consts::FRAC_PI_4, ppp);
                assert!(points.len() < 40, "radius {radius}: {} points", points.len());
                // The midpoint of every chord is within the tolerance of the arc.
                for pair in points.windows(2) {
                    let mid = pair[0] + (pair[1] - pair[0]) / 2.0;
                    let off = (radius - mid.to_vec2().length()) * ppp;
                    assert!(off <= ARC_TOLERANCE * 1.05, "radius {radius} ppp {ppp}: off by {off} px");
                }
                assert!((points[0] - Pos2::new(radius, 0.0)).length() < 1e-3);
            }
            // Finer on a denser display.
            let n = |ppp| arc_points(Pos2::ZERO, 100.0, 0.0..1.0, ppp).len();
            assert!(n(2.0) > n(1.0));
        }

        /// The fill of a highlight wedge `angle` wide, as the polygons painted.
        fn wedge_polygons(angle: f32) -> Vec<Vec<Pos2>> {
            let ctx = egui::Context::default();
            let mut out = ctx.run_ui(Default::default(), |ui| {
                paint(ui.painter(), Highlight {
                    shape: HighlightShape::SLICE,
                    center: Pos2::new(100.0, 100.0),
                    radius: 15.0,
                    angle_range: 0.0..angle,
                    dot_angle: 0.0,
                    dot_radius: 5.0,
                    stroke: Stroke::new(1.0, Color32::WHITE),
                    fill: Color32::RED,
                });
            });
            out.textures_delta.clear();
            out.shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Path(p) if p.fill == Color32::RED => Some(p.points.clone()),
                    _ => None,
                })
                .collect()
        }

        #[test]
        fn a_wedge_wider_than_half_a_turn_is_filled_in_convex_pieces() {
            use std::f32::consts::PI;
            assert_eq!(wedge_polygons(PI / 4.0).len(), 1);
            assert_eq!(wedge_polygons(PI).len(), 1);
            for angle in [1.5 * PI, 2.0 * PI] {
                let polygons = wedge_polygons(angle);
                assert_eq!(polygons.len(), 2, "angle {angle}");
                let centre = Pos2::new(100.0, 100.0);
                let mut swept = 0.0;
                for polygon in &polygons {
                    assert_eq!(polygon[0], centre);
                    let a = |p: Pos2| (p - centre).angle();
                    // Each piece spans at most half a turn, so it is convex.
                    let piece = polygon[1..].windows(2).map(|w| PieMenu::angle_diff(a(w[0]), a(w[1]))).sum::<f32>();
                    assert!(piece <= PI + 1e-3, "piece spans {piece}");
                    swept += piece;
                }
                assert!((swept - angle).abs() < 1e-2, "pieces cover {swept} of {angle}");
            }
        }
    }
}
