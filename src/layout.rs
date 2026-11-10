//! Layout geometry: where each direction sits on the layout shape, button offsets,
//! and which button the pointer picks.

use crate::{PieButton, PieDirection, PieMenu};
// Everything else at the crate root, for the doc links below.
#[allow(unused_imports)]
use crate::*;
use egui::{Key, Vec2};
use std::f32::consts::TAU;

impl PieMenu {
    /// The direction as displayed: mirrored when [`PieMenuSettings::mirror_horizontal`] is on.
    pub(crate) fn eff(dir: &PieDirection, mirror: bool) -> PieDirection {
        if mirror { dir.mirrored() } else { *dir }
    }

    /// Converts a [`PieDirection`] to a [`Vec2`] offset, applying the shape factor.
    ///
    /// - `0.0`  → circle: all buttons equidistant (L2, unit vector)
    /// - `+1.0` → square: diagonals pushed out to the corners (L∞ norm)
    /// - `-1.0` → diamond: diagonals pulled in toward the centre (L1 norm)
    pub(crate) fn direction_vec(dir: &PieDirection, shape_factor: f32) -> Vec2 {
        let fraction = match dir {
            PieDirection::North => 0.75,
            PieDirection::NorthEast => 0.875,
            PieDirection::East => 1.0,
            PieDirection::SouthEast => 0.125,
            PieDirection::South => 0.25,
            PieDirection::SouthWest => 0.375,
            PieDirection::West => 0.5,
            PieDirection::NorthWest => 0.625,
        };
        let angle = TAU * fraction;
        let circle = Vec2::new(angle.cos(), angle.sin());

        match dir {
            PieDirection::NorthEast
            | PieDirection::SouthEast
            | PieDirection::SouthWest
            | PieDirection::NorthWest => {
                if shape_factor > 0.0 {
                    let l_inf = circle / circle.x.abs().max(circle.y.abs());
                    circle * (1.0 - shape_factor) + l_inf * shape_factor
                } else if shape_factor < 0.0 {
                    let l1 = circle / (circle.x.abs() + circle.y.abs());
                    let t = -shape_factor;
                    circle * (1.0 - t) + l1 * t
                } else {
                    circle
                }
            }
            _ => circle,
        }
    }

    /// The point at `angle` on the layout shape of unit radius: the circle, blended
    /// towards the square (`sf > 0`) or the diamond (`sf < 0`), as [`Self::direction_vec`]
    /// does for the eight directions.
    pub(crate) fn curve_vec(angle: f32, sf: f32) -> Vec2 {
        let circle = Vec2::new(angle.cos(), angle.sin());
        if sf > 0.0 {
            let l_inf = circle / circle.x.abs().max(circle.y.abs());
            circle * (1.0 - sf) + l_inf * sf
        } else if sf < 0.0 {
            let l1 = circle / (circle.x.abs() + circle.y.abs());
            circle * (1.0 + sf) + l1 * -sf
        } else {
            circle
        }
    }

    /// Where on the layout shape (unit radius) a button's inner face/corner sits, as
    /// displayed. Cardinals, and diagonals without [`PieMenuSettings::intercardinal_height`],
    /// use their fixed direction. With an intercardinal height, a diagonal slides *along* the
    /// shape to the point that puts its centre at that height (so it follows the curve);
    /// this depends on the button's size and the radius. Its angle is also the angle used
    /// for hover picking and the centre highlight.
    pub(crate) fn layout_vec(&self, dir: &PieDirection, size: Vec2, radius: f32, sf: f32) -> Vec2 {
        let v = Self::direction_vec(dir, sf);
        let diagonal = v.x.abs() > 0.5 && v.y.abs() > 0.5;
        let (Some(t), true) = (self.settings.intercardinal_height, diagonal && radius > 0.0) else {
            return v;
        };
        let hh = size.y / 2.0;
        // A North/South button of this size has its centre at radius + hh; the diagonal's
        // centre should be at t of that, i.e. its inner corner at that minus hh.
        let corner = ((t.clamp(0.0, 1.0) * (radius + hh) - hh) / radius).clamp(0.0, 1.0);
        // The shape's height rises monotonically from 0 (East) to 1 (North) over the first
        // quadrant, so bisect for the angle where it reaches `corner`.
        let (mut lo, mut hi) = (0.0f32, std::f32::consts::FRAC_PI_2);
        for _ in 0..32 {
            let mid = (lo + hi) / 2.0;
            if Self::curve_vec(mid, sf).y < corner { lo = mid } else { hi = mid }
        }
        let u = Self::curve_vec((lo + hi) / 2.0, sf);
        Vec2::new(v.x.signum() * u.x, v.y.signum() * u.y)
    }

