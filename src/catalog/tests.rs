#![allow(clippy::field_reassign_with_default)]
use super::*;
use crate::PieMenuDismissInput;
use crate::settings::{HighlightShape, HoverEmphasis, RevealAnimation, ShowBehavior};
use egui::{FontId, Stroke};
use std::time::Duration;

/// Settings with a change in every group, optional values switched both ways, a
/// translucent colour and a fractional duration.
fn rich_settings() -> PieMenuSettings {
    let mut s = PieMenuSettings::default();
    s.layout.radius = 120.0;
    s.layout.intercardinal_height = Some(0.25);
    s.show_behavior = ShowBehavior::Instant;
    s.input.auto_mnemonics = true;
    s.input.double_tap_window = Duration::from_millis(250);
    s.input.pinned_dismiss_inputs =
        vec![PieMenuDismissInput::Key(egui::Key::Escape), PieMenuDismissInput::PointerButton(egui::PointerButton::Middle)];
    s.center_indicator.background_radius = None;
    s.center_indicator.highlight_stroke = Stroke::new(2.5, Color32::from_rgb(10, 20, 30));
    s.center_indicator.highlight_fill_color = Color32::from_rgba_unmultiplied(200, 100, 50, 128);
    s.center_indicator.highlight_shape = HighlightShape::ARC;
    s.title.text_color = Color32::from_rgb(255, 204, 0);
    s.title.text_font = FontId::monospace(18.0);
    s.title.padding = egui::Margin::same(3);
    s.animations.reveal = RevealAnimation::SlideOut;
    s.animations.slide_fade = None;
    s.animations.reveal_duration = Duration::from_micros(150_500);
    s.animations.hover_emphasis = HoverEmphasis { shift: 2.0, scale: 1.1 };
    s.sub_pies.style = SubPieStyle::FanOut;
    s.sub_pies.hover_delay = Some(Duration::from_millis(300));
    s.sub_pies.fan_out.radius = Some(140.0);
    s.number_hints.color = Some(Color32::WHITE);
    s.text_color = Some(Color32::BLACK);
    s.mirror_horizontal = true;
    s
}

#[test]
fn settings_round_trip_through_toml_text() {
    for s in [PieMenuSettings::default(), rich_settings(), PieMenuSettings::blender_like()] {
        let table = settings_to_table(&s);
        let text = toml::to_string(&table).unwrap();
        let back: Table = toml::from_str(&text).unwrap();
        let rebuilt = settings_from_table(&back).unwrap_or_else(|e| panic!("{e}\n{text}"));
        assert_eq!(format!("{rebuilt:?}"), format!("{s:?}"), "\n{text}");
    }
    assert!(settings_to_table(&PieMenuSettings::default()).is_empty(), "nothing differs");
}

#[test]
fn settings_are_written_readably() {
    let text = toml::to_string(&settings_to_table(&rich_settings())).unwrap();
    for expected in [
        "text_color = \"#ffcc00\"",             // title colour as hex
        "highlight_fill_color = \"#c7643280\"", // with alpha (premultiplied rounding: 200 → 199)
        "scale = 1.1",                          // f32 written short
        "double_tap_window = 250",              // milliseconds
        "reveal_duration = 150.5",              // fractional milliseconds
        "background_radius = false",            // switched off (on by default)
        "slide_fade = false",
        "hover_delay = 300",
    ] {
        assert!(text.contains(expected), "missing `{expected}` in\n{text}");
    }
    assert!(!text.contains("secs"), "{text}");
}

#[test]
fn every_optional_and_duration_path_is_a_field() {
    let paths: Vec<&str> = SettingsField::ALL.iter().map(|f| f.path()).collect();
    for p in OPTIONALS.iter().chain(DURATIONS.iter()) {
        assert!(paths.contains(p), "{p}");
    }
    // set_none knows every optional.
    let mut all_on = rich_settings();
    all_on.center_indicator.background_radius = Some(1.0);
    all_on.animations.slide_fade = Some(Duration::from_millis(1));
    all_on.animations.slide_start_shape_factor = Some(0.5);
    for p in OPTIONALS {
        let mut s = all_on.clone();
        set_none(&mut s, p);
        assert_ne!(format!("{s:?}"), format!("{all_on:?}"), "{p}");
    }
}

