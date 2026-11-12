//! Tests of the menu's behaviour as a whole (open state, input, pinned mode, sub-pies).

#![allow(unused_imports)]
use super::*;
use crate::{button::*, mnemonic::*};

mod pin_tests {
    use super::*;

    #[test]
    fn reveal_shows_menu_immediately_and_open_resets_it() {
        let mut m = PieMenu::new();
        m.open(Pos2::new(5.0, 5.0));
        assert!(!m.visible);
        m.reveal();
        assert!(m.visible);
        m.open(Pos2::ZERO);
        assert!(!m.visible);
    }

    #[test]
    fn revealed_menu_stays_visible_without_key_or_mouse() {
        let ctx = egui::Context::default();
        let buttons = vec![PieButton::new(PieDirection::North)];
        let mut m = PieMenu::new();
        m.settings.block_background = false;
        m.open(Pos2::new(100.0, 100.0));
        m.reveal();
        let mut resp = PieMenuResponse::None;
        let mut out = ctx.run_ui(Default::default(), |ui| {
            resp = m.show_with_pointer(ui.ctx(), &buttons, Some(Pos2::new(100.0, 100.0)), true, |ui, _| {
                ui.label("x");
            });
        });
        out.textures_delta.clear();
        assert!(matches!(resp, PieMenuResponse::None));
        assert!(m.visible);
    }
}

mod open_state_tests {
    use super::*;

    fn key(key: Key) -> Vec<egui::Event> {
        vec![egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }]
    }

    fn click(pos: Pos2) -> Vec<egui::Event> {
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        vec![egui::Event::PointerMoved(pos), button(true), button(false)]
    }

    /// One frame with a single North button; returns the response and whether the
    /// render closure ran (i.e. the menu was drawn).
    fn frame(ctx: &egui::Context, m: &mut PieMenu, events: Vec<egui::Event>) -> (PieMenuResponse, bool) {
        let buttons = vec![PieButton::new(PieDirection::North).with_label("Copy")];
        let (mut resp, mut drawn) = (PieMenuResponse::None, false);
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            resp = m.show(ui.ctx(), &buttons, true, |ui, _| {
                drawn = true;
                ui.label("x");
            });
        });
        out.textures_delta.clear();
        (resp, drawn)
    }

    const CENTER: Pos2 = Pos2::new(400.0, 300.0);
    const ABOVE: Pos2 = Pos2::new(400.0, 200.0); // points at the North button

    #[test]
    fn show_does_nothing_until_opened_and_closes_on_a_response() {
        let ctx = egui::Context::default();
        let mut m = PieMenu::new();
        assert!(!m.is_open());
        assert_eq!(frame(&ctx, &mut m, key(Key::Num8)), (PieMenuResponse::None, false));

        m.open(CENTER);
        m.reveal();
        assert!(m.is_open());
        assert_eq!(frame(&ctx, &mut m, vec![]), (PieMenuResponse::None, true));
        assert_eq!(frame(&ctx, &mut m, key(Key::Num8)).0, PieMenuResponse::Selected(0));
        assert!(!m.is_open(), "a selection closes the menu");
        assert_eq!(frame(&ctx, &mut m, key(Key::Num8)), (PieMenuResponse::None, false));

        m.open(CENTER);
        m.reveal();
        assert_eq!(frame(&ctx, &mut m, key(Key::Escape)).0, PieMenuResponse::Dismissed);
        assert!(!m.is_open(), "a dismissal closes the menu");

        m.open(CENTER);
        m.close();
        assert_eq!(frame(&ctx, &mut m, vec![]), (PieMenuResponse::None, false));
    }

    #[test]
    fn show_reads_the_pointer_from_egui() {
        let ctx = egui::Context::default();
        let mut m = PieMenu::new();
        m.open(CENTER);
        m.reveal();
        frame(&ctx, &mut m, vec![egui::Event::PointerMoved(ABOVE)]);
        assert_eq!(m.hovered(), Some(0));
    }

    #[test]
    fn pinned_menu_stays_open_and_only_escape_dismisses_it() {
        let ctx = egui::Context::default();
        let mut m = PieMenu::new();
        m.pin(CENTER);
        assert!(m.is_pinned());
        assert_eq!(frame(&ctx, &mut m, vec![]), (PieMenuResponse::None, true)); // visible at once
        assert_eq!(frame(&ctx, &mut m, key(Key::Num8)).0, PieMenuResponse::Selected(0));
        assert!(m.is_pinned(), "a selection leaves a pinned menu open");
        // The default dismiss inputs (5, secondary click) don't apply while pinned.
        assert_eq!(frame(&ctx, &mut m, key(Key::Num5)).0, PieMenuResponse::None);
        assert_eq!(frame(&ctx, &mut m, key(Key::Escape)).0, PieMenuResponse::Dismissed);
        assert!(!m.is_open() && !m.is_pinned());
    }

    #[test]
    fn pointer_area_limits_hover_and_clicks() {
        let ctx = egui::Context::default();
        let mut m = PieMenu::new();
        m.pin(CENTER);
        m.pointer_area = Some(Rect::from_min_max(Pos2::new(0.0, 250.0), Pos2::new(800.0, 600.0)));
        frame(&ctx, &mut m, vec![]);
        // Outside the area: not hovered, and a click there neither selects nor dismisses.
        assert_eq!(frame(&ctx, &mut m, click(ABOVE)).0, PieMenuResponse::None);
        assert_eq!(m.hovered(), None);
        assert!(m.is_pinned());
        // Inside the area the same click selects.
        m.pointer_area = None;
        assert_eq!(frame(&ctx, &mut m, click(ABOVE)).0, PieMenuResponse::Selected(0));
    }

    #[test]
    fn open_after_pin_is_a_normal_menu_again() {
        let mut m = PieMenu::new();
        m.pin(CENTER);
        m.open(CENTER);
        assert!(m.is_open() && !m.is_pinned() && !m.visible);
    }

    #[test]
    fn configure_applies_changes_and_composes_in_any_order() {
        let menu = PieMenu::new().with_title("Edit").with_id("x").configure(|s| {
            s.layout.radius = 120.0;
            s.input.auto_mnemonics = true;
        });
        assert_eq!(menu.settings.layout.radius, 120.0);
        assert!(menu.settings.input.auto_mnemonics);
        assert_eq!(menu.title.as_deref(), Some("Edit"));
        assert_eq!(menu.id, egui::Id::new("x"));
        // Everything else untouched.
        let mut expected = PieMenuSettings::default();
        expected.layout.radius = 120.0;
        expected.input.auto_mnemonics = true;
        assert_eq!(format!("{:?}", menu.settings), format!("{expected:?}"));

        // After `with_settings`, it edits those settings; before it, they are replaced.
        let base = PieMenuSettings { mirror_horizontal: true, ..Default::default() };
        let menu = PieMenu::new().with_settings(base.clone()).configure(|s| s.layout.radius = 50.0).with_title("T");
        assert!(menu.settings.mirror_horizontal);
        assert_eq!(menu.settings.layout.radius, 50.0);
        assert_eq!(menu.title.as_deref(), Some("T"));
        let menu = PieMenu::new().configure(|s| s.layout.radius = 50.0).with_settings(base);
        assert_eq!(menu.settings.layout.radius, 100.0);
    }

    #[test]
    fn blender_like_differs_from_the_default_in_exactly_three_fields() {
        let b = PieMenuSettings::blender_like();
        let d = PieMenuSettings::default();
        assert!(!b.input.select_on_primary_click && d.input.select_on_primary_click);
        assert_eq!(b.text_color, Some(Color32::WHITE));
        assert_eq!(b.number_hints.color, Some(Color32::WHITE));
        // Undoing those three gives the default again, compared through Debug.
        let mut undone = b;
        undone.input.select_on_primary_click = true;
        undone.text_color = None;
        undone.number_hints.color = None;
        assert_eq!(format!("{undone:?}"), format!("{d:?}"));
    }

    #[test]
    fn blender_like_menu_ignores_a_click_but_selects_on_key_release() {
        let ctx = egui::Context::default();
        let buttons = vec![PieButton::new(PieDirection::North).with_label("Copy")];
        let run = |m: &mut PieMenu, pointer: Option<Pos2>, key_down: bool, events: Vec<egui::Event>| {
            let mut resp = PieMenuResponse::None;
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                resp = m.show_with_pointer(ui.ctx(), &buttons, pointer, key_down, |ui, _| {
                    ui.label("x");
                });
            });
            out.textures_delta.clear();
            resp
        };
        let mut m = PieMenu::new().with_settings(PieMenuSettings::blender_like());
        m.open(CENTER);
        m.reveal();
        assert_eq!(run(&mut m, Some(ABOVE), true, vec![]), PieMenuResponse::None);
        assert_eq!(m.hovered(), Some(0));
        assert_eq!(run(&mut m, Some(ABOVE), true, click(ABOVE)), PieMenuResponse::None);
        assert!(m.is_open(), "a primary click is ignored");
        assert_eq!(run(&mut m, Some(ABOVE), false, vec![]), PieMenuResponse::Selected(0));
        assert!(!m.is_open());

        // The default menu selects on the same click.
        let mut m = PieMenu::new();
        m.open(CENTER);
        m.reveal();
        assert_eq!(run(&mut m, Some(ABOVE), true, vec![]), PieMenuResponse::None);
        assert_eq!(run(&mut m, Some(ABOVE), true, click(ABOVE)), PieMenuResponse::Selected(0));
    }
}

mod subpie_tests {
    use super::*;

    #[test]
    fn resolve_level_walks_path() {
        let root = vec![
            PieButton::new(PieDirection::North),
            PieButton::new(PieDirection::East).with_children(vec![
                PieButton::new(PieDirection::West),
                PieButton::new(PieDirection::South).with_children(vec![PieButton::new(PieDirection::North)]),
            ]),
        ];
        assert_eq!(PieMenu::resolve_level(&root, &[]).unwrap().len(), 2);
        assert_eq!(PieMenu::resolve_level(&root, &[1]).unwrap().len(), 2);
        assert_eq!(PieMenu::resolve_level(&root, &[1, 1]).unwrap().len(), 1);
        assert!(PieMenu::resolve_level(&root, &[0]).unwrap().is_empty());
        assert!(PieMenu::resolve_level(&root, &[5]).is_none());
    }

    const CENTER: Pos2 = Pos2::new(400.0, 300.0);

    /// North opens a sub-pie holding a back button (South, opposite) and a leaf (North).
    fn nested() -> Vec<PieButton> {
        vec![
            PieButton::new(PieDirection::North).with_label("More").with_children(vec![
                PieButton::back(PieDirection::North.opposite()),
                PieButton::new(PieDirection::North).with_label("Leaf"),
            ]),
            PieButton::new(PieDirection::East).with_label("Plain"),
        ]
    }

    /// One frame at egui time `time`, with the pointer at `pointer` and `keys` pressed.
    fn frame(ctx: &egui::Context, m: &mut PieMenu, time: f64, pointer: Option<Pos2>, keys: &[Key]) -> PieMenuResponse {
        let buttons = nested();
        let events = keys
            .iter()
            .map(|&key| egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE })
            .collect();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            time: Some(time),
            events,
            ..Default::default()
        };
        let mut resp = PieMenuResponse::None;
        let mut out = ctx.run_ui(input, |ui| {
            resp = m.show_with_pointer(ui.ctx(), &buttons, pointer, true, |ui, b| {
                ui.label(b.label_text());
            });
        });
        out.textures_delta.clear();
        resp
    }

    fn opened(center: SubPieCenter) -> (egui::Context, PieMenu) {
        let mut m = PieMenu::new();
        m.settings.sub_pies.center = center;
        m.open(CENTER);
        m.reveal();
        let ctx = egui::Context::default();
        frame(&ctx, &mut m, 0.0, None, &[]); // lays the buttons out
        frame(&ctx, &mut m, 0.0, None, &[]);
        (ctx, m)
    }

    #[test]
    fn sub_pie_centre_follows_the_setting_and_back_restores_the_parent() {
        let at = Pos2::new(500.0, 100.0);
        let pointer = Some(at);
        for (mode, expect) in [(SubPieCenter::Pointer, at), (SubPieCenter::Parent, CENTER)] {
            let (ctx, mut m) = opened(mode);
            assert_eq!(frame(&ctx, &mut m, 0.0, pointer, &[Key::Num8]), PieMenuResponse::None);
            assert_eq!((m.path(), m.position), (&[0][..], expect), "{mode:?}");
        }

        let (ctx, mut m) = opened(SubPieCenter::Button);
        let trigger = m.button_centers[0];
        assert!((trigger.x - CENTER.x).abs() < 1.0 && trigger.y < CENTER.y - 100.0, "North button at {trigger:?}");
        frame(&ctx, &mut m, 0.0, pointer, &[Key::Num8]);
        assert_eq!((m.path(), m.position), (&[0][..], trigger));
        // Backspace goes back to the parent's own centre, not to the pointer.
        frame(&ctx, &mut m, 0.0, pointer, &[Key::Backspace]);
        assert!(m.path().is_empty());
        assert_eq!(m.position, CENTER);
    }

    #[test]
    fn back_button_returns_to_the_parent_without_a_selection() {
        let (ctx, mut m) = opened(SubPieCenter::Button);
        frame(&ctx, &mut m, 0.0, None, &[Key::Num8]); // into the sub-pie
        frame(&ctx, &mut m, 0.0, None, &[]);
        // South (2) is the back button: no `Selected`, the menu stays open at the parent.
        assert_eq!(frame(&ctx, &mut m, 0.0, None, &[Key::Num2]), PieMenuResponse::None);
        assert!(m.is_open() && m.path().is_empty());
        assert_eq!(m.position, CENTER);
    }

    #[test]
    fn back_button_at_the_root_is_an_ordinary_button() {
        let root = vec![PieButton::back(PieDirection::South)];
        assert!(root[0].is_back() && root[0].label == "Back");
        let ctx = egui::Context::default();
        let mut m = PieMenu::new();
        m.open(CENTER);
        m.reveal();
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: Key::Num2,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let mut resp = PieMenuResponse::None;
        let mut out = ctx.run_ui(input, |ui| {
            resp = m.show(ui.ctx(), &root, true, |_, _| {});
        });
        out.textures_delta.clear();
        assert_eq!(resp, PieMenuResponse::Selected(0));
    }

    #[test]
    fn hover_delay_opens_the_sub_pie_only_after_resting_on_it() {
        let above = Some(Pos2::new(400.0, 200.0)); // points at North ("More")
        let east = Some(Pos2::new(500.0, 300.0)); // points at East ("Plain")

        // Off by default: resting does nothing.
        let (ctx, mut m) = opened(SubPieCenter::Parent);
        frame(&ctx, &mut m, 1.0, above, &[]);
        frame(&ctx, &mut m, 9.0, above, &[]);
        assert!(m.path().is_empty());

        let (ctx, mut m) = opened(SubPieCenter::Parent);
        m.settings.sub_pies.hover_delay = Some(Duration::from_millis(500));
        frame(&ctx, &mut m, 1.0, above, &[]); // timer starts
        frame(&ctx, &mut m, 1.4, above, &[]);
        assert!(m.path().is_empty(), "not yet");
        // Leaving the button resets the timer; a plain button never opens anything.
        frame(&ctx, &mut m, 1.45, east, &[]);
        frame(&ctx, &mut m, 3.0, east, &[]);
        assert!(m.path().is_empty());
        frame(&ctx, &mut m, 3.1, above, &[]);
        frame(&ctx, &mut m, 3.5, above, &[]);
        assert!(m.path().is_empty(), "the timer restarted");
        assert_eq!(frame(&ctx, &mut m, 3.7, above, &[]), PieMenuResponse::None);
        assert_eq!(m.path(), &[0], "opened after resting 0.5 s");
        // A leaf ("Leaf", also North) never auto-selects, however long it is hovered.
        frame(&ctx, &mut m, 9.0, above, &[]);
        assert!(m.is_open() && m.path() == [0]);
    }

    #[test]
    fn hover_delay_follows_a_back_button_but_not_the_one_arrived_on() {
        let above = Some(Pos2::new(400.0, 200.0)); // North: "More" at the root, "Leaf" inside
        let below = Some(Pos2::new(400.0, 400.0)); // South: the back button inside
        let (ctx, mut m) = opened(SubPieCenter::Parent);
        m.settings.sub_pies.hover_delay = Some(Duration::from_millis(500));

        // Entering by key with the pointer already on the back button's direction: it is
        // ignored while the pointer stays there, however long.
        frame(&ctx, &mut m, 1.0, below, &[Key::Num8]);
        assert_eq!(m.path(), &[0]);
        frame(&ctx, &mut m, 1.1, below, &[]);
        frame(&ctx, &mut m, 9.0, below, &[]);
        assert_eq!(m.path(), &[0], "arrived on the back button: no bounce");
        // Leaving it and coming back arms the timer.
        frame(&ctx, &mut m, 9.1, above, &[]);
        frame(&ctx, &mut m, 9.2, below, &[]);
        frame(&ctx, &mut m, 9.5, below, &[]);
        assert_eq!(m.path(), &[0], "not yet");
        assert_eq!(frame(&ctx, &mut m, 9.8, below, &[]), PieMenuResponse::None);
        assert!(m.is_open() && m.path().is_empty(), "back at the parent");
        // At the root nothing is hovered-through: South has no button, and the pointer now
        // rests on "More" (nearest), which it arrived on, so it does not reopen.
        frame(&ctx, &mut m, 9.9, below, &[]);
        frame(&ctx, &mut m, 20.0, below, &[]);
        assert!(m.path().is_empty());
    }

    #[test]
    fn hover_delay_flows_through_nested_sub_pies_without_moving_but_never_bounces_back() {
        // North opens "More", whose North button opens "Deeper", whose North button is a leaf.
        let buttons = vec![PieButton::new(PieDirection::North).with_label("More").with_children(vec![
            PieButton::back(PieDirection::South),
            PieButton::new(PieDirection::North).with_label("Deeper").with_children(vec![
                PieButton::back(PieDirection::South),
                PieButton::new(PieDirection::North).with_label("Leaf"),
            ]),
        ])];
        let above = Some(Pos2::new(400.0, 200.0));
        let mut m = PieMenu::new();
        m.settings.sub_pies.center = SubPieCenter::Parent;
        m.settings.sub_pies.hover_delay = Some(Duration::from_millis(500));
        m.open(CENTER);
        m.reveal();
        let ctx = egui::Context::default();
        let step = |m: &mut PieMenu, time: f64, keys: &[Key]| {
            let pointer = if time < 0.0 { None } else { above };
            let events = keys
                .iter()
                .map(|&key| egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE })
                .collect();
            let input = egui::RawInput { time: Some(time.abs()), events, ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| {
                m.show_with_pointer(ui.ctx(), &buttons, pointer, true, |ui, b| {
                    ui.label(b.label_text());
                });
            });
            out.textures_delta.clear();
        };
        // The menu opens with the pointer at the centre (a negative time means "no pointer")
        // and then moves out to "More"; it never moves again after that.
        step(&mut m, -0.0001, &[]);
        step(&mut m, -0.0002, &[]);
        for t in [1.0, 1.1, 1.4] {
            step(&mut m, t, &[]);
        }
        assert!(m.path().is_empty(), "not yet");
        step(&mut m, 1.6, &[]);
        assert_eq!(m.path(), &[0]);
        // … and, still not moving, the sub-pie button now under it opens too, a delay later.
        step(&mut m, 1.7, &[]);
        step(&mut m, 2.1, &[]);
        assert_eq!(m.path(), &[0], "the timer runs from arrival");
        step(&mut m, 2.3, &[]);
        assert_eq!(m.path(), &[0, 1], "flowed on without moving away");
        // The leaf under the pointer is never selected by resting.
        step(&mut m, 9.0, &[]);
        assert!(m.is_open() && m.path() == [0, 1]);
        // Going back: the button the pointer rests on is ignored until it leaves.
        step(&mut m, 9.1, &[Key::Backspace]);
        step(&mut m, 9.2, &[]);
        step(&mut m, 20.0, &[]);
        assert_eq!(m.path(), &[0], "no bounce back into \"Deeper\"");
    }

    #[test]
    fn opposite_directions() {
        for d in PieDirection::ALL {
            assert_ne!(d.opposite(), d);
            assert_eq!(d.opposite().opposite(), d);
        }
        assert_eq!(PieDirection::NorthEast.opposite(), PieDirection::SouthWest);
    }

    /// The buttons changed under an open sub-pie so its level no longer exists (or is
    /// empty): the menu shows the root level again instead of an empty pie.
    #[test]
    fn stale_or_empty_sub_level_falls_back_to_the_root() {
        let ctx = egui::Context::default();
        let leaf_only = vec![PieButton::new(PieDirection::North), PieButton::new(PieDirection::South)];
        for stale in [vec![0], vec![5], vec![0, 1]] {
            let mut m = PieMenu::new();
            m.open(Pos2::new(100.0, 100.0));
            m.reveal();
            m.path = stale;
            let mut drawn = 0;
            let mut out = ctx.run_ui(Default::default(), |ui| {
                m.show(ui.ctx(), &leaf_only, true, |ui, _| {
                    drawn += 1;
                    ui.label("x");
                });
            });
            out.textures_delta.clear();
            assert_eq!(drawn, 2, "the root's buttons are drawn");
            assert!(m.path().is_empty());
        }
    }

    #[test]
    fn restart_level_recentres_and_shows() {
        let mut m = PieMenu::new();
        m.restart_level(Some(Pos2::new(10.0, 20.0)), false);
        assert_eq!(m.position, Pos2::new(10.0, 20.0));
        assert!(m.visible && m.release_handled);
        m.restart_level(None, true);
        assert_eq!(m.position, Pos2::new(10.0, 20.0));
        assert!(!m.release_handled);
    }
}

