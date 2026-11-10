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
        m.settings.sub_pie_center = center;
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
        assert!(root[0].back && root[0].label == "Back");
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
        m.settings.sub_pie_hover_delay = Some(Duration::from_millis(500));
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
        m.settings.sub_pie_hover_delay = Some(Duration::from_millis(500));

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