#[test]
fn settings_errors() {
    let bad = |text: &str| settings_from_table(&toml::from_str(text).unwrap()).unwrap_err();
    assert!(bad("[title]\ncolour = \"#fff\"").contains("unknown setting `title.colour`"));
    assert!(bad("radius = 3.0").contains("unknown setting `radius`"));
    assert!(bad("[title]\ntext_color = \"#12345\"").contains("is not a colour"));
    assert!(bad("[animations]\nhover_duration = -5").contains("negative"));
    assert!(bad("[layout]\nradius = \"big\"").contains("settings"));
}

const EXAMPLE: &str = r##"
[menu.edit]
title = "Edit"

[menu.edit.settings]
layout = { radius = 120.0 }
title = { text_color = "#ffcc00" }
animations = { reveal_duration = 150 }

[menu.edit.bind]
"center_indicator.highlight_fill_color" = "theme.accent"
title = { expr = "tr(\"menu.edit\")", live = true }

[[menu.edit.button]]
dir = "N"
label = "Copy"
id = "edit.copy"
mnemonic = "c"

[[menu.edit.button]]
dir = "E"
label = "Paste Special"
opens = "paste_special"
style = "fan_out"

[[menu.edit.button]]
dir = "S"
label = "Format"
items = [{ number = 1, label = "Bold", id = "fmt.bold" }, { number = 2, label = "Italic", mnemonic = "i" }]

[[menu.edit.button]]
dir = "West"
label = "Undo"
enabled = false
bind_label = "tr(\"edit.undo\")"

[[menu.paste_special.button]]
dir = "N"
label = "Values only"
id = "paste.values"

[[menu.paste_special.button]]
dir = "NE"
label = "Formatting"
"##;

fn catalog(files: &[(&str, &str)]) -> MenuCatalog {
    let mut c = MenuCatalog::new();
    for (name, text) in files {
        c.add_toml(name, text).unwrap_or_else(|e| panic!("{e}"));
    }
    c
}

#[test]
fn the_documented_example_builds() {
    let edit = catalog(&[("menus.toml", EXAMPLE)]).build("edit").unwrap();
    let expected = vec![
        PieButton::new(PieDirection::North).with_label("Copy").with_mnemonic('c'),
        PieButton::new(PieDirection::East).with_label("Paste Special").with_fan_out(vec![
            PieButton::new(PieDirection::North).with_label("Values only"),
            PieButton::new(PieDirection::NorthEast).with_label("Formatting"),
        ]),
        PieButton::cluster(PieDirection::South)
            .with_label("Format")
            .with_items(vec![ClusterItem::new(1, "Bold"), ClusterItem::new(2, "Italic").with_mnemonic('i')]),
        PieButton::new(PieDirection::West).with_label("Undo").with_enabled(false),
    ];
    assert_eq!(format!("{:?}", edit.buttons), format!("{expected:?}"));
    assert_eq!(edit.title.as_deref(), Some("Edit"));
    assert_eq!(edit.settings.layout.radius, 120.0);
    assert_eq!(edit.settings.title.text_color, Color32::from_rgb(255, 204, 0));
    assert_eq!(edit.settings.animations.reveal_duration, Duration::from_millis(150));
    assert_eq!(edit.ids.button(&[0]), Some("edit.copy"));
    assert_eq!(edit.ids.button(&[1, 0]), Some("paste.values"));
    assert_eq!(edit.ids.item(&[2], 1), Some("fmt.bold"));
    assert_eq!(edit.ids.selected(&edit.buttons, &[2, 1]), Some("fmt.bold"));
    assert_eq!(edit.bindings.get(&BindTarget::Title), Some(&Binding::new("tr(\"menu.edit\")", BindingMode::Live)));
    assert_eq!(
        edit.bindings.get(&BindTarget::Field(SettingsField::CenterHighlightFillColor)),
        Some(&Binding::new("theme.accent", BindingMode::Build))
    );
    assert_eq!(edit.bindings.get(&BindTarget::Label(vec![3])), Some(&Binding::new("tr(\"edit.undo\")", BindingMode::Build)));
    // The sub-pie's menu builds on its own too.
    assert_eq!(catalog(&[("menus.toml", EXAMPLE)]).build("paste_special").unwrap().buttons.len(), 2);
}