    /// Offset of a button's centre from the menu centre, for a layout `radius` and shape
    /// factor `sf`: its inner face/corner on the layout shape ([`Self::layout_vec`]), then
    /// pushed outward by half its size on each axis it points along.
    pub(crate) fn button_offset(&self, dir: &PieDirection, size: Vec2, radius: f32, sf: f32) -> Vec2 {
        let dn = Self::direction_vec(dir, sf).normalized();
        // 0.5 threshold: diagonal components are ~0.707, cardinal near-zero components are
        // floating-point noise (~1e-17) and must not contribute.
        let side = |c: f32| if c > 0.5 { 1.0 } else if c < -0.5 { -1.0 } else { 0.0 };
        let (sx, sy) = (side(dn.x), side(dn.y));
        self.layout_vec(dir, size, radius, sf) * radius + Vec2::new(size.x / 2.0 * sx, size.y / 2.0 * sy)
    }

    pub(crate) fn direction_digit(dir: &PieDirection) -> char {
        match dir {
            PieDirection::North => '8',
            PieDirection::NorthEast => '9',
            PieDirection::East => '6',
            PieDirection::SouthEast => '3',
            PieDirection::South => '2',
            PieDirection::SouthWest => '1',
            PieDirection::West => '4',
            PieDirection::NorthWest => '7',
        }
    }

    /// Smallest absolute difference between two angles, in `0..=PI`.
    pub(crate) fn angle_diff(a: f32, b: f32) -> f32 {
        let d = (a - b).rem_euclid(TAU);
        d.min(TAU - d)
    }