mod finding_tests {
    use super::*;

    fn north() -> PieButton {
        PieButton::new(PieDirection::North).with_label("Copy")
    }

    #[test]
    fn hint_digit_styles() {
        let mut m = PieMenu::new();
        let n = PieDirection::North; // number key 8
        assert_eq!(m.hint_digit(&n), '8');
        m.settings.number_hints.digits = HintDigits::ArabicIndic;
        assert_eq!(m.hint_digit(&n), '٨');
        m.settings.number_hints.digits = HintDigits::Persian;
        assert_eq!(m.hint_digit(&n), '۸');
    }

    #[test]
    fn text_colours_follow_egui_widget_states() {
        let v = egui::Visuals::dark();
        let (inactive, hovered) = (v.widgets.inactive.fg_stroke.color, v.widgets.hovered.fg_stroke.color);
        assert_ne!(inactive, hovered, "the theme distinguishes the states");
        assert_eq!(button_text_color(None, &v, false, true), inactive);
        assert_eq!(button_text_color(None, &v, true, true), hovered);
        // Forced (Blender-style): the same colour whether hovered or not.
        for h in [false, true] {
            assert_eq!(button_text_color(Some(Color32::WHITE), &v, h, true), Color32::WHITE);
        }
        // Disabled: egui's grey-out, also for a forced colour, hovered or not.
        for h in [false, true] {
            assert_eq!(button_text_color(None, &v, h, false), v.gray_out(inactive));
            assert_eq!(button_text_color(Some(Color32::WHITE), &v, h, false), v.gray_out(Color32::WHITE));
        }
        assert_ne!(v.gray_out(Color32::WHITE), Color32::WHITE);
    }

    #[test]
    fn label_and_digit_colours_are_independent() {
        let mut m = PieMenu::new();
        let v = egui::Visuals::dark();
        m.settings.text_color = Some(Color32::WHITE); // labels forced …
        // … digits still follow egui's states.
        assert_eq!(m.number_hint_styled(&north(), false, &v).unwrap().color, v.widgets.inactive.fg_stroke.color);
        assert_eq!(m.number_hint_styled(&north(), true, &v).unwrap().color, v.widgets.hovered.fg_stroke.color);
        m.settings.number_hints.color = Some(Color32::RED);
        assert_eq!(m.number_hint_styled(&north(), true, &v).unwrap().color, Color32::RED);
        assert_eq!(m.number_hint_styled(&north(), false, &v).unwrap().color, Color32::RED);
    }

    #[test]
    fn persian_and_arabic_yeh_kaf_fold_together() {
        assert_eq!(fold_char('ی'), fold_char('ي'));
        assert_eq!(fold_char('ى'), fold_char('ي'));
        assert_eq!(fold_char('ک'), fold_char('ك'));
    }

    /// Runs one frame: a focused-or-not text field drawn first (like the demo's
    /// sidebar), then the menu. Returns the menu response and the field's text.
    fn frame(
        ctx: &egui::Context,
        m: &mut PieMenu,
        text: &mut String,
        focus: bool,
        events: Vec<egui::Event>,
    ) -> PieMenuResponse {
        let buttons = vec![north()];
        let mut resp = PieMenuResponse::None;
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            let r = ui.text_edit_singleline(text);
            if focus {
                r.request_focus();
            }
            resp = m.show_with_pointer(ui.ctx(), &buttons, None, true, |ui, _| {
                ui.label("x");
            });
        });
        out.textures_delta.clear();
        resp
    }

    fn digit8() -> Vec<egui::Event> {
        vec![
            egui::Event::Key {
                key: Key::Num8,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::Text("8".into()),
        ]
    }

    #[test]
    fn blocking_menu_takes_focus_from_text_field() {
        let ctx = egui::Context::default();
        let mut m = PieMenu::new();
        let mut text = String::new();
        m.open(Pos2::new(400.0, 300.0));
        m.reveal();
        frame(&ctx, &mut m, &mut text, true, vec![]); // field focused, menu opens
        assert!(ctx.memory(|mem| mem.focused().is_none()), "menu should clear focus");
        let resp = frame(&ctx, &mut m, &mut text, false, digit8());
        assert_eq!(resp, PieMenuResponse::Selected(0));
        assert!(text.is_empty(), "digit leaked into the text field: {text:?}");
    }

    #[test]
    fn non_blocking_menu_leaves_typing_to_focused_field() {
        let ctx = egui::Context::default();
        let mut m = PieMenu::new();
        m.settings.block_background = false;
        let mut text = String::new();
        m.open(Pos2::new(400.0, 300.0));
        m.reveal();
        frame(&ctx, &mut m, &mut text, true, vec![]);
        let resp = frame(&ctx, &mut m, &mut text, true, digit8());
        assert_eq!(resp, PieMenuResponse::None, "typing selected a pie button");
        assert_eq!(text, "8");
        // A dismiss key typed into the field doesn't dismiss the menu either.
        let five = vec![
            egui::Event::Key {
                key: Key::Num5,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::Text("5".into()),
        ];
        let resp = frame(&ctx, &mut m, &mut text, true, five);
        assert_eq!(resp, PieMenuResponse::None, "typing 5 dismissed the menu");
        assert_eq!(text, "85");
        // Without a focused field the key selects as before.
        ctx.memory_mut(|mem| mem.stop_text_input());
        let resp = frame(&ctx, &mut m, &mut text, false, digit8());
        assert_eq!(resp, PieMenuResponse::Selected(0));
    }
}

mod input_tests {
    use super::*;

    fn key(key: Key, modifiers: egui::Modifiers) -> Vec<egui::Event> {
        vec![egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }]
    }

    /// One frame of a revealed menu with a single "Copy" button (mnemonic `c`).
    fn frame(ctx: &egui::Context, m: &mut PieMenu, events: Vec<egui::Event>, title: Option<&str>) -> PieMenuResponse {
        let buttons = vec![PieButton::new(PieDirection::North).with_label("Copy").with_mnemonic('c')];
        let mut resp = PieMenuResponse::None;
        m.title = title.map(String::from);
        let mut out = ctx.run_ui(egui::RawInput { events, ..Default::default() }, |ui| {
            resp = m.show_with_pointer(ui.ctx(), &buttons, None, true, |ui, b| {
                ui.label(b.label_text());
            });
        });
        out.textures_delta.clear();
        resp
    }

    fn revealed() -> PieMenu {
        let mut m = PieMenu::new();
        m.open(Pos2::new(100.0, 100.0));
        m.reveal();
        m
    }

    #[test]
    fn mnemonic_matches_by_key_code_but_not_in_a_ctrl_chord() {
        let ctx = egui::Context::default();
        let mut m = revealed();
        assert_eq!(frame(&ctx, &mut m, key(Key::C, egui::Modifiers::CTRL), None), PieMenuResponse::None);
        assert_eq!(frame(&ctx, &mut m, key(Key::C, egui::Modifiers::NONE), None), PieMenuResponse::Selected(0));
    }

    #[test]
    fn dismiss_key_is_consumed_and_ignores_ctrl_chords() {
        let ctx = egui::Context::default();
        let mut m = revealed();
        assert_eq!(frame(&ctx, &mut m, key(Key::Escape, egui::Modifiers::CTRL), None), PieMenuResponse::None);
        assert_eq!(frame(&ctx, &mut m, key(Key::Escape, egui::Modifiers::NONE), None), PieMenuResponse::Dismissed);
    }

    #[test]
    fn the_apps_label_style_is_never_touched() {
        for app_setting in [true, false] {
            let ctx = egui::Context::default();
            ctx.all_styles_mut(|s| s.interaction.selectable_labels = app_setting);
            let mut m = revealed();
            for _ in 0..2 {
                frame(&ctx, &mut m, vec![], None);
                assert_eq!(ctx.global_style().interaction.selectable_labels, app_setting);
            }
        }
    }

    #[test]
    fn title_is_drawn_without_a_background_circle() {
        let ctx = egui::Context::default();
        let mut m = revealed();
        m.settings.center_indicator.background_radius = None;
        m.title = Some("Edit".into());
        fn title_pos(shape: &egui::Shape) -> Option<Pos2> {
            match shape {
                egui::Shape::Text(t) if t.galley.text() == "Edit" => Some(t.pos),
                egui::Shape::Vec(v) => v.iter().find_map(title_pos),
                _ => None,
            }
        }
        let buttons = vec![PieButton::new(PieDirection::North)];
        let mut out = ctx.run_ui(Default::default(), |ui| {
            m.show_with_pointer(ui.ctx(), &buttons, None, true, |ui, _| {
                ui.label("x");
            });
        });
        out.textures_delta.clear();
        let pos = out.shapes.iter().find_map(|s| title_pos(&s.shape)).expect("title painted");
        assert!(pos.x.is_finite() && pos.y.is_finite(), "title at {pos:?}");
    }

    #[test]
    fn double_tap_uses_egui_time() {
        let ctx = egui::Context::default();
        let buttons = vec![PieButton::new(PieDirection::North)];
        let mut m = PieMenu::new();
        let tap = |m: &mut PieMenu, time: f64| {
            m.open(Pos2::new(100.0, 100.0));
            let mut resp = PieMenuResponse::None;
            let input = egui::RawInput { time: Some(time), ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| {
                resp = m.show_with_pointer(ui.ctx(), &buttons, Some(Pos2::new(100.0, 100.0)), false, |_, _| {});
            });
            out.textures_delta.clear();
            resp
        };
        assert_eq!(tap(&mut m, 1.0), PieMenuResponse::QuickTap);
        assert_eq!(tap(&mut m, 1.2), PieMenuResponse::DoubleTap); // within the 400 ms window
        assert_eq!(tap(&mut m, 2.0), PieMenuResponse::QuickTap);
        assert_eq!(tap(&mut m, 3.0), PieMenuResponse::QuickTap); // too late: a new first tap
    }
}

mod disabled_fade_tests {
    use super::*;

    /// The label colour reaches the painted text: forced white on an enabled button,
    /// egui's grey-out of it on a disabled one (egui's disabled `Ui` alone would leave an
    /// explicitly coloured label untouched).
    #[test]
    fn disabled_button_text_is_greyed_out() {
        let painted = |enabled: bool| {
            let ctx = egui::Context::default();
            let buttons = vec![PieButton::new(PieDirection::North).with_label("Cut").with_enabled(enabled)];
            let mut m = PieMenu::new();
            m.settings.text_color = Some(Color32::WHITE);
            m.open(Pos2::new(400.0, 300.0));
            m.reveal();
            fn find(shape: &egui::Shape) -> Option<Color32> {
                match shape {
                    egui::Shape::Text(t) if t.galley.text() == "Cut" => t.galley.job.sections.first().map(|s| s.format.color),
                    egui::Shape::Vec(v) => v.iter().find_map(find),
                    _ => None,
                }
            }
            // New Areas skip painting on their first (sizing) frame.
            let mut found = None;
            for _ in 0..3 {
                let mut out = ctx.run_ui(Default::default(), |ui| {
                    m.show_with_pointer(ui.ctx(), &buttons, None, true, |ui, b| {
                        ui.label(b.label_text());
                    });
                });
                out.textures_delta.clear();
                found = out.shapes.iter().find_map(|s| find(&s.shape)).or(found);
            }
            (found.expect("label painted"), ctx.global_style().visuals.clone())
        };
        let (on, _) = painted(true);
        let (off, v) = painted(false);
        assert_eq!(on, Color32::WHITE);
        assert_eq!(off, v.gray_out(Color32::WHITE));
    }
}

mod fan_out_tests {
    use super::*;

    const CENTER: Pos2 = Pos2::new(400.0, 300.0);

    /// East opens a fan-out of three buttons (North, East, South); North is a plain button.
    fn buttons() -> Vec<PieButton> {
        vec![
            PieButton::new(PieDirection::East).with_label("More").with_fan_out(vec![
                PieButton::new(PieDirection::North).with_label("Above"),
                PieButton::new(PieDirection::East).with_label("Beyond"),
                PieButton::new(PieDirection::South).with_label("Under"),
            ]),
            PieButton::new(PieDirection::North).with_label("Plain"),
        ]
    }

    /// What one frame drew: each button's path and whether it was shown hovered.
    type Drawn = Vec<(Vec<usize>, bool)>;

    struct Rig {
        ctx: egui::Context,
        menu: PieMenu,
        buttons: Vec<PieButton>,
        time: f64,
        screen: Vec2,
        /// How soon the last frame asked to be repainted.
        repaint_delay: Duration,
        /// Per button the last frame drew: its path and the `ui.opacity()` it was given.
        opacities: Vec<(Vec<usize>, f32)>,
        /// What the last frame painted.
        shapes: Vec<egui::epaint::ClippedShape>,
        /// Per button the last frame drew: its path and `b.scale()`.
        scales: Vec<(Vec<usize>, f32)>,
        /// When set, the closure overrides the label font with it.
        font_override: Option<egui::FontId>,
    }

    impl Rig {
        fn new(buttons: Vec<PieButton>) -> Self {
            Self::with_screen(buttons, Vec2::new(800.0, 600.0))
        }

        fn with_screen(buttons: Vec<PieButton>, screen: Vec2) -> Self {
            let mut menu = PieMenu::new();
            menu.settings.animations.reveal = RevealAnimation::None;
            menu.open(CENTER);
            menu.reveal();
            let mut rig = Self { ctx: egui::Context::default(), menu, buttons, time: 0.0, screen, repaint_delay: Duration::MAX, opacities: Vec::new(), shapes: Vec::new(), scales: Vec::new(), font_override: None };
            rig.frame(None, true, &[]); // lays the buttons out
            rig.frame(None, true, &[]);
            rig
        }

        fn frame(&mut self, pointer: Option<Pos2>, key_down: bool, keys: &[Key]) -> (PieMenuResponse, Drawn) {
            let events = keys
                .iter()
                .map(|&key| egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE })
                .collect();
            self.frame_with(pointer, key_down, events)
        }

