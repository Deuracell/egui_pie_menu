//! The tooth of the center indicator: a shape of circular arcs that grows out of the circle toward
//! the pointer. See [`PieMenuToothSettings`] for what the settings mean.
//!
//! Local coordinates have `x` across the tooth and `y` along its axis, with the circle's center
//! at the origin; `tooth_outline` returns world coordinates. Everything is computed in `f64`, so
//! that the arcs are tangent to the circle to within rounding, and converted to `f32` at the end.
//!
//! The construction, for a circle of radius `R`, a reference point `T = (0, D)` with `D = R + length`,
//! and a bend `b` in `[0, 1]`:
//!
//! * The right main arc leaves the circle at the angle `phi` (it is tangent there, so it is concave)
//!   and ends at `T`. Its circle has radius `r` and its centre is `(R + r)(sin phi, cos phi)`.
//! * The two mirrored arcs would just touch on the axis at `r_touch = (D^2 - R^2) / (2R)`; a bend `b`
//!   (a share of that curvature, capped, see [`bend_cap`]) gives `r = r_touch / b`, and `b = 0` is a
//!   straight line. Then `cos(phi) = (R^2 + 2rR + D^2) / (2D(R + r))`.
//! * The tip fillet is a circle on the axis tangent to both arcs, which trims them back from `T`.
//! * The root is a line along the circle's edge from each foot, outwards.

use crate::settings::PieMenuToothSettings;
use egui::{Color32, Pos2, Stroke, epaint::{Mesh, Shape, Vertex, WHITE_UV}, pos2};
use std::f64::consts::{FRAC_PI_2, PI, TAU};

type V = [f64; 2];

fn add(a: V, b: V) -> V {
    [a[0] + b[0], a[1] + b[1]]
}
fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1]]
}
fn mul(a: V, k: f64) -> V {
    [a[0] * k, a[1] * k]
}
fn unit(a: V) -> V {
    let l = a[0].hypot(a[1]);
    if l > 0.0 { [a[0] / l, a[1] / l] } else { a }
}
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
fn polar(radius: f64, angle: f64) -> V {
    [radius * angle.sin(), radius * angle.cos()]
}

/// The tip angle below which a fillet no longer fits on the arc.
const ALPHA_MIN: f64 = 0.04;
/// The most the bend can be, as a share of the curvature at which the two arcs would touch.
const BEND_MAX: f64 = 0.85;
/// The full shape is reached at this share of the progress.
const FULL_AT: f64 = 0.9;

/// One piece of a main arc.
#[derive(Clone, Copy)]
enum Curve {
    Line { p0: V, p1: V },
    /// A circle piece starting at the angle `a0` and turning by `sweep` (negative: clockwise).
    Circle { c: V, r: f64, a0: f64, sweep: f64 },
}

/// The right main arc: its curve, the half-angle `alpha` between its end and the axis, and the angle
/// `phi` around the circle where it meets it.
#[derive(Clone, Copy)]
struct MainArc {
    curve: Curve,
    alpha: f64,
    phi: f64,
}

fn main_arc(rb: f64, d: f64, bend: f64) -> MainArc {
    let r_touch = (d * d - rb * rb) / (2.0 * rb);
    let b = bend.clamp(0.0, 1.0);
    if b < 1e-4 || r_touch / b > 1e6 {
        let phi = (rb / d).min(1.0).acos();
        return MainArc {
            curve: Curve::Line { p0: polar(rb, phi), p1: [0.0, d] },
            alpha: (rb / d).min(1.0).asin(),
            phi,
        };
    }
    let r = r_touch / b;
    let phi = ((rb * rb + 2.0 * r * rb + d * d) / (2.0 * d * (rb + r))).clamp(-1.0, 1.0).acos();
    let (cx, cy) = ((rb + r) * phi.sin(), (rb + r) * phi.cos());
    let a0 = -FRAC_PI_2 - phi;
    let mut sweep = (d - cy).atan2(-cx) - a0;
    while sweep > 1e-9 {
        sweep -= TAU;
    }
    while sweep <= -TAU {
        sweep += TAU;
    }
    MainArc { curve: Curve::Circle { c: [cx, cy], r, a0, sweep }, alpha: (cx / r).min(1.0).acos(), phi }
}