#[test]
fn variables_resolve_the_bindings() {
    let mut edit = catalog(&[("menus.toml", EXAMPLE)]).build("edit").unwrap();
    let mut menu = edit.to_pie_menu();
    let mut vars = Variables::new();
    vars.color("theme.accent", Color32::RED).text("tr(\"menu.edit\")", "Bearbeiten").text("tr(\"edit.undo\")", "Rückgängig");
    assert!(edit.apply_variables(vars.clone(), &mut menu));
    assert_eq!(menu.title.as_deref(), Some("Bearbeiten"));
    assert_eq!(menu.settings.center_indicator.highlight_fill_color, Color32::RED);
    assert_eq!(edit.buttons[3].label, "Rückgängig");
    assert!(!edit.apply_variables(vars, &mut menu), "unchanged values change nothing");
    // Without the variables the values from the file stay.
    let mut plain = catalog(&[("menus.toml", EXAMPLE)]).build("edit").unwrap();
    let mut menu = plain.to_pie_menu();
    assert!(!plain.apply_variables(Variables::new(), &mut menu));
    assert_eq!(menu.title.as_deref(), Some("Edit"));
}

#[test]
fn files_merge() {
    let base = r##"
        [menu.edit]
        title = "Edit"
        settings = { layout = { radius = 120.0, dead_zone_radius = 20.0 } }
        [[menu.edit.button]]
        dir = "N"
        label = "Copy"
    "##;
    let plugin = r##"
        [menu.edit]
        title = "Edit+"
        settings = { layout = { radius = 150.0 }, text_color = "#ffffff" }
        [[menu.edit.button]]
        dir = "S"
        label = "Translate"
        opens = "translate"
        [[menu.translate.button]]
        dir = "S"
        label = "To English"
        id = "translate.en"
        [[menu.translate.button]]
        dir = "N"
        back = true
    "##;
    let c = catalog(&[("base.toml", base), ("plugin.toml", plugin)]);
    assert_eq!(c.menu_names().collect::<Vec<_>>(), ["edit", "translate"]);
    let edit = c.build("edit").unwrap();
    assert_eq!(edit.buttons.len(), 2);
    assert_eq!(edit.title.as_deref(), Some("Edit+"), "later replaces");
    assert_eq!(edit.settings.layout.radius, 150.0, "later replaces per key");
    assert_eq!(edit.settings.layout.dead_zone_radius, 20.0, "earlier keys of the group stay");
    assert_eq!(edit.settings.text_color, Some(Color32::WHITE));
    assert_eq!(edit.ids.button(&[1, 0]), Some("translate.en"));
    assert!(edit.buttons[1].children()[1].is_back());
    // The same file twice: every direction is taken twice.
    let mut c = catalog(&[("base.toml", base)]);
    let err = c.add_toml("again.toml", base).unwrap_err();
    assert_eq!(err.source, "again.toml");
    assert!(
        err.message.contains("direction N is taken twice: by \"Copy\" (N) (from base.toml) and \"Copy\" (N) (from again.toml)"),
        "{err}"
    );
    assert_eq!(c.build("edit").unwrap().title.as_deref(), Some("Edit"), "a failed file adds nothing");
}

#[test]
fn file_errors() {
    let add = |text: &str| MenuCatalog::new().add_toml("f.toml", text).unwrap_err().to_string();
    assert!(add("[menu.a]\ntitel = \"x\"").contains("titel"), "unknown key");
    assert!(add("[[menu.a.button]]\ndir = \"Up\"").contains("unknown direction `Up`"));
    assert!(add("[[menu.a.button]]\ndir = \"N\"\nback = true\nopens = \"b\"").contains("exclude each other"));
    assert!(add("[[menu.a.button]]\ndir = \"N\"\nstyle = \"fan_out\"").contains("`style` needs `opens`"));
    assert!(add("[[menu.a.button]]\ndir = \"N\"\nmnemonic = \"ab\"").contains("neither one character"));
    assert!(add("[[menu.a.button]]\ndir = \"N\"\nitems = []\nmnemonic = \"a\"").contains("no mnemonic of its own"));
    assert!(add("[menu.a.bind]\n\"layout.radius\" = \"x\"").contains("bindable"));
    assert!(add("[menu.a.settings]\nradius = 1.0").contains("unknown setting `radius`"));
    assert!(add("[menu.a\n").starts_with("f.toml: "), "a TOML syntax error names the file");
}