        fn frame_with(&mut self, pointer: Option<Pos2>, key_down: bool, events: Vec<egui::Event>) -> (PieMenuResponse, Drawn) {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.screen)),
                time: Some(self.time),
                events,
                ..Default::default()
            };
            let (mut resp, mut drawn) = (PieMenuResponse::None, Drawn::new());
            let mut opacities = Vec::new();
            let mut scales = Vec::new();
            let font_override = self.font_override.clone();
            let (menu, buttons) = (&mut self.menu, &self.buttons);
            let mut out = self.ctx.run_ui(input, |ui| {
                resp = menu.show_with_pointer(ui.ctx(), buttons, pointer, key_down, |ui, b| {
                    drawn.push((b.path.to_vec(), b.hovered));
                    opacities.push((b.path.to_vec(), ui.opacity()));
                    scales.push((b.path.to_vec(), b.scale()));
                    let label = match &font_override {
                        Some(f) => b.label_text().font(f.clone()),
                        None => b.label_text(),
                    };
                    ui.label(label);
                });
            });
            out.textures_delta.clear();
            self.scales = scales;
            self.opacities = opacities;
            self.shapes = std::mem::take(&mut out.shapes);
            self.repaint_delay = out.viewport_output.get(&egui::ViewportId::ROOT).map_or(Duration::MAX, |v| v.repaint_delay);
            (resp, drawn)
        }

        /// Moves the pointer there and lets the menu settle (open and lay out a fan).
        fn point(&mut self, pointer: Pos2) -> Drawn {
            self.frame(Some(pointer), true, &[]);
            self.frame(Some(pointer), true, &[]);
            self.frame(Some(pointer), true, &[]).1
        }

        /// Centre of the fan's opening button ("More", East).
        fn trigger(&self) -> Pos2 {
            self.menu.button_centers[0]
        }
    }

    fn paths(drawn: &Drawn) -> Vec<Vec<usize>> {
        drawn.iter().map(|(p, _)| p.clone()).collect()
    }

    #[test]
    fn opens_when_the_pointer_reaches_the_button_and_draws_both_rings() {
        let mut rig = Rig::new(buttons());
        // Pointing east but still well inside the ring: no fan yet.
        let drawn = rig.point(Pos2::new(450.0, 300.0));
        assert_eq!(rig.menu.open_fan_out(), None);
        assert_eq!(paths(&drawn), [vec![0], vec![1]]);
        // Out at the ring: the fan opens, and the parent is still drawn.
        let drawn = rig.point(Pos2::new(505.0, 300.0));
        assert_eq!(rig.menu.open_fan_out(), Some((0, None)), "on the opening button nothing in the fan is hovered");
        assert_eq!(paths(&drawn), [vec![0], vec![1], vec![0, 0], vec![0, 1], vec![0, 2]]);
        assert_eq!(rig.menu.hovered(), Some(0));
        assert!(rig.menu.path().is_empty(), "a fan-out is not a level of its own");
    }

    #[test]
    fn pointer_picks_fan_children_and_release_selects_with_the_full_path() {
        let mut rig = Rig::new(buttons());
        rig.point(Pos2::new(505.0, 300.0));
        let fan_center = rig.trigger();
        let drawn = rig.point(fan_center + Vec2::new(0.0, -80.0));
        assert_eq!(rig.menu.open_fan_out(), Some((0, Some(0))), "north of the fan centre: Above");
        // The fan child is drawn hovered, the opening button too, the other parent button never.
        let hovered: Vec<_> = drawn.iter().filter(|(_, h)| *h).map(|(p, _)| p.clone()).collect();
        assert_eq!(hovered, [vec![0], vec![0, 0]]);
        rig.point(fan_center + Vec2::new(120.0, 0.0));
        assert_eq!(rig.menu.open_fan_out(), Some((0, Some(1))), "east of the fan centre: Beyond");
        let (resp, _) = rig.frame(Some(fan_center + Vec2::new(120.0, 0.0)), false, &[]);
        assert_eq!(resp, PieMenuResponse::Selected(1));
        assert_eq!(rig.menu.selected_path(), &[0, 1]);
        assert!(!rig.menu.is_open());
    }

    /// The layer transform of button `idx` of a ring whose Area ids are salted with `ring`.
    fn transform(rig: &Rig, ring: egui::Id, idx: usize) -> egui::emath::TSTransform {
        let layer = egui::LayerId::new(egui::Order::Tooltip, ring.with(idx));
        rig.ctx.layer_transform_to_global(layer).unwrap_or_default()
    }

    /// `b.scale()` of the button at `path` in the last frame.
    fn scale_of(rig: &Rig, path: &[usize]) -> f32 {
        rig.scales.iter().find(|(p, _)| p == path).unwrap_or_else(|| panic!("{path:?} not drawn")).1
    }

    /// The font size the label `text` was painted with in the last frame.
    fn painted_font_size(rig: &Rig, text: &str) -> f32 {
        fn find(shape: &egui::Shape, text: &str) -> Option<f32> {
            match shape {
                egui::Shape::Text(t) if t.galley.text() == text => t.galley.job.sections.first().map(|s| s.format.font_id.size),
                egui::Shape::Vec(v) => v.iter().find_map(|s| find(s, text)),
                _ => None,
            }
        }
        rig.shapes.iter().find_map(|s| find(&s.shape, text)).unwrap_or_else(|| panic!("{text:?} not painted"))
    }

    fn body_size(rig: &Rig) -> f32 {
        rig.ctx.global_style().text_styles[&egui::TextStyle::Body].size
    }

    #[test]
    fn highlighted_button_is_emphasised_with_separate_amounts_for_root_and_sub_pies() {
        use egui::emath::TSTransform;
        let hover_fan_child = |root: HoverEmphasis, sub: Option<HoverEmphasis>| {
            let mut rig = Rig::new(buttons());
            rig.menu.settings.animations.hover_emphasis = root;
            if let Some(sub) = sub {
                rig.menu.settings.animations.sub_pie_hover_emphasis = sub;
            }
            rig.point(Pos2::new(505.0, 300.0));
            let above = rig.trigger() + Vec2::new(0.0, -80.0); // the fan's "Above" (child 0)
            for _ in 0..5 {
                rig.time += 0.1;
                rig.frame(Some(above), true, &[]);
            }
            rig
        };
        let fan_ring = |rig: &Rig| rig.menu.id.with(("fan", 0usize));

        // Defaults: only the fan's highlighted button moves (outward: up) and grows.
        let rig = hover_fan_child(HoverEmphasis::NONE, None);
        let lifted = transform(&rig, fan_ring(&rig), 0);
        assert_eq!(lifted.scaling, 1.0, "the layer is never scaled: {lifted:?}");
        assert!((lifted.translation - Vec2::new(0.0, -6.0)).length() < 1e-3, "moves 6 px outward: {lifted:?}");
        assert!((scale_of(&rig, &[0, 0]) - 1.08).abs() < 1e-4, "the content grows instead");
        assert!((painted_font_size(&rig, "Above") - body_size(&rig) * 1.08).abs() < 1e-3);
        assert_eq!(scale_of(&rig, &[0, 1]), 1.0, "other fan buttons stay");
        assert_eq!(transform(&rig, fan_ring(&rig), 1), TSTransform::IDENTITY, "other fan buttons stay");
        assert_eq!(transform(&rig, rig.menu.id, 0), TSTransform::IDENTITY, "the root's lit button stays");

        // Sub-pie amount off: nothing moves in the fan.
        let rig = hover_fan_child(HoverEmphasis::NONE, Some(HoverEmphasis::NONE));
        assert_eq!(transform(&rig, fan_ring(&rig), 0), TSTransform::IDENTITY);
        assert_eq!(scale_of(&rig, &[0, 0]), 1.0);
        assert_eq!(painted_font_size(&rig, "Above"), body_size(&rig));

        // The root has its own amount: here it only moves, 10 px east, without growing.
        let rig = hover_fan_child(HoverEmphasis { shift: 10.0, scale: 1.0 }, None);
        let root = transform(&rig, rig.menu.id, 0);
        assert!(root.scaling == 1.0 && (root.translation - Vec2::new(10.0, 0.0)).length() < 1e-3, "{root:?}");
        assert_eq!(scale_of(&rig, &[0]), 1.0, "the root does not grow");
        assert!((scale_of(&rig, &[0, 0]) - 1.08).abs() < 1e-4, "the fan keeps its own");

        // It eases back to the identity once the pointer leaves (layer transforms are sticky).
        let mut rig = hover_fan_child(HoverEmphasis::NONE, None);
        let beyond = rig.trigger() + Vec2::new(120.0, 0.0);
        for _ in 0..5 {
            rig.time += 0.1;
            rig.frame(Some(beyond), true, &[]);
        }
        assert_eq!(transform(&rig, fan_ring(&rig), 0), TSTransform::IDENTITY);
        assert_eq!(scale_of(&rig, &[0, 0]), 1.0);
    }

    /// Layers are only ever translated (a layer scale would blur the glyph images), by
    /// whole physical pixels, in every state and at fractional pixels-per-point too.
    #[test]
    fn layer_transform_is_a_whole_pixel_translation_in_every_state() {
        for ppp in [1.0f32, 1.5] {
            let mut rig = Rig::new(buttons());
            rig.ctx.set_pixels_per_point(ppp);
            rig.menu.settings.animations.hover_emphasis = HoverEmphasis { shift: 4.3, scale: 1.2 };
            let check = |rig: &Rig, what: &str| {
                let rings = [rig.menu.id, rig.menu.id.with(("fan", 0usize))];
                for ring in rings {
                    for idx in 0..3 {
                        let t = transform(rig, ring, idx);
                        assert_eq!(t.scaling, 1.0, "{what} (ppp {ppp}): {t:?}");
                        for v in [t.translation.x, t.translation.y] {
                            let px = v * ppp;
                            assert!((px - px.round()).abs() < 1e-4, "{what} (ppp {ppp}): {t:?}");
                        }
                    }
                }
            };
            rig.point(Pos2::new(505.0, 300.0));
            check(&rig, "idle");
            let above = rig.trigger() + Vec2::new(0.0, -80.0);
            let mut moved = false;
            for step in 0..8 {
                rig.time += 0.03;
                rig.frame(Some(above), true, &[]);
                moved |= transform(&rig, rig.menu.id.with(("fan", 0usize)), 0) != egui::emath::TSTransform::IDENTITY;
                check(&rig, &format!("easing {step}"));
            }
            assert!(moved, "the highlighted button was shifted");
            for _ in 0..5 {
                rig.time += 0.1;
                rig.frame(Some(above), true, &[]);
            }
            check(&rig, "fully emphasised");
            let beyond = rig.trigger() + Vec2::new(120.0, 0.0);
            for step in 0..8 {
                rig.time += 0.03;
                rig.frame(Some(beyond), true, &[]);
                check(&rig, &format!("easing back {step}"));
            }
            for _ in 0..5 {
                rig.time += 0.1;
                rig.frame(Some(beyond), true, &[]);
            }
            check(&rig, "back to idle");
        }
    }

    /// The scale reaches the content: `b.scale()` and the painted label font size follow
    /// the eased emphasis; neighbours stay at 1.0; no emphasis never changes anything.
    #[test]
    fn label_font_size_follows_the_button_scale() {
        let settle = |rig: &mut Rig, at: Pos2| {
            for _ in 0..5 {
                rig.time += 0.1;
                rig.frame(Some(at), true, &[]);
            }
        };
        let mut rig = Rig::new(buttons());
        rig.point(Pos2::new(505.0, 300.0));
        let body = body_size(&rig);
        assert_eq!(scale_of(&rig, &[0, 0]), 1.0);
        assert_eq!(painted_font_size(&rig, "Above"), body);
        let above = rig.trigger() + Vec2::new(0.0, -80.0);
        settle(&mut rig, above);
        assert!((scale_of(&rig, &[0, 0]) - 1.08).abs() < 1e-5);
        assert!((painted_font_size(&rig, "Above") - body * 1.08).abs() < 1e-4);
        for (path, label) in [(vec![0, 1], "Beyond"), (vec![0, 2], "Under")] {
            assert_eq!(scale_of(&rig, &path), 1.0);
            assert_eq!(painted_font_size(&rig, label), body);
        }
        // Easing: somewhere strictly between.
        let beyond = rig.trigger() + Vec2::new(120.0, 0.0);
        rig.time += 0.03;
        rig.frame(Some(beyond), true, &[]);
        let mid = scale_of(&rig, &[0, 0]);
        assert!(mid > 1.0 && mid < 1.08, "{mid}");
        settle(&mut rig, beyond);
        assert_eq!(scale_of(&rig, &[0, 0]), 1.0);
        assert_eq!(painted_font_size(&rig, "Above"), body);

        // No emphasis at all: never changes.
        let mut rig = Rig::new(buttons());
        rig.menu.settings.animations.sub_pie_hover_emphasis = HoverEmphasis::NONE;
        rig.point(Pos2::new(505.0, 300.0));
        let above = rig.trigger() + Vec2::new(0.0, -80.0);
        settle(&mut rig, above);
        assert!(rig.scales.iter().all(|(_, s)| *s == 1.0));
        assert_eq!(painted_font_size(&rig, "Above"), body);
    }

    /// A caller's `.font(..)` override wins, whatever the emphasis.
    #[test]
    fn font_override_wins_over_the_emphasis_scale() {
        let mut rig = Rig::new(buttons());
        rig.font_override = Some(egui::FontId::monospace(20.0));
        rig.point(Pos2::new(505.0, 300.0));
        let above = rig.trigger() + Vec2::new(0.0, -80.0);
        for _ in 0..5 {
            rig.time += 0.1;
            rig.frame(Some(above), true, &[]);
        }
        assert!((scale_of(&rig, &[0, 0]) - 1.08).abs() < 1e-5, "emphasised");
        assert_eq!(painted_font_size(&rig, "Above"), 20.0);
    }

    /// A replace-style sub-pie level uses the sub-pie amount, the root level the root's.
    #[test]
    fn replace_style_sub_pie_uses_the_sub_pie_emphasis() {
        use egui::emath::TSTransform;
        let nested = vec![PieButton::new(PieDirection::East).with_children(vec![PieButton::new(PieDirection::North)])];
        let mut rig = Rig::new(nested);
        rig.menu.settings.sub_pies.center = SubPieCenter::Parent;
        let north = Pos2::new(400.0, 180.0);
        let hover = |rig: &mut Rig, at: Pos2| {
            for _ in 0..5 {
                rig.time += 0.1;
                rig.frame(Some(at), true, &[]);
            }
        };
        hover(&mut rig, Pos2::new(520.0, 300.0)); // root: East highlighted, no emphasis by default
        assert_eq!(transform(&rig, rig.menu.id, 0), TSTransform::IDENTITY);
        rig.frame(Some(north), true, &[Key::Num6]); // into the sub-pie
        hover(&mut rig, north);
        assert_eq!(rig.menu.path(), &[0]);
        let lit = transform(&rig, rig.menu.level_ring_id(&[0]), 0);
        assert_eq!(lit.scaling, 1.0, "{lit:?}");
        assert!((lit.translation - Vec2::new(0.0, -6.0)).length() < 1e-3, "{lit:?}");
        assert!((scale_of(&rig, &[0, 0]) - 1.08).abs() < 1e-4);
    }

    #[test]
    fn closes_when_the_pointer_returns_to_the_parent() {
        let mut rig = Rig::new(buttons());
        rig.point(Pos2::new(505.0, 300.0));
        assert!(rig.menu.open_fan_out().is_some());
        // Back inside the ring, still pointing east: the fan closes and does not reopen.
        let drawn = rig.point(Pos2::new(450.0, 300.0));
        assert_eq!(rig.menu.open_fan_out(), None);
        assert_eq!(paths(&drawn), [vec![0], vec![1]]);
        assert_eq!(rig.menu.hovered(), Some(0));
    }

    #[test]
    fn pointing_back_at_the_parent_from_the_fan_hovers_nothing() {
        let mut rig = Rig::new(buttons());
        rig.frame(None, true, &[Key::Num6]); // open by key: the pointer has not been out there
        let fan_center = rig.trigger();
        // West of the fan centre (towards the parent), outside the opening button itself.
        let toward_parent = fan_center - Vec2::new(60.0, 0.0);
        rig.point(toward_parent);
        assert_eq!(rig.menu.open_fan_out(), Some((0, None)), "the sector facing the parent picks nothing");
        // Just off that sector, the nearest fan child is picked as usual.
        rig.point(fan_center + Vec2::new(-10.0, -70.0));
        assert_eq!(rig.menu.open_fan_out(), Some((0, Some(0))));
    }

    #[test]
    fn open_fan_has_the_keys_to_itself() {
        let mut rig = Rig::new(buttons());
        // 6 (East) opens the fan rather than selecting anything.
        assert_eq!(rig.frame(None, true, &[Key::Num6]).0, PieMenuResponse::None);
        assert_eq!(rig.menu.open_fan_out(), Some((0, None)));
        rig.frame(None, true, &[]);
        // 8 is North: the fan's "Above", not the parent's "Plain".
        assert_eq!(rig.frame(None, true, &[Key::Num8]).0, PieMenuResponse::Selected(0));
        assert_eq!(rig.menu.selected_path(), &[0, 0]);

        // Mnemonics likewise: "p" is nobody's in the fan, so the parent's "Plain" is not selected.
        let mut rig = Rig::new(buttons());
        rig.menu.settings.input.auto_mnemonics = true;
        rig.frame(None, true, &[Key::Num6]);
        assert_eq!(rig.frame(None, true, &[Key::P]).0, PieMenuResponse::None);
        assert_eq!(rig.frame(None, true, &[Key::U]).0, PieMenuResponse::Selected(2), "Under");
        assert_eq!(rig.menu.selected_path(), &[0, 2]);
    }

    #[test]
    fn key_opened_fan_closes_when_pointing_at_another_parent_button_or_on_backspace() {
        let mut rig = Rig::new(buttons());
        rig.frame(None, true, &[Key::Num6]);
        // Pointing at the fan's own button from inside the ring keeps it open …
        rig.point(Pos2::new(450.0, 300.0));
        assert!(rig.menu.open_fan_out().is_some());
        // … pointing at another parent button closes it.
        rig.point(Pos2::new(400.0, 250.0));
        assert_eq!(rig.menu.open_fan_out(), None);
        assert_eq!(rig.menu.hovered(), Some(1));

        rig.frame(None, true, &[Key::Num6]);
        assert!(rig.menu.open_fan_out().is_some());
        assert_eq!(rig.frame(None, true, &[Key::Backspace]).0, PieMenuResponse::None);
        assert!(rig.menu.is_open() && rig.menu.open_fan_out().is_none());
    }

    #[test]
    fn delay_holds_the_fan_back() {
        let mut rig = Rig::new(buttons());
        rig.menu.settings.sub_pies.fan_out.delay = Duration::from_millis(500);
        let at_ring = Pos2::new(505.0, 300.0);
        rig.time = 1.0;
        rig.point(at_ring);
        rig.time = 1.4;
        rig.point(at_ring);
        assert_eq!(rig.menu.open_fan_out(), None, "not yet");
        rig.time = 1.6;
        rig.point(at_ring);
        assert!(rig.menu.open_fan_out().is_some());
    }

    #[test]
    fn menu_style_decides_unless_the_button_overrides_it() {
        let children = || vec![PieButton::new(PieDirection::North)];
        let follows = PieButton::new(PieDirection::East).with_children(children());
        let fan = PieButton::new(PieDirection::East).with_fan_out(children());
        let replace = follows.clone().with_sub_pie_style(Some(SubPieStyle::Replace));
        let mut m = PieMenu::new();
        assert!(!m.opens_fan(&follows) && m.opens_fan(&fan) && !m.opens_fan(&replace));
        m.settings.sub_pies.style = SubPieStyle::FanOut;
        assert!(m.opens_fan(&follows) && m.opens_fan(&fan) && !m.opens_fan(&replace));
        assert!(!m.opens_fan(&PieButton::new(PieDirection::East)), "no children, no fan");

        // A replace-style button in the same menu still replaces the level.
        let mut rig = Rig::new(vec![replace]);
        rig.frame(None, true, &[Key::Num6]);
        assert_eq!(rig.menu.path(), &[0]);
        assert_eq!(rig.menu.open_fan_out(), None);
    }

    #[test]
    fn back_button_in_a_fan_closes_it() {
        let mut rig = Rig::new(vec![PieButton::new(PieDirection::East).with_fan_out(vec![
            PieButton::back(PieDirection::North),
            PieButton::new(PieDirection::South),
        ])]);
        rig.frame(None, true, &[Key::Num6]);
        assert!(rig.menu.open_fan_out().is_some());
        assert_eq!(rig.frame(None, true, &[Key::Num8]).0, PieMenuResponse::None);
        assert!(rig.menu.is_open() && rig.menu.open_fan_out().is_none());
    }

    // ── Findings of the pre-release review ───────────────────────────────────────

    /// The ring (or a fan) not fitting the screen must not panic (`f32::clamp` asserts).
    #[test]
    fn menus_larger_than_the_screen_are_placed_without_panicking() {
        for screen in [Vec2::new(300.0, 600.0), Vec2::new(150.0, 150.0), Vec2::ZERO] {
            let mut rig = Rig::with_screen(buttons(), screen);
            rig.frame(None, true, &[Key::Num6]); // opens the fan, which cannot fit either
            rig.frame(None, true, &[]);
            rig.frame(None, true, &[]);
            assert!(rig.menu.open_fan_out().is_some(), "{screen:?}");
        }
        let mut rig = Rig::with_screen(Vec::new(), Vec2::new(10.0, 10.0));
        rig.frame(None, true, &[]);
        let mut rig = Rig::new(buttons());
        rig.menu.settings.layout.screen_margin = 250.0;
        rig.frame(None, true, &[]);
        rig.frame(None, true, &[Key::Num6]);
    }

    /// Resting on a sub-pie button for the delay changes level: that frame still draws
    /// buttons (no blank flash) and a repaint is asked for, so a still pointer does not
    /// leave the menu empty until the next input, and the new level's timer gets going.
    #[test]
    fn hover_timer_level_change_draws_and_schedules_the_next_frame() {
        let nested = vec![PieButton::new(PieDirection::North).with_children(vec![
            PieButton::new(PieDirection::North).with_label("Deeper").with_children(vec![PieButton::new(PieDirection::North)]),
        ])];
        let mut rig = Rig::new(nested);
        rig.menu.settings.sub_pies.center = SubPieCenter::Parent;
        rig.menu.settings.sub_pies.hover_delay = Some(Duration::from_millis(500));
        let above = Some(Pos2::new(400.0, 200.0));
        rig.time = 1.0;
        rig.frame(above, true, &[]); // the timer starts
        rig.time = 1.2;
        rig.frame(above, true, &[]);
        rig.time = 1.6;
        let (_, drawn) = rig.frame(above, true, &[]); // it expires: into the sub-pie
        assert_eq!(rig.menu.path(), &[0]);
        assert!(!drawn.is_empty(), "the frame that changes level still draws buttons");
        assert_eq!(rig.repaint_delay, Duration::ZERO, "a repaint is requested for the new level");
        rig.time = 1.7;
        let (_, drawn) = rig.frame(above, true, &[]);
        assert_eq!(paths(&drawn), [vec![0, 0]], "the sub-pie is drawn");
        // (New buttons have their own Areas: the first frames ask for repaints to size and
        // show them; look a couple later.)
        rig.time = 1.75;
        rig.frame(above, true, &[]);
        rig.time = 1.8;
        rig.frame(above, true, &[]);
        assert!(
            rig.repaint_delay > Duration::ZERO && rig.repaint_delay <= Duration::from_millis(500),
            "its own timer is scheduled without further input: {:?}",
            rig.repaint_delay
        );
    }

    /// A pinned or non-blocking menu leaves typing alone, Backspace included.
    #[test]
    fn backspace_belongs_to_a_focused_text_field_in_a_pinned_menu() {
        let ctx = egui::Context::default();
        let root = vec![PieButton::new(PieDirection::North).with_children(vec![PieButton::new(PieDirection::North)])];
        let mut m = PieMenu::new();
        m.settings.animations.reveal = RevealAnimation::None;
        m.pin(CENTER);
        let mut text = String::from("ab");
        let mut frame = |m: &mut PieMenu, focus: bool, keys: &[Key]| {
            let events = keys
                .iter()
                .map(|&key| egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE })
                .collect();
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                if focus {
                    let r = ui.text_edit_singleline(&mut text);
                    r.request_focus();
                }
                m.show_with_pointer(ui.ctx(), &root, None, true, |ui, _| {
                    ui.label("x");
                });
            });
            out.textures_delta.clear();
        };
        frame(&mut m, false, &[]);
        frame(&mut m, false, &[Key::Num8]); // into the sub-pie
        assert_eq!(m.path(), &[0]);
        frame(&mut m, true, &[]);
        frame(&mut m, true, &[]);
        frame(&mut m, true, &[Key::Backspace]);
        assert_eq!(m.path(), &[0], "Backspace went to the text field, not the menu");
        frame(&mut m, false, &[]);
        frame(&mut m, false, &[Key::Backspace]);
        assert!(m.path().is_empty(), "without a focused field Backspace goes back");
    }

    /// "Never" delays (`Duration::MAX`) must not panic converting to a repaint delay.
    #[test]
    fn huge_delays_do_not_panic() {
        let mut rig = Rig::new(buttons());
        rig.menu.settings.sub_pies.fan_out.delay = Duration::MAX;
        rig.menu.settings.sub_pies.hover_delay = Some(Duration::MAX);
        for t in [1.0, 2.0, 3.0] {
            rig.time = t;
            rig.point(Pos2::new(505.0, 300.0));
        }
        assert_eq!(rig.menu.open_fan_out(), None, "never opens");
    }

    /// Every fan child can be pointed at, also when `layout.intercardinal_height` moves the
    /// diagonal buttons (the blocked sector must not swallow an allowed child).
    #[test]
    fn every_fan_child_is_reachable_with_intercardinal_height() {
        for height in [0.5, 0.2] {
            let parent = PieDirection::NorthEast;
            let children: Vec<_> = parent
                .fan_out_slots()
                .iter()
                .map(|d| PieButton::new(*d).with_label(format!("{d:?}")))
                .collect();
            let mut rig = Rig::new(vec![PieButton::new(parent).with_label("More").with_fan_out(children)]);
            rig.menu.settings.layout.intercardinal_height = Some(height);
            rig.frame(None, true, &[Key::Num9]); // opens the fan by key
            rig.frame(None, true, &[]);
            assert!(rig.menu.open_fan_out().is_some());
            for i in 0..5 {
                let centre = rig.menu.fan_centers_for_test()[i];
                rig.point(centre);
                assert_eq!(rig.menu.open_fan_out(), Some((0, Some(i))), "height {height}: child {i} at {centre:?}");
            }
        }
    }

    /// The centre-click-to-go-back test uses the centre as drawn (screen-clamped), not the
    /// raw position the menu was opened at.
    #[test]
    fn clicking_the_drawn_centre_goes_back_when_the_menu_was_shifted_on_screen() {
        let root = vec![PieButton::new(PieDirection::East).with_children(vec![PieButton::new(PieDirection::North)])];
        let mut rig = Rig::new(root);
        rig.menu.open(Pos2::new(5.0, 5.0)); // too close to the corner: drawn shifted
        rig.menu.reveal();
        rig.frame(None, true, &[]);
        rig.frame(None, true, &[Key::Num6]);
        assert_eq!(rig.menu.path(), &[0]);
        rig.frame(None, true, &[]);
        rig.frame(None, true, &[]);
        let drawn_centre = rig.menu.display_center;
        assert!((drawn_centre - rig.menu.position).length() > 12.0, "the menu is drawn shifted: {drawn_centre:?}");
        let click = |pressed| egui::Event::PointerButton {
            pos: drawn_centre,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        rig.frame_with(Some(drawn_centre), true, vec![egui::Event::PointerMoved(drawn_centre), click(true), click(false)]);
        assert!(rig.menu.path().is_empty());
    }

    /// Animation state (button emphasis, fan dimming) does not leak from a closed menu
    /// into the next one, nor between levels.
    #[test]
    fn animation_state_is_fresh_after_reopening() {
        use egui::emath::TSTransform;
        let mut rig = Rig::new(buttons());
        rig.menu.settings.animations.hover_emphasis = HoverEmphasis { shift: 5.0, scale: 1.5 };
        let above = Pos2::new(400.0, 200.0); // the root's "Plain" (North, index 1)
        for _ in 0..5 {
            rig.time += 0.1;
            rig.frame(Some(above), true, &[]);
        }
        assert_ne!(transform(&rig, rig.menu.id, 1), TSTransform::IDENTITY, "emphasised while hovered");
        assert!(scale_of(&rig, &[1]) > 1.4);
        assert_eq!(rig.frame(Some(above), true, &[Key::Num8]).0, PieMenuResponse::Selected(1));
        rig.menu.open(CENTER);
        rig.menu.reveal();
        rig.frame(None, true, &[]);
        assert_eq!(transform(&rig, rig.menu.id, 1), TSTransform::IDENTITY, "a new menu starts un-emphasised");
        assert_eq!(scale_of(&rig, &[1]), 1.0);
        assert!(rig.menu.last_fan.is_none());
    }

    #[test]
    fn open_resets_how_the_last_level_was_entered() {
        let root = vec![PieButton::new(PieDirection::East).with_children(vec![PieButton::new(PieDirection::North)])];
        let mut rig = Rig::new(root);
        rig.frame(None, true, &[Key::Num6]);
        assert!(rig.menu.entered_forward);
        rig.menu.open(CENTER);
        assert!(!rig.menu.entered_forward, "a fresh menu treats a sub-pie button under the pointer as inert");
    }

    #[test]
    fn a_stale_path_fallback_also_closes_the_fan() {
        let mut rig = Rig::new(buttons());
        rig.menu.path = vec![5]; // no such sub-pie any more
        rig.menu.fan = Some(0);
        rig.frame(None, true, &[]);
        assert!(rig.menu.path().is_empty());
        assert_eq!(rig.menu.open_fan_out(), None);
    }

    #[test]
    fn a_selection_from_a_fan_in_a_pinned_menu_closes_the_fan_like_a_sub_pie_does() {
        let mut rig = Rig::new(buttons());
        rig.menu.pin(CENTER);
        rig.frame(None, true, &[Key::Num6]); // opens the fan
        assert!(rig.menu.open_fan_out().is_some());
        assert_eq!(rig.frame(None, true, &[Key::Num8]).0, PieMenuResponse::Selected(0));
        assert!(rig.menu.is_pinned());
        assert_eq!(rig.menu.open_fan_out(), None);
    }

    // ── Missing a target in a fan, or at the root ────────────────────────────────

    fn click_at(pos: Pos2) -> Vec<egui::Event> {
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        vec![egui::Event::PointerMoved(pos), button(true), button(false)]
    }

    /// A point beside the fan's opening button, up and back towards the parent: in the
    /// blocked sector, outside the button, and still outside the ring's inner area.
    const BLOCKED: Pos2 = Pos2::new(485.0, 255.0);

    #[test]
    fn clicking_the_centre_of_the_root_dismisses_but_not_a_pinned_menu() {
        // Root: dismisses.
        let mut rig = Rig::new(buttons());
        assert_eq!(rig.frame_with(Some(CENTER), true, click_at(CENTER)).0, PieMenuResponse::Dismissed);
        assert!(!rig.menu.is_open());

        // In a replace-style sub-pie the first centre click goes back, the second dismisses.
        let root = vec![PieButton::new(PieDirection::East).with_children(vec![PieButton::new(PieDirection::North)])];
        let mut rig = Rig::new(root);
        rig.frame(None, true, &[Key::Num6]);
        assert_eq!(rig.menu.path(), &[0]);
        rig.frame(None, true, &[]);
        rig.frame(None, true, &[]);
        assert_eq!(rig.frame_with(Some(CENTER), true, click_at(CENTER)).0, PieMenuResponse::None);
        assert!(rig.menu.path().is_empty() && rig.menu.is_open());
        rig.frame(Some(CENTER), true, &[]);
        assert_eq!(rig.frame_with(Some(CENTER), true, click_at(CENTER)).0, PieMenuResponse::Dismissed);
        assert!(!rig.menu.is_open());

        // Pinned: stays open.
        let mut rig = Rig::new(buttons());
        rig.menu.pin(CENTER);
        rig.frame(None, true, &[]);
        assert_eq!(rig.frame_with(Some(CENTER), true, click_at(CENTER)).0, PieMenuResponse::None);
        assert!(rig.menu.is_open());

        // Clicks off: nothing happens.
        let mut rig = Rig::new(buttons());
        rig.menu.settings.input.select_on_primary_click = false;
        assert_eq!(rig.frame_with(Some(CENTER), true, click_at(CENTER)).0, PieMenuResponse::None);
        assert!(rig.menu.is_open());
    }

    #[test]
    fn releasing_on_the_fans_button_breaks_it_out_but_releasing_on_nothing_dismisses() {
        let at_ring = Pos2::new(505.0, 300.0);
        let mut rig = Rig::new(buttons());
        rig.point(at_ring);
        let trigger = rig.trigger();
        rig.point(trigger);
        assert!(rig.menu.open_fan_out().is_some());
        // Key released on the opening button: the fan breaks out into a sub-pie.
        assert_eq!(rig.frame(Some(trigger), false, &[]).0, PieMenuResponse::None);
        assert!(rig.menu.is_open());
        assert_eq!(rig.menu.path(), &[0]);
        assert_eq!(rig.menu.open_fan_out(), None);

        // Key released over nothing (the blocked sector): cancel, as in the centre.
        let mut rig = Rig::new(buttons());
        rig.frame(None, true, &[Key::Num6]); // opens the fan by key
        rig.point(BLOCKED);
        assert_eq!(rig.menu.open_fan_out(), Some((0, None)), "nothing is highlighted there");
        assert_eq!(rig.frame(Some(BLOCKED), false, &[]).0, PieMenuResponse::Dismissed);
        assert!(!rig.menu.is_open());
    }

    #[test]
    fn clicking_on_nothing_closes_the_fan_and_it_stays_closed_under_the_resting_pointer() {
        let at_ring = Pos2::new(505.0, 300.0);
        let mut rig = Rig::new(buttons());
        rig.menu.settings.sub_pies.fan_out.opener_action = FanOpenerAction::Dismiss;
        rig.point(at_ring);
        let trigger = rig.trigger();
        assert!(rig.menu.open_fan_out().is_some());
        // Click on the opening button (with the Dismiss setting): the fan closes, the menu stays.
        assert_eq!(rig.frame_with(Some(trigger), true, click_at(trigger)).0, PieMenuResponse::None);
        assert!(rig.menu.is_open());
        assert_eq!(rig.menu.open_fan_out(), None);
        for _ in 0..3 {
            rig.time += 0.1;
            rig.frame(Some(at_ring), true, &[]);
        }
        assert_eq!(rig.menu.open_fan_out(), None, "no popping straight back open under the pointer");
        // Moving away and out again opens it as usual.
        rig.point(Pos2::new(450.0, 300.0));
        rig.point(at_ring);
        assert!(rig.menu.open_fan_out().is_some());

        // The same for a click in the blocked sector.
        let mut rig = Rig::new(buttons());
        rig.frame(None, true, &[Key::Num6]);
        rig.point(BLOCKED);
        assert_eq!(rig.frame_with(Some(BLOCKED), true, click_at(BLOCKED)).0, PieMenuResponse::None);
        assert!(rig.menu.is_open() && rig.menu.open_fan_out().is_none());
    }

    #[test]
    fn releasing_on_the_opener_breaks_out_and_backspace_does_not_reopen_the_fan() {
        let at_ring = Pos2::new(505.0, 300.0);
        let mut rig = Rig::new(buttons());
        rig.point(at_ring);
        assert!(rig.menu.open_fan_out().is_some());
        assert_eq!(rig.frame(Some(at_ring), false, &[]).0, PieMenuResponse::None);
        assert_eq!(rig.menu.path(), &[0]);
        assert_eq!(rig.menu.open_fan_out(), None);
        // Inside: the same children as a sub-pie (no reveal animation here, so the main
        // ring, the opener included, is already gone).
        let (resp, drawn) = rig.frame(Some(at_ring), false, &[]);
        assert_eq!(resp, PieMenuResponse::None);
        assert_eq!(paths(&drawn), [vec![0, 0], vec![0, 1], vec![0, 2]]);
        // Backspace goes back to the root; the pointer rests on the opener beyond the ring.
        rig.frame(Some(at_ring), false, &[Key::Backspace]);
        for _ in 0..4 {
            rig.time += 0.1;
            rig.frame(Some(at_ring), false, &[]);
        }
        assert!(rig.menu.path().is_empty());
        assert_eq!(rig.menu.open_fan_out(), None, "no popping back open under the resting pointer");

        // And a number key inside a break-out selects with the full path.
        let mut rig = Rig::new(buttons());
        rig.point(at_ring);
        rig.frame(Some(at_ring), false, &[]);
        rig.frame(Some(at_ring), false, &[]);
        assert_eq!(rig.frame(Some(at_ring), false, &[Key::Num8]).0, PieMenuResponse::Selected(0));
        assert_eq!(rig.menu.selected_path(), &[0, 0]);
        assert!(!rig.menu.is_open());
    }

    #[test]
    fn clicking_the_opener_breaks_the_fan_out_by_default() {
        let at_ring = Pos2::new(505.0, 300.0);
        let mut rig = Rig::new(buttons());
        rig.point(at_ring);
        let trigger = rig.trigger();
        assert!(rig.menu.open_fan_out().is_some());
        assert_eq!(rig.frame_with(Some(trigger), true, click_at(trigger)).0, PieMenuResponse::None);
        assert!(rig.menu.is_open());
        assert_eq!(rig.menu.path(), &[0]);
        assert_eq!(rig.menu.open_fan_out(), None);
    }

    #[test]
    fn dismiss_setting_makes_releasing_on_the_opener_a_cancel() {
        let mut rig = Rig::new(buttons());
        rig.menu.settings.sub_pies.fan_out.opener_action = FanOpenerAction::Dismiss;
        rig.point(Pos2::new(505.0, 300.0));
        let trigger = rig.trigger();
        rig.point(trigger);
        assert!(rig.menu.open_fan_out().is_some());
        assert_eq!(rig.frame(Some(trigger), false, &[]).0, PieMenuResponse::Dismissed);
        assert!(!rig.menu.is_open());
    }

    #[test]
    fn breaking_out_with_the_button_centre_opens_the_sub_pie_at_the_opener() {
        let mut rig = Rig::new(buttons());
        rig.menu.settings.sub_pies.center = SubPieCenter::Button;
        let opener = rig.trigger();
        rig.point(Pos2::new(505.0, 300.0));
        rig.frame(Some(opener), false, &[]);
        assert_eq!(rig.menu.path(), &[0]);
        assert_eq!(rig.menu.position, opener);
    }

    // ── Breaking a fan out into a sub-pie, in place ──────────────────────────────

    const AT_RING: Pos2 = Pos2::new(505.0, 300.0);

    /// A rig with the default fan open (East), fading reveals of 200 ms, time at 1.0.
    fn fan_rig(fade: bool) -> Rig {
        let mut rig = Rig::new(buttons());
        if fade {
            rig.menu.settings.animations.reveal = RevealAnimation::Fade;
            rig.menu.settings.animations.reveal_duration = Duration::from_millis(200);
        }
        rig.time = 1.0;
        rig.point(AT_RING);
        rig.time = 2.0; // the fan's own reveal is over
        rig.frame(Some(AT_RING), true, &[]);
        rig.frame(Some(AT_RING), true, &[]);
        assert!(rig.menu.open_fan_out().is_some());
        rig
    }

    /// The rect of Area `id` (which must exist).
    fn area(rig: &Rig, id: egui::Id) -> Rect {
        rig.ctx.memory(|m| m.area_rect(id)).unwrap_or_else(|| panic!("no area {id:?}"))
    }

    /// The fan ring's children's Area rects.
    fn fan_rects(rig: &Rig) -> Vec<Rect> {
        (0..3).map(|i| area(rig, rig.menu.id.with(("fan", 0usize)).with(i))).collect()
    }

    fn opacity_of(rig: &Rig, path: &[usize]) -> Option<f32> {
        rig.opacities.iter().find(|(p, _)| p == path).map(|(_, o)| *o)
    }

    /// The break-out trigger: a click on the opener, or the key released on it.
    fn break_out(rig: &mut Rig, by_click: bool) {
        let trigger = rig.trigger();
        let resp = if by_click {
            rig.frame_with(Some(trigger), true, click_at(trigger)).0
        } else {
            rig.frame(Some(AT_RING), false, &[]).0
        };
        assert_eq!(resp, PieMenuResponse::None);
        assert_eq!(rig.menu.path(), &[0]);
    }

    #[test]
    fn breaking_out_keeps_the_children_exactly_where_the_fan_drew_them() {
        for by_click in [true, false] {
            let mut rig = fan_rig(true);
            let fan_center = rig.menu.fan_centers_for_test().to_vec();
            let before = fan_rects(&rig);
            break_out(&mut rig, by_click);
            let (_, drawn) = rig.frame(Some(AT_RING), false, &[]);
            assert_eq!(fan_rects(&rig), before, "no jump (click: {by_click})");
            for i in 0..3 {
                assert!(paths(&drawn).contains(&vec![0, i]), "the children are drawn on that very frame");
                assert_eq!(opacity_of(&rig, &[0, i]), Some(1.0), "no fading in");
            }
            assert_eq!(rig.menu.button_centers, fan_center, "same final centres");
        }
    }

    #[test]
    fn breaking_out_fades_the_whole_main_ring_including_the_opener() {
        let mut rig = fan_rig(true);
        let opener_id = rig.menu.id.with(0usize);
        let plain_id = rig.menu.id.with(1usize);
        let (opener_rect, plain_rect) = (area(&rig, opener_id), area(&rig, plain_id));
        break_out(&mut rig, true);
        let mut opener_opacities = Vec::new();
        let mut plain_opacities = Vec::new();
        for step in 0..=4 {
            let (_, drawn) = rig.frame(Some(AT_RING), true, &[]);
            if step < 4 {
                assert!(drawn.contains(&(vec![0], true)), "the opener stays lit while it fades");
            }
            for i in 0..3 {
                assert_eq!(opacity_of(&rig, &[0, i]), Some(1.0));
            }
            opener_opacities.push(opacity_of(&rig, &[0]));
            plain_opacities.push(opacity_of(&rig, &[1]));
            if step < 2 {
                assert_eq!(area(&rig, plain_id), plain_rect, "the fading ring stays at its old position");
                assert_eq!(area(&rig, opener_id), opener_rect, "the fading opener stays at its old position");
            }
            rig.time += 0.05;
        }
        // 200 ms in 50 ms steps: strictly down (opener from full, the rest from the dimmed
        // opacity), then gone.
        for (series, start) in [(&opener_opacities, 1.0), (&plain_opacities, DIMMED_OPACITY)] {
            let first = series[0].expect("drawn on the first frame");
            assert!((first - start).abs() < 1e-4, "{first} vs {start}");
            assert!(series[1].is_some_and(|o| o < first));
            assert!(series[2].is_some_and(|o| o < series[1].unwrap()));
            assert!(series[3].is_some_and(|o| o < series[2].unwrap()));
            assert_eq!(series[4], None, "not drawn once the fade is over");
        }
        // After the transition only the children are drawn.
        let (_, drawn) = rig.frame(Some(AT_RING), true, &[]);
        assert_eq!(paths(&drawn), [vec![0, 0], vec![0, 1], vec![0, 2]]);
    }

    /// Alpha of the painted background circles: (at the new centre, at the old centre).
    fn circle_alphas(rig: &Rig, new: Pos2, old: Pos2) -> (Option<u8>, Option<u8>) {
        let alpha_at = |at: Pos2| {
            rig.shapes.iter().find_map(|s| match &s.shape {
                egui::Shape::Circle(c) if (c.center - at).length() < 0.01 => Some(c.fill.a()),
                _ => None,
            })
        };
        (alpha_at(new), alpha_at(old))
    }

    #[test]
    fn breaking_out_moves_the_centre_and_cross_fades_the_circles() {
        let mut rig = fan_rig(true);
        let fan_center = rig.trigger();
        let old_center = rig.menu.display_center;
        assert_eq!(old_center, CENTER);
        let full = rig.menu.settings.center_indicator.background_fill_color.a();
        // Dimmed while the fan is open.
        let (_, dimmed) = circle_alphas(&rig, fan_center, old_center);
        let dimmed = dimmed.expect("old circle drawn");
        assert!((dimmed as f32 - full as f32 * DIMMED_OPACITY).abs() <= 2.0, "{dimmed} of {full}");

        break_out(&mut rig, true);
        assert_eq!(rig.menu.display_center, fan_center, "the new centre is the fan's drawn centre");
        let mut series = Vec::new();
        for _ in 0..6 {
            rig.frame(Some(AT_RING), true, &[]);
            series.push(circle_alphas(&rig, fan_center, old_center));
            rig.time += 0.04;
        }
        let new: Vec<u8> = series.iter().map(|(n, _)| n.unwrap_or(0)).collect();
        let old: Vec<u8> = series.iter().map(|(_, o)| o.unwrap_or(0)).collect();
        assert!(new[0] <= 2, "the new centre starts invisible: {new:?}");
        assert!(new.windows(2).all(|w| w[0] <= w[1]) && new[1] > new[0], "{new:?}");
        assert!((*new.last().unwrap() as i32 - full as i32).abs() <= 2, "ends at the full alpha: {new:?}");
        assert!((old[0] as f32 - dimmed as f32).abs() <= 2.0, "the old one starts dimmed: {old:?}");
        assert!(old.windows(2).all(|w| w[0] >= w[1]) && old[1] < old[0], "{old:?}");
        assert_eq!(*old.last().unwrap(), 0, "and is gone: {old:?}");
    }

    #[test]
    fn breaking_out_without_a_reveal_animation_is_instant() {
        let mut rig = fan_rig(false);
        let fan_center = rig.trigger();
        break_out(&mut rig, true);
        let (_, drawn) = rig.frame(Some(AT_RING), true, &[]);
        assert_eq!(paths(&drawn), [vec![0, 0], vec![0, 1], vec![0, 2]], "no main-ring button, the opener included");
        assert_eq!(rig.menu.display_center, fan_center);
        let (new, old) = circle_alphas(&rig, fan_center, CENTER);
        assert_eq!(new, Some(rig.menu.settings.center_indicator.background_fill_color.a()));
        assert_eq!(old, None);
    }

    #[test]
    fn going_back_from_a_break_out_restores_the_main_pie_where_it_was() {
        for centre_click in [true, false] {
            let mut rig = fan_rig(true);
            let trigger = rig.trigger();
            let opener_rect = area(&rig, rig.menu.id.with(0usize));
            break_out(&mut rig, true);
            rig.time += 0.1;
            rig.frame(Some(AT_RING), true, &[]);
            assert!(rig.menu.breakout.is_some());
            if centre_click {
                rig.frame_with(Some(trigger), true, click_at(trigger));
            } else {
                rig.frame(Some(AT_RING), true, &[Key::Backspace]);
            }
            assert!(rig.menu.path().is_empty());
            assert!(rig.menu.breakout.is_none() && rig.menu.level_radius.is_none());
            assert_eq!((rig.menu.position, rig.menu.display_center), (CENTER, CENTER));
            for _ in 0..5 {
                rig.time += 0.1;
                let (_, drawn) = rig.frame(Some(trigger), true, &[]);
                assert_eq!(rig.menu.open_fan_out(), None, "the fan does not reopen under the pointer");
                assert_eq!(paths(&drawn), [vec![0], vec![1]]);
            }
            assert_eq!(rig.menu.display_center, CENTER);
            assert_eq!(area(&rig, rig.menu.id.with(0usize)), opener_rect, "the main ring is where it was");
        }
        // A fresh opening has no break-out state.
        let mut rig = fan_rig(true);
        break_out(&mut rig, false);
        rig.menu.open(CENTER);
        assert!(rig.menu.breakout.is_none() && rig.menu.level_radius.is_none());
    }

    #[test]
    fn a_fan_with_its_own_radius_breaks_out_at_that_radius() {
        let mut rig = Rig::new(buttons());
        rig.menu.settings.sub_pies.fan_out.radius = Some(140.0);
        rig.menu.settings.animations.reveal = RevealAnimation::Fade;
        rig.time = 1.0;
        rig.point(AT_RING);
        rig.time = 2.0;
        rig.frame(Some(AT_RING), true, &[]);
        rig.frame(Some(AT_RING), true, &[]);
        let before = fan_rects(&rig);
        break_out(&mut rig, true);
        rig.frame(Some(AT_RING), true, &[]);
        assert_eq!(fan_rects(&rig), before);
        assert_eq!(rig.menu.level_radius, Some(140.0));
    }

    #[test]
    fn backspace_closes_the_fan_for_good_while_the_pointer_rests_on_its_button() {
        let at_ring = Pos2::new(505.0, 300.0);
        let mut rig = Rig::new(buttons());
        rig.point(at_ring);
        assert!(rig.menu.open_fan_out().is_some());
        rig.frame(Some(at_ring), true, &[Key::Backspace]);
        for _ in 0..3 {
            rig.time += 0.1;
            rig.frame(Some(at_ring), true, &[]);
        }
        assert_eq!(rig.menu.open_fan_out(), None);
    }

    #[test]
    fn releasing_with_nothing_selectable_dismisses_instead_of_stranding_the_menu() {
        let all_disabled = || vec![PieButton::new(PieDirection::North).with_enabled(false)];
        let above = Some(Pos2::new(400.0, 200.0));
        let mut rig = Rig::new(all_disabled());
        rig.frame(above, true, &[]);
        assert_eq!(rig.frame(above, false, &[]).0, PieMenuResponse::Dismissed);
        // No pointer at all at release (outside the window): also a cancel.
        let mut rig = Rig::new(buttons());
        assert_eq!(rig.frame(None, false, &[]).0, PieMenuResponse::Dismissed);
    }

    /// The closure can tell folders (buttons that only open something) from actions.
    #[test]
    fn button_view_tells_what_a_button_opens() {
        let root = vec![
            PieButton::new(PieDirection::East).with_fan_out(vec![PieButton::new(PieDirection::North)]),
            PieButton::new(PieDirection::North).with_label("Plain"),
            PieButton::new(PieDirection::West)
                .with_children(vec![PieButton::new(PieDirection::North)])
                .with_sub_pie_style(Some(SubPieStyle::Replace)),
        ];
        let mut rig = Rig::new(root.clone());
        rig.frame(None, true, &[Key::Num6]); // opens the fan so its child is drawn too
        let ctx = egui::Context::default();
        let mut seen = Vec::new();
        let mut m = PieMenu::new();
        m.settings.animations.reveal = RevealAnimation::None;
        m.open(CENTER);
        m.reveal();
        for key in [None, None, Some(Key::Num6), None] {
            let events = key
                .into_iter()
                .map(|key| egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE })
                .collect();
            let input = egui::RawInput { events, ..Default::default() };
            seen.clear();
            let mut out = ctx.run_ui(input, |ui| {
                m.show_with_pointer(ui.ctx(), &root, None, true, |ui, b| {
                    seen.push((b.path.to_vec(), b.opens_sub_pie(), b.opens_fan_out()));
                    ui.label("x");
                });
            });
            out.textures_delta.clear();
        }
        seen.sort();
        assert_eq!(
            seen,
            [
                (vec![0], true, true),       // fan-out
                (vec![0, 0], false, false),  // the fan's child
                (vec![1], false, false),     // a plain action
                (vec![2], true, false),      // a replace-style sub-pie
            ]
        );
    }

    #[test]
    fn problems_cover_blocked_directions_and_nesting() {
        let m = PieMenu::new();
        let level = vec![
            PieButton::new(PieDirection::North),
            PieButton::new(PieDirection::East).with_fan_out(vec![
                PieButton::new(PieDirection::North),
                PieButton::new(PieDirection::West), // straight back at the parent
                PieButton::new(PieDirection::NorthWest), // its neighbour
                PieButton::new(PieDirection::South).with_children(vec![PieButton::new(PieDirection::South)]),
                PieButton::new(PieDirection::North), // same direction as the first
            ]),
        ];
        let in_fan = |problem| ButtonProblem::InFanOut { parent: 1, problem: Box::new(problem) };
        let problems = m.check_buttons(&level);
        assert_eq!(
            problems,
            vec![
                in_fan(ButtonProblem::SharedDirection { first: 0, second: 4, direction: PieDirection::North }),
                in_fan(ButtonProblem::BlockedDirection { button: 1, direction: PieDirection::West }),
                in_fan(ButtonProblem::BlockedDirection { button: 2, direction: PieDirection::NorthWest }),
                in_fan(ButtonProblem::NestedInFanOut { button: 3 }),
            ]
        );
        assert_eq!(problems[1].buttons(), [1]);
        assert_eq!(problems[1].to_string(), "in the fan-out of button 1: button 1 is in direction West, which faces the parent pie");
        // The same children under a replace-style button are an ordinary level: no fan problems.
        let replace = vec![level[1].clone().with_sub_pie_style(Some(SubPieStyle::Replace))];
        assert!(m.check_buttons(&replace).is_empty());

        for parent in PieDirection::ALL {
            let slots = parent.fan_out_slots();
            assert_eq!(slots[2], parent, "straight ahead is the middle slot");
            assert!(parent.blocks_fan_out(parent.opposite()));
            assert_eq!(PieDirection::ALL.iter().filter(|d| parent.blocks_fan_out(**d)).count(), 3);
        }
    }

    // ── set_open_fan_out ──────────────────────────────────────────────────────

    fn pinned_rig() -> Rig {
        let mut rig = Rig::new(buttons());
        rig.menu.pin(CENTER);
        rig.frame(None, true, &[]);
        rig.frame(None, true, &[]);
        rig
    }

    #[test]
    fn set_open_fan_out_opens_and_stays_open_without_a_pointer() {
        let mut rig = pinned_rig();
        assert_eq!(rig.menu.open_fan_out(), None);
        rig.menu.set_open_fan_out(Some(0));
        assert_eq!(rig.menu.open_fan_out(), Some((0, None)));
        for _ in 0..10 {
            rig.time += 0.05;
            rig.frame(None, true, &[]);
        }
        assert_eq!(rig.menu.open_fan_out(), Some((0, None)));
        assert_eq!(rig.menu.fan_centers_for_test().len(), 3);
        assert!(rig.menu.path().is_empty() && rig.menu.is_pinned());
    }

    #[test]
    fn set_open_fan_out_is_closed_by_a_real_pointer_on_another_button() {
        let mut rig = pinned_rig();
        rig.menu.set_open_fan_out(Some(0));
        rig.frame(None, true, &[]);
        assert!(rig.menu.open_fan_out().is_some());
        // North (index 1) of the parent ring.
        rig.frame(Some(CENTER + Vec2::new(0.0, -90.0)), true, &[]);
        assert_eq!(rig.menu.open_fan_out(), None);
    }

    #[test]
    fn set_open_fan_out_on_a_button_without_a_fan_closes_on_the_next_show() {
        for bad in [1usize, 7, usize::MAX] {
            let mut rig = pinned_rig();
            rig.menu.set_open_fan_out(Some(bad));
            assert_eq!(rig.menu.open_fan_out(), Some((bad, None)));
            rig.frame(None, true, &[]);
            assert_eq!(rig.menu.open_fan_out(), None, "trigger {bad}");
            rig.frame(None, true, &[]);
            assert_eq!(rig.menu.open_fan_out(), None);
        }
    }

    #[test]
    fn set_open_fan_out_none_closes_and_a_closed_menu_is_untouched() {
        let mut rig = pinned_rig();
        rig.menu.set_open_fan_out(Some(0));
        rig.frame(None, true, &[]);
        rig.menu.set_open_fan_out(None);
        assert_eq!(rig.menu.open_fan_out(), None);
        rig.frame(None, true, &[]);
        assert_eq!(rig.menu.open_fan_out(), None);

        let mut closed = PieMenu::new();
        closed.set_open_fan_out(Some(0));
        assert_eq!(closed.open_fan_out(), None);
        assert!(!closed.is_open());
    }

    #[test]
    fn set_open_fan_out_again_does_not_restart_the_fan() {
        let mut rig = pinned_rig();
        rig.menu.set_open_fan_out(Some(0));
        rig.time = 1.0;
        rig.frame(None, true, &[]);
        let shown = rig.menu.fan_shown_at;
        assert!(shown.is_some());
        rig.menu.set_open_fan_out(Some(0));
        rig.time = 2.0;
        rig.frame(None, true, &[]);
        assert_eq!(rig.menu.fan_shown_at, shown);
        // Another trigger does restart it.
        rig.menu.set_open_fan_out(Some(1));
        assert_eq!(rig.menu.fan_shown_at, None);
    }

    #[test]
    fn set_open_fan_out_works_in_a_replace_level_and_a_broken_out_level() {
        let root = vec![PieButton::new(PieDirection::East).with_label("Sub").with_children(vec![
            PieButton::new(PieDirection::NorthEast).with_label("More").with_fan_out(vec![
                PieButton::new(PieDirection::North).with_label("A"),
                PieButton::new(PieDirection::East).with_label("B"),
            ]),
            PieButton::new(PieDirection::West).with_label("Plain"),
        ])];
        let mut rig = Rig::new(root);
        rig.menu.pin(CENTER);
        rig.frame(None, true, &[]);
        rig.frame(None, true, &[Key::Num6]);
        rig.frame(None, true, &[]);
        assert_eq!(rig.menu.path(), &[0]);
        rig.menu.set_open_fan_out(Some(0));
        for _ in 0..5 {
            rig.frame(None, true, &[]);
        }
        assert_eq!(rig.menu.open_fan_out(), Some((0, None)));
        assert_eq!(rig.menu.fan_centers_for_test().len(), 2);
        assert_eq!(rig.menu.path(), &[0]);

        // A broken-out level: the fan of the default rig, broken out, then a plain trigger.
        let mut rig = fan_rig(true);
        break_out(&mut rig, true);
        rig.frame(Some(AT_RING), true, &[]);
        assert!(rig.menu.breakout.is_some());
        rig.menu.set_open_fan_out(Some(2));
        rig.frame(None, true, &[]);
        assert_eq!(rig.menu.open_fan_out(), None, "a plain button of the broken-out level opens no fan");
        assert_eq!(rig.menu.path(), &[0]);
    }
}