/// The sharpest bend (a share of the touching curvature) that still takes a fillet: at most
/// [`BEND_MAX`], and lower when the tip angle would otherwise drop below [`ALPHA_MIN`], which
/// happens with a long tooth on a small circle. The setting's `1.0` is this value.
fn bend_cap(rb: f64, d: f64) -> f64 {
    let alpha = |b: f64| main_arc(rb, d, b).alpha;
    if alpha(BEND_MAX) >= ALPHA_MIN {
        return BEND_MAX;
    }
    let (mut lo, mut hi) = (0.0, BEND_MAX);
    for _ in 0..24 {
        let m = (lo + hi) / 2.0;
        if alpha(m) >= ALPHA_MIN { lo = m } else { hi = m }
    }
    lo
}

fn sample(curve: Curve, n: usize) -> Vec<V> {
    (0..=n)
        .map(|i| {
            let f = i as f64 / n as f64;
            match curve {
                Curve::Line { p0, p1 } => add(p0, mul(sub(p1, p0), f)),
                Curve::Circle { c, r, a0, sweep } => {
                    let w = a0 + sweep * f;
                    [c[0] + r * w.cos(), c[1] + r * w.sin()]
                }
            }
        })
        .collect()
}

/// The curve cut short at `q`, a point on it; `None` when `q` is beyond its end.
fn trim_end(curve: Curve, q: V) -> Option<Curve> {
    match curve {
        Curve::Line { p0, .. } => Some(Curve::Line { p0, p1: q }),
        Curve::Circle { c, r, a0, sweep } => {
            let mut d = (q[1] - c[1]).atan2(q[0] - c[0]) - a0;
            while d > 1e-9 {
                d -= TAU;
            }
            while d <= -TAU {
                d += TAU;
            }
            (d.abs() <= sweep.abs() + 1e-6).then_some(Curve::Circle { c, r, a0, sweep: d })
        }
    }
}

struct Fillet {
    /// Centre, on the axis.
    fc: V,
    /// Where it touches the right arc.
    q: V,
    rho: f64,
}

/// A fillet of radius `rho` on the axis, tangent to the main arc near the reference point `(0, d)`.
fn fillet(arc: &MainArc, rho: f64, d: f64) -> Option<Fillet> {
    match arc.curve {
        Curve::Line { .. } => {
            if arc.alpha < 0.03 {
                return None;
            }
            let fc = [0.0, d - rho / arc.alpha.sin()];
            Some(Fillet { fc, q: add(fc, mul([arc.alpha.cos(), arc.alpha.sin()], rho)), rho })
        }
        Curve::Circle { c, r, .. } => {
            let dy = ((r + rho) * (r + rho) - c[0] * c[0]).max(0.0).sqrt();
            let fc = [0.0, c[1] - dy];
            Some(Fillet { fc, q: add(fc, mul(unit(sub(c, fc)), rho)), rho })
        }
    }
}

/// The main arc trimmed by the biggest fillet, up to `rho`, that fits on it.
fn with_fillet(arc: &MainArc, rho: f64, d: f64) -> (Curve, Option<Fillet>) {
    if rho < 0.05 || arc.alpha < 0.03 {
        return (arc.curve, None);
    }
    let attempt = |r: f64| fillet(arc, r, d).and_then(|f| trim_end(arc.curve, f.q).map(|c| (c, f)));
    if let Some((c, f)) = attempt(rho) {
        return (c, Some(f));
    }
    let (mut lo, mut hi) = (0.0, rho);
    let mut best = None;
    for _ in 0..18 {
        let m = (lo + hi) / 2.0;
        match attempt(m) {
            Some(found) => {
                lo = m;
                best = Some(found);
            }
            None => hi = m,
        }
    }
    match best {
        Some((c, f)) => (c, Some(f)),
        None => (arc.curve, None),
    }
}

/// The tooth for one progress value, in world coordinates.
pub(crate) struct ToothOutline {
    /// The tooth from its right foot over the tip to its left foot: an open path.
    pub mid: Vec<Pos2>,
    /// The closed polygon to fill: `mid` plus the stretch of the circle's edge between the feet.
    pub fill: Vec<Pos2>,
    /// The root lines, one per side, from the foot outwards along the circle's edge. Empty without a root.
    pub roots: [Vec<Pos2>; 2],
    /// The centre of the circle the roots follow.
    pub center: Pos2,
}