    /// Index of the enabled button the pointer vector `mouse_vec` (from the menu
    /// centre) points at, or `None`. `dirs[i]` is button `i`'s displayed layout vector
    /// ([`Self::layout_vec`]). Empty directions are skipped: the pointer snaps to the
    /// closest available button. On an exact tie (e.g. straight down between East and
    /// West buttons) the earlier button in the slice wins.
    pub(crate) fn pick_button(buttons: &[PieButton], mouse_vec: Vec2, dirs: &[Vec2]) -> Option<usize> {
        let angle = mouse_vec.y.atan2(mouse_vec.x);
        buttons
            .iter()
            .enumerate()
            .filter(|(_, b)| b.enabled)
            .map(|(idx, _)| (idx, Self::angle_diff(angle, dirs[idx].y.atan2(dirs[idx].x))))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(idx, _)| idx)
    }

    pub(crate) fn direction_numpad(dir: &PieDirection) -> Key {
        match dir {
            PieDirection::North => Key::Num8,
            PieDirection::NorthEast => Key::Num9,
            PieDirection::East => Key::Num6,
            PieDirection::SouthEast => Key::Num3,
            PieDirection::South => Key::Num2,
            PieDirection::SouthWest => Key::Num1,
            PieDirection::West => Key::Num4,
            PieDirection::NorthWest => Key::Num7,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use super::*;
    use crate::{button::*, highlight_shapes::*, mnemonic::*, *};
    use egui::{Color32, FontId, Key, Pos2, Rect, Stroke, Vec2};

    mod selection_tests {
        use super::*;

        fn btns() -> Vec<PieButton> {
            vec![
                PieButton::new(PieDirection::West),
                PieButton::new(PieDirection::NorthWest),
                PieButton::new(PieDirection::East),
            ]
        }

        /// Default layout vectors (no intercardinal height), as displayed.
        fn dirs(b: &[PieButton], mirror: bool) -> Vec<Vec2> {
            b.iter().map(|b| PieMenu::direction_vec(&PieMenu::eff(&b.direction, mirror), 0.0)).collect()
        }

        fn pick(b: &[PieButton], v: Vec2) -> Option<usize> {
            PieMenu::pick_button(b, v, &dirs(b, false))
        }

        #[test]
        fn west_sector_covers_both_sides_of_the_axis() {
            let b = btns();
            // Slightly above and below the pure-west axis must both pick West (index 0).
            assert_eq!(pick(&b, Vec2::new(-10.0, -1.0)), Some(0));
            assert_eq!(pick(&b, Vec2::new(-10.0, 1.0)), Some(0));
            assert_eq!(pick(&b, Vec2::new(-10.0, -6.0)), Some(1)); // beyond 22.5deg -> NW
        }

        #[test]
        fn empty_sectors_snap_to_the_closest_button() {
            let b = btns(); // W, NW, E
            assert_eq!(pick(&b, Vec2::new(-1.0, -10.0)), Some(1)); // north, slightly west -> NW
            assert_eq!(pick(&b, Vec2::new(10.0, 10.0)), Some(2)); // south-east -> E
        }

        #[test]
        fn disabled_buttons_are_skipped() {
            let mut b = btns();
            b[0].enabled = false;
            assert_eq!(pick(&b, Vec2::new(-10.0, 0.0)), Some(1)); // next nearest
            b[1].enabled = false;
            assert_eq!(pick(&b, Vec2::new(-10.0, 0.0)), Some(2));
            b[2].enabled = false;
            assert_eq!(pick(&b, Vec2::new(-10.0, 0.0)), None);
        }

        fn lab(d: PieDirection, label: &str) -> PieButton {
            PieButton::new(d).with_label(label)
        }

        #[test]
        fn problems_detect_same_direction_and_duplicate_mnemonics() {
            let b = vec![
                lab(PieDirection::North, "Copy").with_mnemonic('c'),
                lab(PieDirection::North, "Cut").with_mnemonic('x'),
                lab(PieDirection::East, "Cat").with_mnemonic('c'),
            ];
            let p = button_problems(&b, &resolve_mnemonics(&b, true));
            assert_eq!(
                p,
                vec![
                    ButtonProblem::SharedDirection { first: 0, second: 1, direction: PieDirection::North },
                    ButtonProblem::DuplicateMnemonic { first: 0, second: 2, key: 'c' },
                ]
            );
            assert_eq!(p[1].buttons(), [0, 2]);
            assert!(p[1].to_string().contains("mnemonic 'c'"));
        }

        #[test]
        fn problems_ignore_disabled_buttons_and_distinct_levels() {
            let b = vec![
                lab(PieDirection::North, "Copy").with_mnemonic('c'),
                lab(PieDirection::East, "Cat").with_mnemonic('c').with_enabled(false),
            ];
            assert!(button_problems(&b, &resolve_mnemonics(&b, true)).is_empty());
            // Another level may reuse the same direction and key: checked separately.
            let other = vec![lab(PieDirection::North, "Cut").with_mnemonic('c')];
            assert!(button_problems(&other, &resolve_mnemonics(&other, true)).is_empty());
        }

        #[cfg(debug_assertions)]
        #[test]
        #[should_panic(expected = "are both in direction")]
        fn show_asserts_on_buttons_sharing_a_direction() {
            let ctx = egui::Context::default();
            let buttons = vec![PieButton::new(PieDirection::North), PieButton::new(PieDirection::North)];
            let mut m = PieMenu::new();
            m.settings.show_behavior = ShowBehavior::Instant;
            m.open(Pos2::new(100.0, 100.0));
            let _ = ctx.run_ui(Default::default(), |ui| {
                m.show_with_pointer(ui.ctx(), &buttons, Some(Pos2::new(100.0, 100.0)), true, |_, _| {});
            });
        }

        #[test]
        fn mirror_applies_to_picking() {
            let b = btns(); // W, NW, E
            // Mirrored: the East button is displayed on the west side.
            assert_eq!(PieMenu::pick_button(&b, Vec2::new(-10.0, 0.0), &dirs(&b, true)), Some(2));
            assert_eq!(PieMenu::pick_button(&b, Vec2::new(10.0, 0.0), &dirs(&b, true)), Some(0));
        }

        #[test]
        fn disabled_buttons_get_no_generated_mnemonic() {
            let b = vec![PieButton::new(PieDirection::North).with_label("Copy").with_enabled(false)];
            assert_eq!(resolve_mnemonics(&b, true), vec![None]);
        }
    }

    mod intercardinal_height_tests {
        use super::*;

        const SIZE: Vec2 = Vec2::new(80.0, 24.0);
        const HALF: Vec2 = Vec2::new(40.0, 12.0);

        fn menu(height: Option<f32>, sf: f32) -> PieMenu {
            let mut m = PieMenu::new();
            m.settings.intercardinal_height = height;
            m.settings.shape_factor = sf;
            m
        }

        /// Inner corner of the NE button (centre minus the half-size push outward).
        fn ne_corner(m: &PieMenu) -> Vec2 {
            let r = m.settings.layout_radius;
            m.button_offset(&PieDirection::NorthEast, SIZE, r, m.settings.shape_factor) - Vec2::new(HALF.x, -HALF.y)
        }

        #[test]
        fn default_keeps_the_inner_corner_on_the_circle() {
            let m = menu(None, 0.0);
            let r = m.settings.layout_radius;
            let d = std::f32::consts::FRAC_1_SQRT_2 * r;
            assert!((ne_corner(&m) - Vec2::new(d, -d)).length() < 1e-3);
        }

        #[test]
        fn half_puts_diagonal_centres_halfway_and_on_the_curve() {
            let m = menu(Some(0.5), 0.0);
            let r = m.settings.layout_radius;
            let n = m.button_offset(&PieDirection::North, SIZE, r, 0.0);
            let e = m.button_offset(&PieDirection::East, SIZE, r, 0.0);
            for (dir, sign) in [
                (PieDirection::NorthEast, 1.0), (PieDirection::NorthWest, 1.0),
                (PieDirection::SouthEast, -1.0), (PieDirection::SouthWest, -1.0),
            ] {
                let o = m.button_offset(&dir, SIZE, r, 0.0);
                assert!((o.y - sign * (n.y + e.y) / 2.0).abs() < 1e-2, "{dir:?}: {o:?}");
            }
            // The inner corner stays on the circle: the button slid along it, outward.
            let c = ne_corner(&m);
            assert!((c.length() - r).abs() < 1e-2, "corner off the circle: {c:?}");
            assert!(c.x > ne_corner(&menu(None, 0.0)).x, "lowered diagonal should move outward");
        }

        #[test]
        fn follows_the_square_and_diamond_shapes() {
            for sf in [1.0, -1.0, 0.5] {
                let m = menu(Some(0.4), sf);
                let r = m.settings.layout_radius;
                let c = ne_corner(&m) / r;
                let on_shape = PieMenu::curve_vec((-c.y).atan2(c.x), sf);
                assert!((Vec2::new(c.x, -c.y) - on_shape).length() < 1e-3, "sf {sf}: {c:?} vs {on_shape:?}");
            }
        }

        #[test]
        fn picking_follows_the_lowered_diagonals() {
            let b = vec![PieButton::new(PieDirection::East), PieButton::new(PieDirection::NorthEast)];
            let r = PieMenu::new().settings.layout_radius;
            let dirs = |m: &PieMenu| -> Vec<Vec2> {
                b.iter().map(|b| m.layout_vec(&b.direction, SIZE, r, 0.0)).collect()
            };
            let a = 20f32.to_radians();
            let mouse = Vec2::new(a.cos(), -a.sin()); // 20° above East
            // At 45° the boundary is 22.5°, so 20° still picks East …
            assert_eq!(PieMenu::pick_button(&b, mouse, &dirs(&menu(None, 0.0))), Some(0));
            // … lowered to half height the boundary drops below 20°, so it picks NorthEast.
            let low = menu(Some(0.5), 0.0);
            assert_eq!(PieMenu::pick_button(&b, mouse, &dirs(&low)), Some(1));
            // Pointing at the drawn button's centre picks it.
            let centre = low.button_offset(&PieDirection::NorthEast, SIZE, r, 0.0);
            assert_eq!(PieMenu::pick_button(&b, centre, &dirs(&low)), Some(1));
        }
    }
}