mod view_color_tests {
    use super::*;

    /// `ButtonView::text_color` is the colour `label_text` paints with, forced or from the
    /// theme, and greyed out for a disabled button.
    #[test]
    fn text_color_matches_the_label() {
        let colors = |enabled: bool, forced: Option<Color32>| {
            let ctx = egui::Context::default();
            let buttons = vec![PieButton::new(PieDirection::North).with_label("Cut").with_enabled(enabled)];
            let mut m = PieMenu::new();
            m.settings.text_color = forced;
            m.open(Pos2::new(400.0, 300.0));
            m.reveal();
            let mut seen = None;
            for _ in 0..2 {
                let mut out = ctx.run_ui(Default::default(), |ui| {
                    m.show(ui.ctx(), &buttons, true, |ui, b| {
                        seen = Some(b.text_color());
                        ui.label(b.label_text());
                    });
                });
                out.textures_delta.clear();
            }
            (seen.expect("button drawn"), ctx.global_style().visuals.clone())
        };
        let (forced, _) = colors(true, Some(Color32::WHITE));
        assert_eq!(forced, Color32::WHITE);
        let (themed, v) = colors(true, None);
        assert_eq!(themed, v.widgets.inactive.fg_stroke.color);
        let (off, v) = colors(false, Some(Color32::WHITE));
        assert_eq!(off, v.gray_out(Color32::WHITE));
    }
}