/// The tooth growing out of the circle of radius `base_radius` around `center` toward `angle`, at
/// `progress` (0 to 1). `max_length` caps the length. `None` for no progress, no room, or input that is
/// not finite.
pub(crate) fn tooth_outline(
    center: Pos2,
    base_radius: f32,
    angle: f32,
    progress: f32,
    s: &PieMenuToothSettings,
    max_length: f32,
) -> Option<ToothOutline> {
    let inputs = [
        base_radius, angle, progress, s.length, s.start_length, s.bend, s.start_bend, s.tip_radius,
        s.start_tip_radius, s.root, s.start_root, s.length_easing, s.sharpen_easing,
    ];
    if !center.is_finite() || inputs.iter().any(|v| !v.is_finite()) || max_length.is_nan() || progress <= 0.0 || base_radius <= 0.0 {
        return None;
    }
    let r0 = base_radius as f64;
    let tt = (progress.min(1.0) as f64 / FULL_AT).min(1.0);
    let grow = tt.powf(1.0 / (s.length_easing as f64).max(0.05));
    let sharpen = tt.powf((s.sharpen_easing as f64).max(0.05));

    let length = lerp(s.start_length as f64, s.length as f64, grow).min(max_length as f64).max(0.0);
    if length < 0.5 {
        return None;
    }
    let d = r0 + length;
    let bend = lerp(s.start_bend as f64, s.bend as f64, grow).clamp(0.0, 1.0);
    let rho = lerp(s.start_tip_radius as f64, s.tip_radius as f64, sharpen).max(0.0);
    let psi = lerp(s.start_root as f64, s.root as f64, grow).clamp(0.0, PI);

    let arc = main_arc(r0, d, bend * bend_cap(r0, d));
    let (curve, fil) = with_fillet(&arc, rho, d);
    let top = fil.as_ref().map_or(d, |f| f.fc[1] + f.rho);
    // Under a point tall it cannot be seen, and its arcs lie so close to the circle that rounding
    // would make the outline cross itself.
    if top - r0 < 1.0 {
        return None;
    }

    // The right main arc, with its points kept on its own side of the axis.
    let main: Vec<V> = sample(curve, 36).into_iter().map(|q| [q[0].max(0.0), q[1]]).collect();
    let mirror_rev = |pts: &[V]| -> Vec<V> { pts.iter().rev().map(|q| [-q[0], q[1]]).collect() };
    let mut mid = main.clone();
    if let Some(f) = &fil {
        let w0 = (f.q[0] - f.fc[0]).atan2(f.q[1] - f.fc[1]);
        for i in 1..=16 {
            let w = w0 * (1.0 - 2.0 * i as f64 / 16.0);
            mid.push([f.rho * w.sin(), f.fc[1] + f.rho * w.cos()]);
        }
    }
    mid.extend(mirror_rev(&main).into_iter().skip(1));

    // The base: the circle's edge between the two feet, without the feet themselves.
    let phi = arc.phi;
    let mut fill = mid.clone();
    // (A hair inside the edge: the arcs next to the feet hug the circle, and rounding must not make them cross it.)
    fill.extend((1..24).map(|i| polar(r0 - 0.03, -phi + 2.0 * phi * i as f64 / 24.0)));

    // The roots: lines along the circle's edge from each foot, not so far that they meet behind it.
    let th = phi.max((phi + psi).min(PI - 0.1));
    let right: Vec<V> = if th - phi > 1e-3 { (0..=24).map(|i| polar(r0, phi + (th - phi) * i as f64 / 24.0)).collect() } else { Vec::new() };
    let left: Vec<V> = right.iter().map(|q| [-q[0], q[1]]).collect();

    let (sin, cos) = (angle.sin() as f64, angle.cos() as f64);
    let world = |pts: &[V]| -> Vec<Pos2> {
        pts.iter()
            .map(|q| pos2((center.x as f64 + q[1] * cos - q[0] * sin) as f32, (center.y as f64 + q[1] * sin + q[0] * cos) as f32))
            .collect()
    };
    Some(ToothOutline { mid: world(&mid), fill: world(&fill), roots: [world(&right), world(&left)], center })
}