#[test]
fn build_errors() {
    let build = |text: &str, name: &str| catalog(&[("f.toml", text)]).build(name).unwrap_err().to_string();
    assert!(build("[[menu.a.button]]\ndir = \"N\"\nopens = \"nowhere\"", "a").contains("opens `nowhere`, which no file defines"));
    let circle = "[[menu.a.button]]\ndir = \"N\"\nopens = \"b\"\n[[menu.b.button]]\ndir = \"N\"\nopens = \"a\"";
    assert!(build(circle, "a").contains("menus open each other: a \u{2192} b \u{2192} a"), "{}", build(circle, "a"));
    let twice = "[[menu.a.button]]\ndir = \"N\"\nlabel = \"One\"\nmnemonic = \"x\"\n[[menu.a.button]]\ndir = \"S\"\nlabel = \"Two\"\nmnemonic = \"x\"";
    assert!(build(twice, "a").contains("mnemonic"), "{}", build(twice, "a"));
    assert_eq!(MenuCatalog::new().build("x").unwrap_err().message, "no file defines this menu");
}

/// Every kind of button, nested sub-pies of both styles, ids and bindings of every kind.
fn rich_definition() -> MenuDefinition {
    let buttons = vec![
        PieButton::new(PieDirection::North).with_label("Copy").with_mnemonic('c'),
        PieButton::new(PieDirection::East).with_label("Paste Special").with_fan_out(vec![
            PieButton::new(PieDirection::North).with_label("Values"),
            PieButton::new(PieDirection::East).with_label("Formats").without_mnemonic(),
        ]),
        PieButton::new(PieDirection::South).with_label("More").with_children(vec![
            PieButton::back(PieDirection::North),
            PieButton::new(PieDirection::South).with_label("Deeper").with_children(vec![
                PieButton::back(PieDirection::North).with_label("Up"),
                PieButton::new(PieDirection::East).with_label("Leaf").with_enabled(false),
            ]),
            PieButton::new(PieDirection::East).with_label("Paste Special").with_children(vec![
                PieButton::back(PieDirection::West),
                PieButton::new(PieDirection::East).with_label("Same name, other menu"),
            ]),
        ]),
        PieButton::cluster(PieDirection::West)
            .with_label("Format")
            .with_items(vec![ClusterItem::new(1, "Bold").with_mnemonic('b'), ClusterItem::new(3, "Italic").with_enabled(false)]),
        PieButton::new(PieDirection::NorthWest)
            .with_label("Sub")
            .with_children(vec![PieButton::back(PieDirection::SouthEast)])
            .with_sub_pie_style(Some(SubPieStyle::Replace)),
    ];
    let mut ids = ButtonIds::new();
    ids.set(IdKey { path: vec![0], item: None }, "edit.copy");
    ids.set(IdKey { path: vec![2, 1, 1], item: None }, "deep.leaf");
    ids.set(IdKey { path: vec![3], item: Some(3) }, "fmt.italic");
    let mut bindings = Bindings::new();
    bindings.set(BindTarget::Title, Binding::new("tr(\"edit\")", BindingMode::Build));
    bindings.set(BindTarget::Field(SettingsField::TitleTextColor), Binding::new("theme.accent", BindingMode::Live));
    bindings.set(BindTarget::Label(vec![1, 0]), Binding::new("tr(\"values\")", BindingMode::Build));
    bindings.set(BindTarget::ItemLabel(vec![3], 1), Binding::new("tr(\"bold\")", BindingMode::Live));
    let mut settings = rich_settings();
    settings.sub_pies.style = SubPieStyle::Replace; // the tree's sub-pies are not all legal fan-outs
    MenuDefinition { name: "edit".into(), title: Some("Edit".into()), settings, buttons, ids, bindings }
}