mod pie_hover_tests {
    use super::*;

    enum Pointer {
        None,
        /// In the East sector but far from the widget.
        Far,
        /// Pressed (and held) on the widget itself.
        PressedOnWidget,
    }

    /// Paints one East button with an `egui::Button`; returns its (fill, stroke) in each
    /// of the pointer states, in order, and the theme's widget visuals.
    fn looks(use_helper: bool, states: &[Pointer]) -> (Vec<(Color32, Stroke)>, egui::style::Widgets) {
        fn find(shape: &egui::Shape) -> Option<(Rect, Color32, Stroke)> {
            match shape {
                egui::Shape::Rect(r) if (20.0..200.0).contains(&r.rect.width()) => Some((r.rect, r.fill, r.stroke)),
                egui::Shape::Vec(v) => v.iter().find_map(find),
                _ => None,
            }
        }
        let ctx = egui::Context::default();
        let buttons = vec![PieButton::new(PieDirection::East).with_label("Go")];
        let mut m = PieMenu::new();
        m.settings.animations.reveal = RevealAnimation::None;
        m.open(Pos2::new(400.0, 300.0));
        m.reveal();
        let mut time = 0.0;
        let mut run = |pointer: Option<Pos2>, events: Vec<egui::Event>| {
            time += 0.1;
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
                time: Some(time),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                m.show_with_pointer(ui.ctx(), &buttons, pointer, true, |ui, b| {
                    if use_helper {
                        b.use_pie_hover(ui);
                    }
                    ui.add(egui::Button::new(b.label_text()));
                });
            });
            out.textures_delta.clear();
            out.shapes.iter().find_map(|s| find(&s.shape))
        };
        // Lay the button out and learn where it is.
        let mut widget = None;
        for _ in 0..3 {
            widget = run(None, vec![]).or(widget);
        }
        let centre = widget.expect("button painted").0.center();
        let mut result = vec![];
        for state in states {
            let pointer = match state {
                Pointer::None => None,
                Pointer::Far => Some(Pos2::new(700.0, 300.0)),
                Pointer::PressedOnWidget => Some(centre),
            };
            let mut last = None;
            for n in 0..4 {
                let events = match (state, pointer) {
                    (Pointer::PressedOnWidget, Some(p)) if n == 1 => vec![
                        egui::Event::PointerMoved(p),
                        egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE },
                    ],
                    _ => vec![],
                };
                last = run(pointer, events).or(last);
            }
            let (_, fill, stroke) = last.expect("button painted");
            result.push((fill, stroke));
        }
        (result, ctx.global_style().visuals.widgets.clone())
    }

    #[test]
    fn pie_hover_drives_the_widget_look_not_the_pointer_over_it() {
        let states = [Pointer::None, Pointer::Far, Pointer::PressedOnWidget];
        let (got, w) = looks(true, &states);
        let idle = (w.inactive.weak_bg_fill, w.inactive.bg_stroke);
        let hovered = (w.hovered.weak_bg_fill, w.hovered.bg_stroke);
        assert_ne!(idle, hovered, "the theme distinguishes the two looks");
        assert_eq!(got[0], idle, "idle");
        assert_eq!(got[1], hovered, "pie-highlighted, pointer far from the widget");
        assert_eq!(got[2], hovered, "pressed on the widget: no press change");

        // Without the helper the far pointer leaves the widget looking idle: the problem it solves.
        let (plain, _) = looks(false, &states);
        assert_eq!(plain[1], idle);
    }
}