fn cross(a: Pos2, b: Pos2, c: Pos2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// Splits a simple polygon into triangles (ear clipping), as indices into `points`. egui can only
/// fill convex polygons, and a tooth with concave sides is not one.
pub(crate) fn triangulate(points: &[Pos2]) -> Vec<u32> {
    let n = points.len();
    if n < 3 {
        return Vec::new();
    }
    let area: f32 = (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum();
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

/// Paints a tooth: filled with `fill`, outlined with `stroke` (sides and tip, plus the base when
/// `outline_base`), and its roots as lines of the same stroke, optionally tapered to a point.
pub(crate) fn paint_tooth(
    painter: &egui::Painter,
    tooth: &ToothOutline,
    fill: Color32,
    stroke: Stroke,
    outline_base: bool,
    taper: bool,
) {
    if fill != Color32::TRANSPARENT {
        painter.add(Shape::mesh(Mesh {
            vertices: tooth.fill.iter().map(|&pos| Vertex { pos, uv: WHITE_UV, color: fill }).collect(),
            indices: triangulate(&tooth.fill),
            ..Default::default()
        }));
    }
    if stroke.width <= 0.0 || stroke.color == Color32::TRANSPARENT {
        return;
    }
    if outline_base {
        painter.add(Shape::closed_line(tooth.fill.clone(), stroke));
    } else {
        painter.add(Shape::line(tooth.mid.clone(), stroke));
    }
    for root in tooth.roots.iter().filter(|r| r.len() >= 2) {
        if !taper {
            painter.add(Shape::line(root.clone(), stroke));
            continue;
        }
        // A strip along the root whose width goes from the stroke's to nothing.
        let n = root.len();
        let mut mesh = Mesh::default();
        for (i, &p) in root.iter().enumerate() {
            let half = stroke.width / 2.0 * (1.0 - i as f32 / (n - 1) as f32);
            let radial = (p - tooth.center).normalized();
            for side in [1.0, -1.0] {
                mesh.vertices.push(Vertex { pos: p + radial * (half * side), uv: WHITE_UV, color: stroke.color });
            }
        }
        for i in 0..(n as u32 - 1) {
            let (a, b, c, d) = (2 * i, 2 * i + 1, 2 * i + 2, 2 * i + 3);
            mesh.indices.extend([a, b, c, b, d, c]);
        }
        painter.add(Shape::mesh(mesh));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const C: Pos2 = Pos2::new(100.0, 50.0);
    const R0: f32 = 20.0;

    fn outline(progress: f32, s: &PieMenuToothSettings) -> Option<ToothOutline> {
        tooth_outline(C, R0, 0.0, progress, s, f32::INFINITY)
    }
    fn dist(p: Pos2) -> f32 {
        (p - C).length()
    }
    /// How far the tooth reaches along its axis (angle 0 is +x), from the circle's centre.
    fn reach(t: &ToothOutline) -> f32 {
        t.mid.iter().map(|p| p.x - C.x).fold(f32::MIN, f32::max)
    }
    fn sharp() -> PieMenuToothSettings {
        PieMenuToothSettings { tip_radius: 0.0, start_tip_radius: 0.0, ..Default::default() }
    }

    /// A small deterministic generator, so the random test needs no dependency.
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self, lo: f32, hi: f32) -> f32 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            lo + ((self.0 >> 40) as f32 / (1u64 << 24) as f32) * (hi - lo)
        }
    }

    fn crosses(a: Pos2, b: Pos2, c: Pos2, d: Pos2) -> bool {
        let o = |p: Pos2, q: Pos2, r: Pos2| (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
        // Ignore a crossing of a hundredth of a point: below what can be drawn.
        o(a, b, c) * o(a, b, d) < -1e-2 && o(c, d, a) * o(c, d, b) < -1e-2
    }
    fn is_simple(p: &[Pos2]) -> bool {
        let n = p.len();
        (0..n).all(|i| ((i + 2)..n).all(|j| (i == 0 && j == n - 1) || !crosses(p[i], p[(i + 1) % n], p[j], p[(j + 1) % n])))
    }
    fn area(p: &[Pos2]) -> f32 {
        let n = p.len();
        (0..n).map(|i| p[i].x * p[(i + 1) % n].y - p[(i + 1) % n].x * p[i].y).sum::<f32>().abs() / 2.0
    }

    #[test]
    fn no_tooth_without_progress_or_with_bad_input() {
        let s = PieMenuToothSettings::default();
        assert!(outline(0.0, &s).is_none());
        assert!(tooth_outline(C, R0, f32::NAN, 0.5, &s, 100.0).is_none());
        assert!(tooth_outline(C, R0, 0.0, f32::NAN, &s, 100.0).is_none());
        assert!(tooth_outline(C, -5.0, 0.0, 0.5, &s, 100.0).is_none());
        assert!(tooth_outline(C, R0, 0.0, 0.5, &PieMenuToothSettings { length: f32::INFINITY, ..s }, 100.0).is_none());
        assert!(tooth_outline(C, R0, 0.0, 1.0, &s, 0.2).is_none(), "no room: nothing to draw");
    }

    #[test]
    fn without_a_fillet_the_tooth_reaches_the_reference_point() {
        for bend in [0.0, 0.3, 1.0] {
            let s = PieMenuToothSettings { bend, ..sharp() };
            let t = outline(1.0, &s).unwrap();
            assert!((reach(&t) - (R0 + s.length)).abs() < 0.01, "bend {bend}: {}", reach(&t));
        }
    }

    #[test]
    fn the_fillet_eats_length_and_still_works_at_full_bend() {
        // The fillet must have an effect over the whole bend range, for short and long teeth on small and large circles.
        for r0 in [10.0, 16.0, 22.0, 40.0] {
            for length in [20.0, 46.0, 70.0, 100.0] {
                for bend in [0.0, 0.5, 1.0] {
                    let base = PieMenuToothSettings { length, bend, start_bend: bend, start_length: length, ..sharp() };
                    let with = PieMenuToothSettings { tip_radius: 3.0, start_tip_radius: 3.0, ..base };
                    let reach_of = |s: &PieMenuToothSettings| {
                        tooth_outline(C, r0, 0.0, 1.0, s, f32::INFINITY).unwrap().mid.iter().map(|p| p.x - C.x).fold(f32::MIN, f32::max)
                    };
                    assert!(reach_of(&base) - reach_of(&with) > 0.5, "r0 {r0} length {length} bend {bend}");
                }
            }
        }
    }

    #[test]
    fn the_feet_are_tangent_to_the_circle_and_nothing_is_inside_it() {
        let s = PieMenuToothSettings::default();
        let t = outline(1.0, &s).unwrap();
        for foot in [0, t.mid.len() - 1] {
            assert!((dist(t.mid[foot]) - R0).abs() < 1e-3, "the foot is on the circle");
        }
        // The arc leaves the circle along its tangent: its first step is almost perpendicular to the radius.
        let (p0, p1) = (t.mid[0], t.mid[1]);
        let step = (p1 - p0).normalized();
        let radial = (p0 - C).normalized();
        assert!(step.dot(radial).abs() < 0.02, "not tangent: {}", step.dot(radial));
        assert!(t.mid.iter().all(|p| dist(*p) >= R0 - 1e-3));
        assert!(t.fill.iter().all(|p| dist(*p) >= R0 - 0.05), "the base is at most a hair inside");
    }

    #[test]
    fn bend_zero_is_a_straight_line() {
        let s = PieMenuToothSettings { bend: 0.0, start_bend: 0.0, ..sharp() };
        let t = outline(1.0, &s).unwrap();
        let half = t.mid.len() / 2 + 1;
        let (a, b) = (t.mid[0], t.mid[half - 1]);
        for p in &t.mid[..half] {
            let off = ((*p - a).x * (b - a).y - (*p - a).y * (b - a).x).abs() / (b - a).length();
            assert!(off < 1e-3, "off the line by {off}");
        }
    }

    #[test]
    fn the_full_shape_is_reached_at_ninety_percent() {
        let s = PieMenuToothSettings::default();
        let (a, b) = (outline(0.9, &s).unwrap(), outline(1.0, &s).unwrap());
        assert_eq!(a.mid.len(), b.mid.len());
        assert!(a.mid.iter().zip(&b.mid).all(|(p, q)| (*p - *q).length() < 1e-4));
        let early = outline(0.45, &s).unwrap();
        assert!(reach(&early) < reach(&b) - 1.0, "still growing at 45 %");
    }

    #[test]
    fn the_length_is_capped() {
        let s = sharp();
        let t = tooth_outline(C, R0, 0.0, 1.0, &s, 10.0).unwrap();
        assert!((reach(&t) - (R0 + 10.0)).abs() < 0.01);
    }

    #[test]
    fn the_root_is_a_line_along_the_circle() {
        let none = PieMenuToothSettings { root: 0.0, start_root: 0.0, ..Default::default() };
        assert!(outline(1.0, &none).unwrap().roots.iter().all(|r| r.is_empty()));
        let s = PieMenuToothSettings { root: 40.0_f32.to_radians(), start_root: 40.0_f32.to_radians(), ..Default::default() };
        let t = outline(1.0, &s).unwrap();
        let [right, left] = &t.roots;
        assert!(right.len() > 2 && right.len() == left.len());
        assert!(right.iter().chain(left).all(|p| (dist(*p) - R0).abs() < 1e-3), "the roots follow the circle's edge");
        assert!((right[0] - t.mid[0]).length() < 1e-3, "it starts at the foot");
        assert!((left[0] - t.mid[t.mid.len() - 1]).length() < 1e-3);
        // Extending the root too far must not make the two roots meet behind the circle.
        let wrap = PieMenuToothSettings { root: PI as f32, start_root: PI as f32, ..Default::default() };
        let t = outline(1.0, &wrap).unwrap();
        let ends = (*t.roots[0].last().unwrap() - *t.roots[1].last().unwrap()).length();
        assert!(ends > 1.0, "the roots end {ends} apart");
    }

    #[test]
    fn outlines_are_simple_and_finite_for_random_settings() {
        let mut r = Lcg(11);
        for _ in 0..3000 {
            let s = PieMenuToothSettings {
                length: r.next(8.0, 100.0),
                start_length: r.next(0.0, 100.0),
                bend: r.next(0.0, 1.0),
                start_bend: r.next(0.0, 1.0),
                tip_radius: if r.next(0.0, 1.0) < 0.2 { 0.0 } else { r.next(0.0, 20.0) },
                start_tip_radius: r.next(0.0, 40.0),
                root: r.next(0.0, 2.1),
                start_root: r.next(0.0, 2.1),
                root_taper: false,
                length_easing: r.next(0.5, 3.0),
                sharpen_easing: r.next(1.0, 4.0),
                outline_base: false,
            };
            let radius = r.next(10.0, 40.0);
            for progress in [r.next(0.01, 0.2), r.next(0.2, 0.6), r.next(0.6, 1.0), 1.0] {
                let Some(t) = tooth_outline(C, radius, r.next(-3.2, 3.2), progress, &s, f32::INFINITY) else { continue };
                assert!(t.fill.iter().chain(t.roots.iter().flatten()).all(|p| p.is_finite()), "{s:?}");
                assert!(is_simple(&t.fill), "self-crossing outline for {s:?} at radius {radius}, progress {progress}");
                assert!(t.fill.iter().chain(t.roots.iter().flatten()).all(|p| (*p - C).length() >= radius - 0.05), "inside the circle: {s:?}");
            }
        }
    }

    #[test]
    fn triangles_cover_exactly_the_polygon() {
        for s in [
            PieMenuToothSettings::default(),
            PieMenuToothSettings { bend: 0.0, ..Default::default() },
            PieMenuToothSettings { tip_radius: 0.0, ..Default::default() },
        ] {
            for progress in [0.1, 0.5, 1.0] {
                let t = outline(progress, &s).unwrap();
                let idx = triangulate(&t.fill);
                assert_eq!(idx.len(), (t.fill.len() - 2) * 3);
                let covered: f32 = idx.chunks(3).map(|c| area(&[t.fill[c[0] as usize], t.fill[c[1] as usize], t.fill[c[2] as usize]])).sum();
                let whole = area(&t.fill);
                assert!((covered - whole).abs() < whole * 0.001 + 0.01, "{covered} vs {whole}");
            }
        }
    }
}
