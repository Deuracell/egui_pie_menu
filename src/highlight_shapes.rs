//! Drawing of the directional highlight (arc, wedge and/or dot) at the centre indicator.

use crate::settings::{HighlightShape, PieMenuToothSettings};
use egui::{Color32, Pos2, Stroke, epaint::{Mesh, Shape, Vertex, WHITE_UV}, vec2};

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

/// The outline of a tooth: a simple (never self-crossing) closed polygon.
pub(crate) struct ToothOutline {
    /// Sides and tip first, then the base from the left corner to the right corner.
    pub points: Vec<Pos2>,
    /// How many of the leading `points` are the sides and the tip.
    pub side_points: usize,
}

/// Points per side of the tooth, and along its base.
const TOOTH_STEPS: usize = 28;
const BASE_STEPS: usize = 16;

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// `progress^(1/easing)`, safe for any easing.
fn eased(progress: f32, easing: f32) -> f32 {
    progress.powf(1.0 / easing.max(0.05))
}

/// The tooth at `progress` (0 to 1) growing from the circle of `base_radius` around
/// `center` toward `angle`. Its tip is kept within `max_length` of the base.
///
/// Walking from the base to the tip in steps `s = 0..=1`, each step sits at radius
/// `base_radius + length * s` and at plus and minus a half-width angle
/// `half * (1 - s^q)^(1/q)` from the axis. The radius only grows and the sides never swap
/// places, so the outline cannot cross itself. Returns `None` for no progress, or for input
/// that is not finite.
pub(crate) fn tooth_outline(
    center: Pos2,
    base_radius: f32,
    angle: f32,
    progress: f32,
    s: &PieMenuToothSettings,
    max_length: f32,
) -> Option<ToothOutline> {
    let inputs = [base_radius, angle, progress, s.length, s.start_length, s.start_width,
        s.end_width, s.start_roundness, s.end_sharpness, s.inset, s.dig, s.length_easing,
        s.width_easing, s.sharpness_easing];
    if !center.is_finite() || inputs.iter().any(|v| !v.is_finite()) || max_length.is_nan() || progress <= 0.0 {
        return None;
    }
    let progress = progress.min(1.0);
    let rb = base_radius + s.inset;
    if rb <= 0.0 {
        return None;
    }
    // A short ramp so the tooth appears from nothing at the edge of the dead zone.
    let ramp = (progress / 0.08).min(1.0);
    let length = (lerp(s.start_length, s.length, eased(progress, s.length_easing)) * ramp)
        .min(max_length)
        .max(0.0);
    if length < 0.3 {
        return None;
    }
    let half = (lerp(s.start_width, s.end_width, eased(progress, s.width_easing)) / 2.0)
        .clamp(0.01, 85.0_f32.to_radians());
    let q = lerp(s.start_roundness, s.end_sharpness, eased(progress, s.sharpness_easing)).clamp(0.2, 4.0);
    let dig = s.dig.clamp(0.0, rb * 0.5);
    let at = |a: f32, r: f32| center + r * vec2(a.cos(), a.sin());
    let profile = |t: f32| (1.0 - t.powf(q)).max(0.0).powf(1.0 / q);
    // Denser steps near the tip, where the shape changes fastest.
    let step = |i: usize| {
        let u = i as f32 / TOOTH_STEPS as f32;
        1.0 - (1.0 - u) * (1.0 - u)
    };
    let mut points = Vec::with_capacity(2 * TOOTH_STEPS + BASE_STEPS + 1);
    // Right side, base to tip.
    for i in 0..=TOOTH_STEPS {
        let t = step(i);
        points.push(at(angle + half * profile(t), rb + length * t));
    }
    // Left side, from just below the tip down to the base.
    for i in (0..TOOTH_STEPS).rev() {
        let t = step(i);
        points.push(at(angle - half * profile(t), rb + length * t));
    }
    let side_points = points.len();
    // The base, left corner to right corner, along the circle.
    for i in 1..BASE_STEPS {
        let psi = -half + 2.0 * half * i as f32 / BASE_STEPS as f32;
        points.push(at(angle + psi, rb - dig * (1.0 - (psi / half).powi(2))));
    }
    Some(ToothOutline { points, side_points })
}