mod cluster_tests {
    use super::*;
    use std::collections::BTreeMap;

    const CENTER: Pos2 = Pos2::new(400.0, 300.0);
    /// The item numbers, deliberately not in layout order.
    const NUMS: [usize; 4] = [7, 3, 9, 1];

    #[derive(Clone, Copy, PartialEq)]
    enum Layout {
        Row,
        Spaced,
        Column,
        Grid,
        /// One 420 px wide item (number 5): reaches far into the East button's sector.
        Wide,
    }

    impl Layout {
        fn nums(self) -> &'static [usize] {
            match self {
                Layout::Row | Layout::Spaced => &NUMS[..3],
                Layout::Column | Layout::Grid => &NUMS,
                Layout::Wide => &[5],
            }
        }
    }

    struct Rig {
        ctx: egui::Context,
        menu: PieMenu,
        buttons: Vec<PieButton>,
        layout: Layout,
        /// Item number to draw disabled (`ui.add_enabled(false, ..)`).
        disabled_item: Option<usize>,
        /// Each item's rect, as the last frame drew it.
        rects: BTreeMap<usize, Rect>,
        /// `b.item` returned a response equal to the one given, every time.
        same_response: bool,
        /// Labels of the ordinary buttons drawn last frame (index -> number hint).
        hints: BTreeMap<usize, Option<String>>,
        /// Per item number (999: undeclared): `item_label(n)`'s layout job and `item_mnemonic(n)`.
        item_labels: BTreeMap<usize, (egui::text::LayoutJob, Option<char>)>,
        /// Draw a focused text field before the menu (egui then wants the keyboard).
        focus_text: bool,
        text: String,
    }

    /// A cluster at North (index 0), ordinary buttons at East (1) and West (2).
    fn mixed() -> Vec<PieButton> {
        vec![
            PieButton::cluster(PieDirection::North).with_label("Zeta"),
            PieButton::new(PieDirection::East).with_label("Zoom"),
            PieButton::new(PieDirection::West).with_label("Alpha"),
        ]
    }

    fn click_at(pos: Pos2) -> Vec<egui::Event> {
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        vec![egui::Event::PointerMoved(pos), button(true), button(false)]
    }

    impl Rig {
        fn new(buttons: Vec<PieButton>, layout: Layout) -> Self {
            let mut menu = PieMenu::new();
            menu.settings.animations.reveal = RevealAnimation::None;
            menu.open(CENTER);
            menu.reveal();
            let mut rig = Self {
                ctx: egui::Context::default(),
                menu,
                buttons,
                layout,
                disabled_item: None,
                rects: BTreeMap::new(),
                same_response: true,
                hints: BTreeMap::new(),
                item_labels: BTreeMap::new(),
                focus_text: false,
                text: String::new(),
            };
            rig.settle();
            rig
        }

        /// Frames with no pointer, so every button is laid out at its real size.
        fn settle(&mut self) {
            for _ in 0..3 {
                self.frame(None, true, &[]);
            }
        }

        fn frame(&mut self, pointer: Option<Pos2>, key_down: bool, keys: &[Key]) -> PieMenuResponse {
            let events = keys
                .iter()
                .map(|&key| egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE })
                .collect();
            self.frame_with(pointer, key_down, events)
        }

        fn frame_with(&mut self, pointer: Option<Pos2>, key_down: bool, events: Vec<egui::Event>) -> PieMenuResponse {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
                events,
                ..Default::default()
            };
            let (layout, disabled_item) = (self.layout, self.disabled_item);
            let (mut rects, mut same, mut hints) = (BTreeMap::new(), true, BTreeMap::new());
            let (menu, buttons) = (&mut self.menu, &self.buttons);
            let (focus_text, text) = (self.focus_text, &mut self.text);
            let mut labels = BTreeMap::new();
            let mut resp = PieMenuResponse::None;
            let mut out = self.ctx.run_ui(input, |ui| {
                if focus_text {
                    ui.text_edit_singleline(text).request_focus();
                }
                resp = menu.show_with_pointer(ui.ctx(), buttons, pointer, key_down, |ui, b| {
                    hints.insert(b.idx, b.number_hint.as_ref().map(|h| h.text.clone()));
                    if !b.button.is_cluster() {
                        ui.label(b.label_text());
                        return;
                    }
                    for n in b.button.cluster_items().iter().map(|i| i.number).chain([999]) {
                        labels.insert(n, (b.item_label(n).to_job(), b.item_mnemonic(n)));
                    }
                    let mut item = |ui: &mut Ui, n: usize, width: Option<f32>| {
                        let text = format!("Item {n}");
                        let enabled = disabled_item != Some(n);
                        let response = match width {
                            Some(w) => ui.add_enabled(enabled, egui::Button::new(text).min_size(Vec2::new(w, 20.0))),
                            None => ui.add_enabled(enabled, egui::Button::new(text)),
                        };
                        let returned = b.item(n, response.clone());
                        same &= returned.rect == response.rect && returned.id == response.id;
                        rects.insert(n, response.rect);
                    };
                    match layout {
                        Layout::Row => {
                            ui.horizontal(|ui| NUMS[..3].iter().for_each(|&n| item(ui, n, None)));
                        }
                        Layout::Spaced => {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 30.0;
                                NUMS[..3].iter().for_each(|&n| item(ui, n, None));
                            });
                        }
                        Layout::Column => {
                            ui.vertical(|ui| NUMS.iter().for_each(|&n| item(ui, n, None)));
                        }
                        Layout::Grid => {
                            egui::Grid::new("cluster_grid").show(ui, |ui| {
                                item(ui, NUMS[0], None);
                                item(ui, NUMS[1], None);
                                ui.end_row();
                                item(ui, NUMS[2], None);
                                item(ui, NUMS[3], None);
                                ui.end_row();
                            });
                        }
                        Layout::Wide => item(ui, 5, Some(420.0)),
                    }
                });
            });
            out.textures_delta.clear();
            if !rects.is_empty() {
                self.rects = rects;
            }
            if !labels.is_empty() {
                self.item_labels = labels;
            }
            self.same_response &= same;
            if !hints.is_empty() {
                self.hints = hints;
            }
            resp
        }

        fn centre_of(&self, n: usize) -> Pos2 {
            self.rects[&n].center()
        }

        fn click(&mut self, pos: Pos2) -> PieMenuResponse {
            self.frame_with(Some(pos), true, click_at(pos))
        }

        /// The menu's drawn centre (it may have been shifted to fit the screen).
        fn centre(&self) -> Pos2 {
            self.menu.display_center
        }

        /// A point in the cluster's sector (straight up from the centre, past the dead
        /// zone) that is on none of the items.
        fn dead_spot(&self) -> Pos2 {
            let p = self.centre() + Vec2::new(0.0, -(self.menu.settings.layout.dead_zone_radius + 3.0));
            assert!(self.rects.values().all(|r| !r.contains(p)), "the spot must be off every item");
            p
        }
    }

    const LAYOUTS: [Layout; 4] = [Layout::Row, Layout::Spaced, Layout::Column, Layout::Grid];

    #[test]
    fn api_basics() {
        let c = PieButton::cluster(PieDirection::North);
        assert!(c.is_cluster());
        assert!(c.children().is_empty());
        assert!(c.label.is_empty());
        assert_eq!(c.mnemonic, Mnemonic::None);
        assert!(matches!(c.kind, PieButtonKind::Cluster(_)));
        assert!(c.cluster_items().is_empty());
        assert!(!PieButton::new(PieDirection::North).is_cluster());
        assert!(!PieMenu::new().opens_fan(&c));
        assert_eq!(c.with_label("x").label, "x");
    }

    #[test]
    fn clicking_each_item_selects_its_number_in_every_layout() {
        for layout in LAYOUTS {
            for &n in layout.nums() {
                let mut rig = Rig::new(mixed(), layout);
                assert_eq!(rig.rects.len(), layout.nums().len());
                let pos = rig.centre_of(n);
                assert_eq!(rig.click(pos), PieMenuResponse::Selected(n), "item {n}");
                assert_eq!(rig.menu.selected_path(), &[0, n]);
                assert!(!rig.menu.is_open());
            }
        }
    }

    #[test]
    fn a_spaced_row_has_gaps_between_the_items() {
        let rig = Rig::new(mixed(), Layout::Spaced);
        let gap = rig.rects[&3].min.x - rig.rects[&7].max.x;
        assert!(gap >= 29.0, "gap {gap}");
        // The gap itself is not an item.
        let mid = Pos2::new((rig.rects[&3].min.x + rig.rects[&7].max.x) / 2.0, rig.rects[&7].center().y);
        let mut rig = rig;
        assert_eq!(rig.click(mid), PieMenuResponse::None);
    }

    #[test]
    fn releasing_on_an_item_selects_it_and_releasing_beside_the_items_dismisses() {
        for layout in LAYOUTS {
            for &n in layout.nums() {
                let mut rig = Rig::new(mixed(), layout);
                let pos = rig.centre_of(n);
                rig.frame(Some(pos), true, &[]);
                assert_eq!(rig.frame(Some(pos), false, &[]), PieMenuResponse::Selected(n));
                assert_eq!(rig.menu.selected_path(), &[0, n]);
                assert!(!rig.menu.is_open());
            }
        }
        let mut rig = Rig::new(mixed(), Layout::Row);
        let spot = rig.dead_spot();
        rig.frame(Some(spot), true, &[]);
        assert_eq!(rig.frame(Some(spot), false, &[]), PieMenuResponse::Dismissed);
        assert!(!rig.menu.is_open());
    }

    #[test]
    fn the_clusters_direction_is_dead() {
        let mut rig = Rig::new(mixed(), Layout::Row);
        let spot = rig.dead_spot();
        rig.frame(Some(spot), true, &[]);
        assert_eq!(rig.menu.hovered(), None);
        // Far out in the same direction, beside the items: still nothing.
        let far = rig.centre() + Vec2::new(0.0, -250.0);
        rig.frame(Some(far), true, &[]);
        assert_eq!(rig.menu.hovered(), None);
        // A click there selects nothing either.
        assert_eq!(rig.click(spot), PieMenuResponse::None);
        assert!(rig.menu.is_open());

        // The neighbours still highlight, and the cluster is never skipped over to them:
        // slightly towards East from the cluster's direction is still the cluster's sector.
        let near_north = rig.centre() + Vec2::new(10.0, -60.0);
        rig.frame(Some(near_north), true, &[]);
        assert_eq!(rig.menu.hovered(), None);
        rig.frame(Some(rig.centre() + Vec2::new(60.0, 0.0)), true, &[]);
        assert_eq!(rig.menu.hovered(), Some(1));
    }

    #[test]
    fn no_number_key_or_mnemonic_selects_a_cluster() {
        let mut rig = Rig::new(mixed(), Layout::Row);
        rig.menu.settings.input.auto_mnemonics = true;
        assert_eq!(rig.frame(None, true, &[Key::Num8]), PieMenuResponse::None);
        assert!(rig.menu.is_open());
        // The cluster has no hint, the others do; and it never gets a mnemonic.
        rig.frame(None, true, &[]);
        assert_eq!(rig.hints[&0], None);
        assert_eq!(rig.hints[&1].as_deref(), Some("6"));
        assert_eq!(rig.menu.number_hint(&rig.buttons[0]), None);
        assert!(rig.menu.number_hint(&rig.buttons[1]).is_some());
        // "Zeta" would have claimed Z, which stays with "Zoom"; "Alpha" keeps A.
        let m = rig.menu.mnemonics(&rig.buttons);
        assert_eq!(m, [None, Some('z'), Some('a')]);
        // An explicit mnemonic on a cluster is ignored too, and does not reserve its key.
        let mut buttons = mixed();
        buttons[0] = buttons[0].clone().with_mnemonic('z');
        assert_eq!(rig.menu.mnemonics(&buttons), [None, Some('z'), Some('a')]);
        assert!(rig.menu.check_buttons(&buttons).is_empty());
        // Typing the letter picks the ordinary button.
        let ev = vec![egui::Event::Text("z".into())];
        assert_eq!(rig.frame_with(None, true, ev), PieMenuResponse::Selected(1));
    }

    #[test]
    fn with_clicks_off_a_click_does_nothing_and_a_release_selects() {
        let mut rig = Rig::new(mixed(), Layout::Column);
        rig.menu.settings.input.select_on_primary_click = false;
        let pos = rig.centre_of(9);
        assert_eq!(rig.click(pos), PieMenuResponse::None);
        assert!(rig.menu.is_open());
        assert_eq!(rig.frame(Some(pos), false, &[]), PieMenuResponse::Selected(9));
        assert_eq!(rig.menu.selected_path(), &[0, 9]);
    }

    #[test]
    fn disabled_items_and_disabled_clusters_cannot_be_selected() {
        let mut rig = Rig::new(mixed(), Layout::Row);
        rig.disabled_item = Some(3);
        rig.settle();
        let pos = rig.centre_of(3);
        assert_eq!(rig.click(pos), PieMenuResponse::None);
        assert!(rig.menu.is_open());
        rig.frame(Some(pos), true, &[]);
        assert_eq!(rig.frame(Some(pos), false, &[]), PieMenuResponse::Dismissed);
        // Its neighbours still work.
        let mut rig = Rig::new(mixed(), Layout::Row);
        rig.disabled_item = Some(3);
        rig.settle();
        let pos = rig.centre_of(9);
        assert_eq!(rig.click(pos), PieMenuResponse::Selected(9));

        // A disabled cluster button: none of its items.
        let mut buttons = mixed();
        buttons[0].enabled = false;
        let mut rig = Rig::new(buttons.clone(), Layout::Row);
        let pos = rig.centre_of(7);
        // Like any disabled button its sector snaps to the closest enabled one (West here),
        // but never to one of its items.
        rig.click(pos);
        assert_ne!(rig.menu.selected_path(), &[0, 7]);
        let mut rig = Rig::new(buttons, Layout::Row);
        let pos = rig.centre_of(7);
        rig.frame(Some(pos), true, &[]);
        rig.frame(Some(pos), false, &[]);
        assert_ne!(rig.menu.selected_path(), &[0, 7]);
    }

    #[test]
    fn a_cluster_inside_a_replace_style_sub_pie() {
        let root = vec![PieButton::new(PieDirection::East).with_label("More").with_children(vec![
            PieButton::cluster(PieDirection::North),
            PieButton::new(PieDirection::South).with_label("Down"),
        ])];
        assert!(PieMenu::new().check_buttons(&root).is_empty());
        let mut rig = Rig::new(root, Layout::Grid);
        rig.frame(None, true, &[Key::Num6]);
        assert_eq!(rig.menu.path(), &[0]);
        rig.settle();
        assert_eq!(rig.rects.len(), 4);
        let pos = rig.centre_of(1);
        assert_eq!(rig.click(pos), PieMenuResponse::Selected(1));
        assert_eq!(rig.menu.selected_path(), &[0, 0, 1]);
        assert!(!rig.menu.is_open());
    }

    #[test]
    fn a_pinned_menu_stays_open_after_an_item_is_clicked() {
        let mut rig = Rig::new(mixed(), Layout::Row);
        rig.menu.pin(CENTER);
        rig.settle();
        for n in [3, 9] {
            let pos = rig.centre_of(n);
            assert_eq!(rig.click(pos), PieMenuResponse::Selected(n));
            assert_eq!(rig.menu.selected_path(), &[0, n]);
            assert!(rig.menu.is_open() && rig.menu.is_pinned());
            rig.settle();
        }
    }

    #[test]
    fn a_cluster_in_a_fan_out_is_a_problem() {
        let m = PieMenu::new();
        let level = vec![PieButton::new(PieDirection::East).with_fan_out(vec![
            PieButton::new(PieDirection::South),
            PieButton::cluster(PieDirection::North),
        ])];
        assert_eq!(
            m.check_buttons(&level),
            vec![ButtonProblem::InFanOut { parent: 0, problem: Box::new(ButtonProblem::NestedInFanOut { button: 1 }) }]
        );
        let text = m.check_buttons(&level)[0].to_string();
        assert!(text.contains("a sub-pie or cluster of its own"), "{text}");
        // As an ordinary replace-style child it is fine.
        assert!(m.check_buttons(&[level[0].clone().with_sub_pie_style(Some(SubPieStyle::Replace))]).is_empty());
    }

    #[test]
    fn an_item_reaching_into_another_sector_wins_over_that_buttons_pick() {
        let mut rig = Rig::new(mixed(), Layout::Wide);
        let rect = rig.rects[&5];
        let spot = Pos2::new(rect.max.x - 8.0, rect.center().y);
        // That spot is nearer the East direction than North: without the item it is East's.
        let v = spot - rig.centre();
        assert!(v.x > -v.y, "the spot is in East's half: {v:?}");
        rig.frame(Some(spot), true, &[]);
        assert_eq!(rig.menu.hovered(), Some(1), "the pie itself points at East there");
        assert_eq!(rig.click(spot), PieMenuResponse::Selected(5));
        assert_eq!(rig.menu.selected_path(), &[0, 5], "the item, not the East button");

        // Releasing there selects the item too.
        let mut rig = Rig::new(mixed(), Layout::Wide);
        rig.frame(Some(spot), true, &[]);
        assert_eq!(rig.frame(Some(spot), false, &[]), PieMenuResponse::Selected(5));
        assert_eq!(rig.menu.selected_path(), &[0, 5]);
    }

    #[test]
    fn ordinary_buttons_next_to_a_cluster_still_work() {
        let mut rig = Rig::new(mixed(), Layout::Row);
        let east = rig.centre() + Vec2::new(150.0, 0.0);
        rig.frame(Some(east), true, &[]);
        assert_eq!(rig.menu.hovered(), Some(1));
        assert_eq!(rig.click(east), PieMenuResponse::Selected(1));
        assert_eq!(rig.menu.selected_path(), &[1]);

        let mut rig = Rig::new(mixed(), Layout::Row);
        let west = rig.centre() + Vec2::new(-150.0, 0.0);
        rig.frame(Some(west), true, &[]);
        assert_eq!(rig.menu.hovered(), Some(2));
        assert_eq!(rig.frame(Some(west), false, &[]), PieMenuResponse::Selected(2));

        let mut rig = Rig::new(mixed(), Layout::Row);
        assert_eq!(rig.frame(None, true, &[Key::Num6]), PieMenuResponse::Selected(1));

        // Keys 4 and 8 only reach the West button; the cluster's 8 does nothing.
        let mut rig = Rig::new(mixed(), Layout::Row);
        assert_eq!(rig.frame(None, true, &[Key::Num4]), PieMenuResponse::Selected(2));

        // `b.item` hands the response back unchanged.
        assert!(rig.same_response);
    }

    #[test]
    fn the_cluster_is_never_hovered_so_nothing_emphasises_it() {
        let mut rig = Rig::new(mixed(), Layout::Row);
        let spot = rig.dead_spot();
        for _ in 0..4 {
            rig.frame(Some(spot), true, &[]);
        }
        assert_eq!(rig.menu.hovered(), None);
        assert_eq!(rig.menu.open_fan_out(), None);
    }

    // ── Declared items: mnemonics ────────────────────────────────────────────────

    fn items(list: &[(usize, &str)]) -> Vec<ClusterItem> {
        list.iter().map(|&(n, l)| ClusterItem::new(n, l)).collect()
    }

    /// Cluster North with items 7 "Cut", 3 "Cursor", 9 "Paste"; "Copy" East; "Zoom" West.
    fn declared() -> Vec<PieButton> {
        vec![
            PieButton::cluster(PieDirection::North).with_items(items(&[(7, "Cut"), (3, "Cursor"), (9, "Paste")])),
            PieButton::new(PieDirection::East).with_label("Copy"),
            PieButton::new(PieDirection::West).with_label("Zoom"),
        ]
    }

    fn with_auto(buttons: Vec<PieButton>) -> Rig {
        let mut rig = Rig::new(buttons, Layout::Row);
        rig.menu.settings.input.auto_mnemonics = true;
        rig
    }

    fn typed(c: char) -> Vec<egui::Event> {
        vec![egui::Event::Text(c.to_string())]
    }

    #[test]
    fn pressing_an_auto_mnemonic_selects_the_item_with_its_number() {
        let probe = with_auto(declared());
        let keys = probe.menu.cluster_mnemonics(&probe.buttons);
        assert_eq!(keys.len(), 3);
        assert!(keys[1].is_empty() && keys[2].is_empty());
        for (k, n) in [7usize, 3, 9].into_iter().enumerate() {
            let c = keys[0][k].expect("every item has a key");
            let mut rig = with_auto(declared());
            assert_eq!(rig.frame_with(None, true, typed(c)), PieMenuResponse::Selected(n), "key {c}");
            assert_eq!(rig.menu.selected_path(), &[0, n]);
            assert!(!rig.menu.is_open());
        }
        // Without auto mnemonics there are no keys, and typing does nothing.
        let mut rig = Rig::new(declared(), Layout::Row);
        assert_eq!(rig.menu.cluster_mnemonics(&rig.buttons), vec![vec![None; 3], vec![], vec![]]);
        assert_eq!(rig.frame_with(None, true, typed('p')), PieMenuResponse::None);
        // The use_mnemonic_keys switch gates items like buttons.
        let mut rig = with_auto(declared());
        rig.menu.settings.input.use_mnemonic_keys = false;
        assert_eq!(rig.frame_with(None, true, typed('p')), PieMenuResponse::None);
    }

    #[test]
    fn auto_keys_are_distinct_across_buttons_and_items() {
        let rig = with_auto(declared());
        let m = rig.menu.mnemonics(&rig.buttons);
        let c = rig.menu.cluster_mnemonics(&rig.buttons);
        let mut all: Vec<char> = m.iter().flatten().copied().chain(c.iter().flatten().flatten().copied()).collect();
        assert_eq!(all.len(), 5, "Copy, Zoom and three items all have a key");
        all.sort();
        all.dedup();
        assert_eq!(all.len(), 5, "no key twice: {m:?} {c:?}");
        assert_eq!(m[0], None);
        assert!(rig.menu.check_buttons(&rig.buttons).is_empty());
        // The button side of a clash-prone set: "Copy" and the items "Cut"/"Cursor" all want C.
        let copy = rig.menu.mnemonics(&rig.buttons)[1];
        assert!(c[0].iter().all(|k| *k != copy));
    }

    #[test]
    fn an_explicit_item_key_is_reserved_against_auto_keys() {
        let mut buttons = declared();
        buttons[0] = PieButton::cluster(PieDirection::North).with_items(vec![
            ClusterItem::new(7, "Cut").with_mnemonic('C'),
            ClusterItem::new(3, "Cursor"),
            ClusterItem::new(9, "Paste"),
        ]);
        let rig = with_auto(buttons);
        let c = rig.menu.cluster_mnemonics(&rig.buttons);
        assert_eq!(c[0][0], Some('c'));
        assert_ne!(rig.menu.mnemonics(&rig.buttons)[1], Some('c'), "Copy avoids the reserved c");
        assert!(c[0][1].is_some() && c[0][1] != Some('c'));
        assert!(rig.menu.check_buttons(&rig.buttons).is_empty());
        // Typing it selects the item, not the button.
        let mut rig = rig;
        assert_eq!(rig.frame_with(None, true, typed('c')), PieMenuResponse::Selected(7));
        assert_eq!(rig.menu.selected_path(), &[0, 7]);
    }

    #[test]
    fn items_without_a_key_or_disabled_are_not_selectable_by_key() {
        let buttons = vec![
            PieButton::cluster(PieDirection::North).with_items(vec![
                ClusterItem::new(7, "Cut").without_mnemonic(),
                ClusterItem::new(3, "Cursor").with_enabled(false),
                ClusterItem::new(9, "Paste"),
            ]),
            PieButton::new(PieDirection::East).with_label("Cursor"),
        ];
        let mut rig = with_auto(buttons.clone());
        let c = rig.menu.cluster_mnemonics(&rig.buttons);
        assert_eq!(c[0][..2], [None, None]);
        assert!(c[0][2].is_some());
        // The disabled item reserves nothing: the button takes the C.
        assert_eq!(rig.menu.mnemonics(&rig.buttons)[1], Some('c'));
        assert_eq!(rig.frame_with(None, true, typed('c')), PieMenuResponse::Selected(1));
        // A disabled item is not selectable by click or release either, though drawn enabled.
        let mut rig = with_auto(buttons.clone());
        let pos = rig.centre_of(3);
        assert_eq!(rig.click(pos), PieMenuResponse::None);
        rig.frame(Some(pos), true, &[]);
        assert_eq!(rig.frame(Some(pos), false, &[]), PieMenuResponse::Dismissed);
        // A disabled cluster button: none of its items has a key.
        let mut disabled = buttons;
        disabled[0].enabled = false;
        let rig = with_auto(disabled);
        assert_eq!(rig.menu.cluster_mnemonics(&rig.buttons)[0], vec![None; 3]);
    }

    #[test]
    fn an_undeclared_number_still_works_by_click_but_has_no_key() {
        let buttons = vec![
            PieButton::cluster(PieDirection::North).with_items(items(&[(7, "Cut")])),
            PieButton::new(PieDirection::East).with_label("Copy"),
        ];
        let mut rig = with_auto(buttons);
        let pos = rig.centre_of(3); // drawn by the closure, never declared
        assert_eq!(rig.click(pos), PieMenuResponse::Selected(3));
        assert_eq!(rig.menu.selected_path(), &[0, 3]);
    }

    #[test]
    fn the_clusters_number_key_does_nothing_but_the_others_work() {
        let mut rig = with_auto(declared());
        assert_eq!(rig.frame(None, true, &[Key::Num8]), PieMenuResponse::None);
        assert!(rig.menu.is_open());
        assert_eq!(rig.menu.number_hint(&rig.buttons[0]), None);
        assert_eq!(rig.frame(None, true, &[Key::Num6]), PieMenuResponse::Selected(1));
        let mut rig = with_auto(declared());
        assert_eq!(rig.frame(None, true, &[Key::Num4]), PieMenuResponse::Selected(2));
    }

    #[test]
    fn item_label_underlines_the_items_mnemonic() {
        let rig = with_auto(declared());
        let mut rig = rig;
        rig.settle();
        let keys = rig.menu.cluster_mnemonics(&rig.buttons);
        for (k, n) in [7usize, 3, 9].into_iter().enumerate() {
            let (job, m) = &rig.item_labels[&n];
            assert_eq!(*m, keys[0][k], "item_mnemonic agrees with cluster_mnemonics");
            let underlined: Vec<&str> = job
                .sections
                .iter()
                .filter(|s| s.format.underline != Stroke::NONE)
                .map(|s| &job.text[s.byte_range.start.0..s.byte_range.end.0])
                .collect();
            assert_eq!(underlined.len(), 1, "item {n}: {:?}", job.text);
            assert_eq!(underlined[0].to_lowercase(), m.unwrap().to_string());
        }
        assert_eq!(rig.item_labels[&7].0.text, "Cut");
        assert_eq!(rig.item_labels[&9].0.text, "Paste");
        // An undeclared number: empty label, no key.
        let (job, m) = &rig.item_labels[&999];
        assert!(job.text.is_empty() && m.is_none());
    }

    #[test]
    fn check_buttons_reports_item_number_and_key_clashes() {
        let m = PieMenu::new();
        let dup_numbers = vec![PieButton::cluster(PieDirection::North).with_items(items(&[(4, "A"), (5, "B"), (4, "C")]))];
        let problems = m.check_buttons(&dup_numbers);
        assert_eq!(problems, vec![ButtonProblem::DuplicateItemNumber { cluster: 0, first: 0, second: 2, number: 4 }]);
        assert_eq!(problems[0].buttons(), [0]);
        assert!(problems[0].to_string().contains("number 4"));

        // An item and a button.
        let level = vec![
            PieButton::new(PieDirection::East).with_mnemonic('x'),
            PieButton::cluster(PieDirection::North).with_items(vec![ClusterItem::new(0, "a").with_mnemonic('X')]),
        ];
        let problems = m.check_buttons(&level);
        let expected = ButtonProblem::DuplicateItemMnemonic {
            first: MnemonicOwner::Button(0),
            second: MnemonicOwner::Item { cluster: 1, item: 0 },
            key: 'x',
        };
        assert_eq!(problems, vec![expected]);
        assert_eq!(problems[0].buttons(), [0, 1]);
        assert_eq!(problems[0].to_string(), "button 0 and item 0 of cluster 1 share the mnemonic 'x'");

        // Two items of one cluster; and of two clusters.
        let level = vec![
            PieButton::cluster(PieDirection::North)
                .with_items(vec![ClusterItem::new(0, "a").with_mnemonic('q'), ClusterItem::new(1, "b").with_mnemonic('q')]),
            PieButton::cluster(PieDirection::South).with_items(vec![ClusterItem::new(0, "c").with_mnemonic('q')]),
        ];
        let problems = m.check_buttons(&level);
        assert_eq!(problems.len(), 2);
        assert_eq!(problems[0].buttons(), [0], "same cluster: listed once");
        assert_eq!(problems[1].buttons(), [0, 1]);

        // Disabled items clash with nothing.
        let level = vec![
            PieButton::new(PieDirection::East).with_mnemonic('x'),
            PieButton::cluster(PieDirection::North)
                .with_items(vec![ClusterItem::new(0, "a").with_mnemonic('x').with_enabled(false)]),
        ];
        assert!(m.check_buttons(&level).is_empty());
        // A cluster child of a fan-out is still flagged.
        let fan = vec![PieButton::new(PieDirection::East).with_fan_out(vec![PieButton::cluster(PieDirection::North)])];
        assert!(matches!(
            &m.check_buttons(&fan)[0],
            ButtonProblem::InFanOut { problem, .. } if **problem == ButtonProblem::NestedInFanOut { button: 0 }
        ));
    }

    #[test]
    fn item_mnemonics_in_a_pinned_menu_yield_to_a_focused_text_field() {
        let mut rig = with_auto(declared());
        rig.menu.pin(CENTER);
        rig.settle();
        let c = rig.menu.cluster_mnemonics(&rig.buttons)[0][2].unwrap(); // Paste (9)
        rig.focus_text = true;
        rig.frame(None, true, &[]);
        rig.frame(None, true, &[]);
        assert_eq!(rig.frame_with(None, true, typed(c)), PieMenuResponse::None, "the text field has the key");
        assert!(rig.menu.is_open());
        rig.focus_text = false;
        rig.ctx.memory_mut(|mem| mem.stop_text_input());
        rig.frame(None, true, &[]);
        assert_eq!(rig.frame_with(None, true, typed(c)), PieMenuResponse::Selected(9));
        assert!(rig.menu.is_open() && rig.menu.is_pinned());
    }

    #[test]
    fn a_sub_pie_resolves_item_mnemonics_for_its_own_level() {
        let root = vec![
            PieButton::new(PieDirection::East).with_label("More").with_children(vec![
                PieButton::cluster(PieDirection::North).with_items(items(&[(7, "Cut"), (3, "Zap"), (9, "Paste")])),
                PieButton::new(PieDirection::South).with_label("Down"),
            ]),
            PieButton::new(PieDirection::West).with_label("Cat"),
        ];
        let mut rig = with_auto(root);
        // At the root the C belongs to "Cat".
        assert_eq!(rig.menu.mnemonics(&rig.buttons), [Some('m'), Some('c')]);
        rig.frame(None, true, &[Key::Num6]);
        assert_eq!(rig.menu.path(), &[0]);
        rig.settle();
        // In the sub-pie level the same key belongs to the item "Cut".
        let level = rig.menu.current_buttons(&rig.buttons).to_vec();
        let keys = rig.menu.cluster_mnemonics(&level);
        assert_eq!(keys[0][0], Some('c'));
        assert_eq!(rig.frame_with(None, true, typed('c')), PieMenuResponse::Selected(7));
        assert_eq!(rig.menu.selected_path(), &[0, 0, 7]);
    }
}