#[test]
fn definitions_round_trip_through_toml() {
    let def = rich_definition();
    let text = def.to_toml();
    let back = catalog(&[("saved.toml", &text)]).build("edit").unwrap_or_else(|e| panic!("{e}\n{text}"));
    assert_eq!(format!("{:?}", back.buttons), format!("{:?}", def.buttons), "\n{text}");
    assert_eq!(format!("{:?}", back.settings), format!("{:?}", def.settings));
    assert_eq!(back.title, def.title);
    assert_eq!(back.ids, def.ids);
    assert_eq!(back.bindings, def.bindings);
    // One line per value, several tables one per line.
    assert!(text.contains("highlight_stroke = { width = 2.5, color = \"#0a141e\" }"), "{text}");
    assert!(text.contains("items = [\n    { number = 1, label = \"Bold\""), "{text}");
    // The root menu first, sub-pies named after their labels, unique.
    assert!(text.starts_with("[menu.edit]"), "{text}");
    for name in ["[[menu.paste_special.button]]", "[[menu.more.button]]", "[[menu.deeper.button]]", "[[menu.paste_special_2.button]]"] {
        assert!(text.contains(name), "missing {name} in\n{text}");
    }
    // An empty definition writes and reads back.
    let empty = MenuDefinition { name: "e".into(), ..MenuDefinition::default() };
    let text = empty.to_toml();
    assert_eq!(text.trim(), "[menu.e]");
    let back = catalog(&[("e.toml", &text)]).build("e").unwrap();
    assert!(back.buttons.is_empty());
}

#[test]
fn menu_names_from_labels() {
    let mut taken = vec!["edit".to_string()];
    assert_eq!(unique_name("Paste Special…", &[1], &mut taken), "paste_special");
    assert_eq!(unique_name("Paste  special", &[2], &mut taken), "paste_special_2");
    assert_eq!(unique_name("", &[2, 3], &mut taken), "sub_2_3");
    assert_eq!(unique_name("Edit", &[4], &mut taken), "edit_2");
    assert_eq!(unique_name("Копировать", &[5], &mut taken), "копировать");
}

#[test]
fn directions_and_mnemonics() {
    for (short, d) in DIRECTION_NAMES {
        assert_eq!(parse_direction(short), Some(d));
        assert_eq!(parse_direction(&format!("{d:?}").to_lowercase()), Some(d));
        assert_eq!(direction_name(d), short);
    }
    assert_eq!(parse_mnemonic("x"), Ok(Mnemonic::Key('x')));
    assert_eq!(parse_mnemonic("None"), Ok(Mnemonic::None));
    assert!(parse_mnemonic("").is_err());
}

#[test]
fn the_example_files_build_and_merge() {
    let base = include_str!("../../examples/menus/edit.toml");
    let plugin = include_str!("../../examples/menus/plugin.toml");
    let alone = catalog(&[("edit.toml", base)]).build("edit").unwrap();
    assert_eq!(alone.buttons.len(), 6);
    assert_eq!(alone.settings.layout.radius, 110.0);
    let both = catalog(&[("edit.toml", base), ("plugin.toml", plugin)]);
    let edit = both.build("edit").unwrap();
    assert_eq!(edit.buttons.len(), 7);
    assert_eq!(edit.settings.layout.radius, 130.0);
    assert_eq!(edit.settings.animations.reveal_duration, Duration::from_millis(150));
    assert_eq!(edit.ids.find("plugin.share.chat"), Some(&IdKey { path: vec![6, 2], item: None }));
    assert_eq!(edit.ids.button(&[1, 3]), Some("plugin.paste_image"));
    assert_eq!(edit.ids.selected(&edit.buttons, &[5, 2]), Some("format.italic"));
    assert!(edit.buttons[1].children().len() == 4 && edit.buttons[1].children()[3].label == "As Image");
    // Saved and read back, the merged menu is the same.
    let again = catalog(&[("saved.toml", &edit.to_toml())]).build("edit").unwrap();
    assert_eq!(format!("{:?}", again.buttons), format!("{:?}", edit.buttons));
    assert_eq!(again.ids, edit.ids);
    assert_eq!(again.bindings, edit.bindings);
}