fn cross(a: Pos2, b: Pos2, c: Pos2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// Splits a simple polygon into triangles (ear clipping), as indices into `points`.
/// egui can only fill convex polygons and a tooth with concave sides is not one.
pub(crate) fn triangulate(points: &[Pos2]) -> Vec<u32> {
    let n = points.len();
    if n < 3 {
        return Vec::new();
    }
    let area: f32 = (0..n).map(|i| {
        let (a, b) = (points[i], points[(i + 1) % n]);
        a.x * b.y - b.x * a.y
    }).sum();
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    let mut rest: Vec<usize> = (0..n).collect();
    let mut out = Vec::with_capacity((n - 2) * 3);
    let inside = |p: Pos2, a: Pos2, b: Pos2, c: Pos2| {
        sign * cross(a, b, p) >= 0.0 && sign * cross(b, c, p) >= 0.0 && sign * cross(c, a, p) >= 0.0
    };
    while rest.len() > 3 {
        let m = rest.len();
        let ear = (0..m).find(|&i| {
            let (a, b, c) = (rest[(i + m - 1) % m], rest[i], rest[(i + 1) % m]);
            let (pa, pb, pc) = (points[a], points[b], points[c]);
            if sign * cross(pa, pb, pc) <= 0.0 {
                return false;
            }
            !rest.iter().any(|&k| k != a && k != b && k != c && inside(points[k], pa, pb, pc))
        });
        // A degenerate sliver has no ear: clip the first corner so it still terminates.
        let i = ear.unwrap_or(0);
        out.extend([rest[(i + m - 1) % m] as u32, rest[i] as u32, rest[(i + 1) % m] as u32]);
        rest.remove(i);
    }
    out.extend(rest.iter().map(|&i| i as u32));
    out
}

/// Paints a tooth: filled with `fill`, then outlined with `stroke` (sides and tip, plus the
/// base when `outline_base`).
pub(crate) fn paint_tooth(
    painter: &egui::Painter,
    tooth: &ToothOutline,
    fill: Color32,
    stroke: Stroke,
    outline_base: bool,
) {
    if fill != Color32::TRANSPARENT {
        let mut mesh = Mesh::default();
        mesh.vertices = tooth.points.iter().map(|&pos| Vertex { pos, uv: WHITE_UV, color: fill }).collect();
        mesh.indices = triangulate(&tooth.points);
        painter.add(Shape::mesh(mesh));
    }
    if stroke.width > 0.0 && stroke.color != Color32::TRANSPARENT {
        if outline_base {
            painter.add(Shape::closed_line(tooth.points.clone(), stroke));
        } else {
            painter.add(Shape::line(tooth.points[..tooth.side_points].to_vec(), stroke));
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use super::*;
    use crate::{button::*, highlight_shapes::*, mnemonic::*, *};
    use egui::{Color32, FontId, Key, Pos2, Rect, Stroke, Vec2};


    mod tooth_tests {
        use super::*;
        use crate::settings::PieMenuToothSettings;
        use crate::highlight_shapes::{tooth_outline, triangulate};

        const C: Pos2 = Pos2::new(100.0, 50.0);

        fn outline(angle: f32, progress: f32, s: &PieMenuToothSettings) -> Option<crate::highlight_shapes::ToothOutline> {
            tooth_outline(C, 20.0, angle, progress, s, f32::INFINITY)
        }

        fn dist(p: Pos2) -> f32 {
            (p - C).length()
        }

        /// Tiny deterministic generator, so the random test needs no dependency.
        struct Lcg(u64);
        impl Lcg {
            fn next(&mut self, lo: f32, hi: f32) -> f32 {
                self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                lo + ((self.0 >> 40) as f32 / (1u64 << 24) as f32) * (hi - lo)
            }
        }

        fn segments_cross(a: Pos2, b: Pos2, c: Pos2, d: Pos2) -> bool {
            let o = |p: Pos2, q: Pos2, r: Pos2| (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
            o(a, b, c) * o(a, b, d) < 0.0 && o(c, d, a) * o(c, d, b) < 0.0
        }

        fn is_simple(p: &[Pos2]) -> bool {
            let n = p.len();
            for i in 0..n {
                for j in (i + 2)..n {
                    if i == 0 && j == n - 1 {
                        continue;
                    }
                    if segments_cross(p[i], p[(i + 1) % n], p[j], p[(j + 1) % n]) {
                        return false;
                    }
                }
            }
            true
        }

        fn area(p: &[Pos2]) -> f32 {
            let n = p.len();
            (0..n).map(|i| p[i].x * p[(i + 1) % n].y - p[(i + 1) % n].x * p[i].y).sum::<f32>().abs() / 2.0
        }

        #[test]
        fn no_tooth_without_progress_or_with_bad_input() {
            let s = PieMenuToothSettings::default();
            assert!(outline(0.3, 0.0, &s).is_none());
            assert!(outline(f32::NAN, 0.5, &s).is_none());
            assert!(outline(0.3, f32::NAN, &s).is_none());
            assert!(tooth_outline(C, -5.0, 0.3, 0.5, &s, 100.0).is_none());
            let bad = PieMenuToothSettings { length: f32::INFINITY, ..s };
            assert!(outline(0.3, 0.5, &bad).is_none());
        }

        #[test]
        fn tip_reaches_base_radius_plus_length_at_full_travel() {
            let s = PieMenuToothSettings::default();
            let t = outline(0.7, 1.0, &s).unwrap();
            let far = t.points.iter().map(|&p| dist(p)).fold(0.0, f32::max);
            assert!((far - (20.0 + s.length)).abs() < 0.01, "{far}");
        }

        #[test]
        fn length_is_capped_by_max_length() {
            let s = PieMenuToothSettings::default();
            let t = tooth_outline(C, 20.0, 0.7, 1.0, &s, 10.0).unwrap();
            let far = t.points.iter().map(|&p| dist(p)).fold(0.0, f32::max);
            assert!((far - 30.0).abs() < 0.01, "{far}");
        }

        #[test]
        fn base_follows_the_circle_and_nothing_is_inside_it() {
            let s = PieMenuToothSettings { inset: 3.0, ..Default::default() };
            let t = outline(-1.0, 0.6, &s).unwrap();
            for &p in &t.points {
                assert!(dist(p) >= 23.0 - 0.01, "a point is inside the circle: {}", dist(p));
            }
            for &p in &t.points[t.side_points..] {
                assert!((dist(p) - 23.0).abs() < 0.01);
            }
        }

        #[test]
        fn dig_curves_the_base_into_the_circle() {
            let s = PieMenuToothSettings { dig: 6.0, ..Default::default() };
            let t = outline(0.0, 0.6, &s).unwrap();
            let lowest = t.points[t.side_points..].iter().map(|&p| dist(p)).fold(f32::MAX, f32::min);
            assert!((lowest - 14.0).abs() < 0.5, "{lowest}");
            // Never past the centre, however large.
            let huge = PieMenuToothSettings { dig: 1000.0, ..Default::default() };
            let t = outline(0.0, 0.6, &huge).unwrap();
            assert!(t.points.iter().all(|&p| dist(p) >= 10.0 - 0.01));
        }

        #[test]
        fn starts_round_and_sharpens() {
            let s = PieMenuToothSettings::default();
            // Width of the tooth a quarter of the way up, relative to its length.
            let slenderness = |progress: f32| {
                let t = outline(0.0, progress, &s).unwrap();
                let len = t.points.iter().map(|&p| dist(p)).fold(0.0, f32::max) - 20.0;
                let mid_r = 20.0 + len * 0.75;
                let ys: Vec<f32> = t.points[..t.side_points].iter()
                    .filter(|p| (dist(**p) - mid_r).abs() < len * 0.1).map(|p| p.y - C.y).collect();
                let width = ys.iter().cloned().fold(f32::MIN, f32::max) - ys.iter().cloned().fold(f32::MAX, f32::min);
                width / len
            };
            assert!(slenderness(1.0) < slenderness(0.15), "the tooth should narrow relative to its length");
        }

        #[test]
        fn outline_is_never_self_crossing_or_non_finite() {
            let mut r = Lcg(7);
            for _ in 0..3000 {
                let s = PieMenuToothSettings {
                    length: r.next(5.0, 80.0),
                    start_length: r.next(0.0, 30.0),
                    start_width: r.next(10.0, 150.0).to_radians(),
                    end_width: r.next(10.0, 150.0).to_radians(),
                    start_roundness: r.next(1.0, 3.0),
                    end_sharpness: r.next(0.4, 2.0),
                    inset: r.next(-6.0, 6.0),
                    dig: r.next(0.0, 40.0),
                    length_easing: r.next(0.5, 3.0),
                    width_easing: r.next(0.5, 3.0),
                    sharpness_easing: r.next(0.5, 3.0),
                    outline_base: false,
                };
                let radius = r.next(10.0, 40.0);
                let t = tooth_outline(C, radius, r.next(-3.2, 3.2), r.next(0.01, 1.0), &s, f32::INFINITY);
                let Some(t) = t else { continue };
                assert!(t.points.iter().all(|p| p.is_finite()), "{s:?}");
                assert!(is_simple(&t.points), "self-crossing outline for {s:?} at radius {radius}");
            }
        }

        #[test]
        fn triangles_cover_exactly_the_polygon() {
            for s in [
                PieMenuToothSettings::default(),
                PieMenuToothSettings { end_sharpness: 2.0, ..Default::default() },
                PieMenuToothSettings { dig: 8.0, start_width: 2.4, ..Default::default() },
            ] {
                for progress in [0.1, 0.5, 1.0] {
                    let t = outline(0.4, progress, &s).unwrap();
                    let idx = triangulate(&t.points);
                    assert_eq!(idx.len(), (t.points.len() - 2) * 3);
                    let covered: f32 = idx.chunks(3)
                        .map(|c| area(&[t.points[c[0] as usize], t.points[c[1] as usize], t.points[c[2] as usize]])).sum();
                    let whole = area(&t.points);
                    assert!((covered - whole).abs() < whole * 0.001 + 0.01, "{covered} vs {whole}");
                }
            }
        }
    }

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