mod first_frame_tests {
    //! A button's painted position in the first frame it is painted must be the one it
    //! settles at (no pop), whatever level, fan or opening came before. The per-frame
    //! positions are those of the painted label text in `FullOutput::shapes`.
    use super::*;
    use egui::epaint::Shape;

    const CENTER: Pos2 = Pos2::new(400.0, 300.0);
    const AT_RING: Pos2 = Pos2::new(505.0, 300.0);

    fn collect_texts(shape: &Shape, out: &mut Vec<(String, Pos2)>) {
        match shape {
            Shape::Text(t) => out.push((t.galley.text().to_string(), t.pos)),
            Shape::Vec(v) => v.iter().for_each(|s| collect_texts(s, out)),
            _ => {}
        }
    }

    struct Probe {
        ctx: egui::Context,
        menu: PieMenu,
        buttons: Vec<PieButton>,
        t: f64,
    }

    impl Probe {
        fn new(buttons: Vec<PieButton>, reveal: RevealAnimation) -> Self {
            let mut menu = PieMenu::new();
            menu.settings.animations.reveal = reveal;
            menu.settings.sub_pies.fan_out.delay = Duration::ZERO;
            menu.open(CENTER);
            menu.reveal();
            Self { ctx: egui::Context::default(), menu, buttons, t: 0.0 }
        }

        /// Runs one frame; returns every painted text with its position.
        fn frame(&mut self, pointer: Option<Pos2>, key_down: bool, events: Vec<egui::Event>) -> Vec<(String, Pos2)> {
            self.t += 0.05;
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
                time: Some(self.t),
                events,
                ..Default::default()
            };
            let (menu, buttons) = (&mut self.menu, &self.buttons);
            let mut out = self.ctx.run_ui(input, |ui| {
                menu.show_with_pointer(ui.ctx(), buttons, pointer, key_down, |ui, b| {
                    ui.label(b.button.label.clone());
                });
            });
            out.textures_delta.clear();
            let mut texts = Vec::new();
            out.shapes.iter().for_each(|c| collect_texts(&c.shape, &mut texts));
            texts
        }

        fn key(&mut self, pointer: Option<Pos2>, key: Key) -> Vec<(String, Pos2)> {
            let ev = egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE };
            self.frame(pointer, true, vec![ev])
        }

        fn idle(&mut self, pointer: Option<Pos2>, n: usize) -> Vec<Vec<(String, Pos2)>> {
            (0..n).map(|_| self.frame(pointer, true, Vec::new())).collect()
        }
    }

    fn pos_of(texts: &[(String, Pos2)], label: &str) -> Option<Pos2> {
        texts.iter().find(|(l, _)| l == label).map(|(_, p)| *p)
    }

    /// `frames[0]` is the trigger frame. The first painted position must equal the settled
    /// (last) one; with `RevealAnimation::None` it must be painted by the second frame after.
    fn assert_no_pop(frames: &[Vec<(String, Pos2)>], label: &str, reveal: RevealAnimation, what: &str) {
        let seq: Vec<Option<Pos2>> = frames.iter().map(|f| pos_of(f, label)).collect();
        let first = seq.iter().position(|p| p.is_some()).unwrap_or_else(|| panic!("{what}: {label:?} never painted"));
        let settled = seq.last().unwrap().unwrap_or_else(|| panic!("{what}: {label:?} not painted at the end"));
        let at_first = seq[first].unwrap();
        assert!((at_first - settled).length() < 0.01, "{what}: {label:?} pops from {at_first:?} to {settled:?} ({seq:?})");
        // Each later frame stays put too.
        for p in seq[first..].iter().flatten() {
            assert!((*p - settled).length() < 0.01, "{what}: {label:?} moves later ({seq:?})");
        }
        if matches!(reveal, RevealAnimation::None) {
            assert!(first <= 2, "{what}: {label:?} first painted on frame {first} ({seq:?})");
        }
    }

    const REVEALS: [RevealAnimation; 2] = [RevealAnimation::None, RevealAnimation::Fade];

    fn nested_root() -> Vec<PieButton> {
        vec![
            PieButton::new(PieDirection::East).with_label("A").with_children(vec![
                PieButton::new(PieDirection::North).with_label("A very wide child label"),
                PieButton::new(PieDirection::South).with_label("x"),
            ]),
            PieButton::new(PieDirection::North).with_label("Up"),
            PieButton::new(PieDirection::South).with_label("Down below, a long root label"),
        ]
    }

    fn fan_root() -> Vec<PieButton> {
        vec![
            PieButton::new(PieDirection::East).with_label("More").with_fan_out(vec![
                PieButton::new(PieDirection::North).with_label("Above"),
                PieButton::new(PieDirection::East).with_label("A considerably longer fan child"),
                PieButton::new(PieDirection::South).with_label("Un"),
            ]),
            PieButton::new(PieDirection::North).with_label("Plain"),
        ]
    }

    const FAN_LABELS: [&str; 3] = ["Above", "A considerably longer fan child", "Un"];

    #[test]
    fn first_open_paints_each_button_where_it_settles() {
        for reveal in REVEALS {
            let root = vec![
                PieButton::new(PieDirection::North).with_label("N"),
                PieButton::new(PieDirection::South).with_label("A much longer south label"),
                PieButton::new(PieDirection::West).with_label("West side"),
            ];
            let mut p = Probe::new(root, reveal);
            let frames = p.idle(None, 6);
            for l in ["N", "A much longer south label", "West side"] {
                assert_no_pop(&frames, l, reveal, "first open");
            }
        }
    }

    #[test]
    fn descending_into_a_sub_pie_paints_the_children_where_they_settle() {
        for reveal in REVEALS {
            let mut p = Probe::new(nested_root(), reveal);
            p.idle(None, 4);
            let mut frames = vec![p.key(None, Key::Num6)];
            frames.extend(p.idle(None, 6));
            assert_eq!(p.menu.path(), &[0]);
            for l in ["A very wide child label", "x"] {
                assert_no_pop(&frames, l, reveal, "descend");
            }
        }
    }

    #[test]
    fn going_back_paints_the_root_where_it_settles() {
        for reveal in REVEALS {
            let mut p = Probe::new(nested_root(), reveal);
            p.idle(None, 4);
            p.key(None, Key::Num6);
            p.idle(None, 4);
            let mut frames = vec![p.key(None, Key::Backspace)];
            frames.extend(p.idle(None, 6));
            assert!(p.menu.path().is_empty());
            for l in ["Up", "Down below, a long root label", "A"] {
                assert_no_pop(&frames, l, reveal, "go back");
            }
        }
    }

    fn open_fan(p: &mut Probe) -> Vec<Vec<(String, Pos2)>> {
        let mut frames = vec![p.frame(Some(AT_RING), true, Vec::new())];
        frames.extend(p.idle(Some(AT_RING), 6));
        assert!(p.menu.open_fan_out().is_some());
        frames
    }

    #[test]
    fn opening_a_fan_paints_the_children_where_they_settle_also_the_second_time() {
        for reveal in REVEALS {
            let mut p = Probe::new(fan_root(), reveal);
            p.idle(Some(CENTER), 4);
            let frames = open_fan(&mut p);
            for l in FAN_LABELS {
                assert_no_pop(&frames, l, reveal, "first fan");
            }
            p.key(Some(AT_RING), Key::Backspace);
            assert!(p.menu.open_fan_out().is_none());
            p.idle(Some(CENTER), 4);
            let frames = open_fan(&mut p);
            for l in FAN_LABELS {
                assert_no_pop(&frames, l, reveal, "second fan");
            }
        }
    }

    #[test]
    fn breaking_a_fan_out_never_moves_the_children() {
        for reveal in REVEALS {
            let mut p = Probe::new(fan_root(), reveal);
            p.idle(Some(CENTER), 4);
            open_fan(&mut p);
            // Fully revealed fan: the settled positions.
            p.t += 1.0;
            let settled = p.frame(Some(AT_RING), true, Vec::new());
            let before: Vec<Pos2> = FAN_LABELS.iter().map(|l| pos_of(&settled, l).expect("fan painted")).collect();
            let trigger = p.menu.button_centers[0];
            let click = |pressed| egui::Event::PointerButton { pos: trigger, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
            let mut frames = vec![p.frame(Some(trigger), true, vec![egui::Event::PointerMoved(trigger), click(true), click(false)])];
            assert_eq!(p.menu.path(), &[0], "broken out");
            frames.extend(p.idle(Some(AT_RING), 6));
            for (l, at) in FAN_LABELS.iter().zip(&before) {
                for (i, f) in frames.iter().enumerate() {
                    let q = pos_of(f, l).unwrap_or_else(|| panic!("{l:?} not painted on frame {i} of the break-out"));
                    assert!((q - *at).length() < 0.01, "{l:?} moved on frame {i}: {at:?} -> {q:?}");
                }
            }
        }
    }

    #[test]
    fn closing_and_reopening_paints_each_button_where_it_settles() {
        for reveal in REVEALS {
            let root = vec![
                PieButton::new(PieDirection::North).with_label("N"),
                PieButton::new(PieDirection::South).with_label("A much longer south label"),
            ];
            let mut p = Probe::new(root, reveal);
            p.idle(None, 4);
            p.menu.close();
            p.idle(None, 2);
            p.menu.open(CENTER);
            p.menu.reveal();
            let frames = p.idle(None, 6);
            for l in ["N", "A much longer south label"] {
                assert_no_pop(&frames, l, reveal, "reopen");
            }
        }
    }

    #[test]
    fn visiting_the_same_sub_pie_twice_paints_the_children_where_they_settle() {
        for reveal in REVEALS {
            let mut p = Probe::new(nested_root(), reveal);
            p.idle(None, 4);
            p.key(None, Key::Num6);
            p.idle(None, 4);
            p.key(None, Key::Backspace);
            p.idle(None, 4);
            let mut frames = vec![p.key(None, Key::Num6)];
            frames.extend(p.idle(None, 6));
            assert_eq!(p.menu.path(), &[0]);
            for l in ["A very wide child label", "x"] {
                assert_no_pop(&frames, l, reveal, "second visit");
            }
        }
    }
}

#[cfg(feature = "serde")]
mod serde_tests {
    use super::*;
    use serde::de::IntoDeserializer;
    use serde::de::value::{Error, MapDeserializer};

    fn assert_serde<T: serde::Serialize + serde::de::DeserializeOwned>() {}

    #[test]
    fn settings_implement_serialize_and_deserialize() {
        assert_serde::<PieMenuSettings>();
        assert_serde::<PieMenuLayoutSettings>();
        assert_serde::<PieMenuInput>();
        assert_serde::<ShowBehavior>();
        assert_serde::<HighlightShape>();
        assert_serde::<HoverEmphasis>();
    }

    #[test]
    fn an_empty_map_deserializes_to_the_default() {
        let map = MapDeserializer::<_, Error>::new(std::iter::empty::<(&str, bool)>());
        let settings = <PieMenuSettings as serde::Deserialize>::deserialize(map).unwrap();
        assert_eq!(format!("{settings:?}"), format!("{:?}", PieMenuSettings::default()));
    }

    #[test]
    fn a_partial_map_sets_only_what_it_names() {
        let map = MapDeserializer::<_, Error>::new([("mirror_horizontal", true)].into_iter().map(|(k, v)| (k, v.into_deserializer())));
        let settings = <PieMenuSettings as serde::Deserialize>::deserialize(map).unwrap();
        assert!(settings.mirror_horizontal);
        let expected = PieMenuSettings { mirror_horizontal: true, ..Default::default() };
        assert_eq!(format!("{settings:?}"), format!("{expected:?}"));
    }
}
