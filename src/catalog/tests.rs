#![allow(clippy::field_reassign_with_default)]
use super::*;
use crate::settings::{HighlightShape, HoverEmphasis, RevealAnimation, ShowBehavior};
use crate::{PieMenuDismissInput, button_at};
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
        let (rebuilt, refs) = settings_from_table(&back).unwrap_or_else(|e| panic!("{e}\n{text}"));
        assert!(refs.is_empty());
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
    assert!(bad("[title]\ntext_color = \"red\"").contains("is not a colour"));
    assert!(bad("[center_indicator]\nhighlight_stroke = { width = 1.0, color = \"blue\" }").contains("is not a colour"));
    assert!(bad("[animations]\nhover_duration = -5").contains("negative"));
    assert!(bad("[layout]\nradius = \"big\"").contains("settings"));
}

const EXAMPLE: &str = r##"
[menu.edit]
title = { var = "i18n.edit", default = "Edit" }

[menu.edit.settings]
layout = { radius = 120.0 }
title = { text_color = "#ffcc00" }
animations = { reveal_duration = 150 }
center_indicator = { highlight_fill_color = { var = "theme.accent", default = "#102030" } }

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
label = { var = "i18n.undo", default = "Undo" }

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
        PieButton::new(PieDirection::West).with_label("Undo"),
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
    assert_eq!(edit.bindings.get(&BindTarget::Title), Some(&var_binding("i18n.edit")));
    assert_eq!(edit.bindings.get(&BindTarget::Field(SettingsField::CenterHighlightFillColor)), Some(&var_binding("theme.accent")));
    assert_eq!(edit.settings.center_indicator.highlight_fill_color, Color32::from_rgb(16, 32, 48), "the default until applied");
    assert_eq!(edit.bindings.get(&BindTarget::Label(vec![3])), Some(&var_binding("i18n.undo")));
    assert_eq!(edit.bindings.len(), 3);
    // The sub-pie's menu builds on its own too.
    assert_eq!(catalog(&[("menus.toml", EXAMPLE)]).build("paste_special").unwrap().buttons.len(), 2);
}

#[test]
fn variables_resolve_the_bindings() {
    let mut edit = catalog(&[("menus.toml", EXAMPLE)]).build("edit").unwrap();
    let mut menu = edit.to_pie_menu();
    let mut vars = Variables::new();
    vars.color("theme.accent", Color32::RED).text("i18n.edit", "Bearbeiten").text("i18n.undo", "Rückgängig");
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
fn references_without_defaults_and_merged_over() {
    let base = r##"
        [menu.m]
        title = { var = "i18n.title" }
        settings = { text_color = { var = "theme.text" }, title = { text_font = { var = "theme.font", default = { size = 20.0, family = "Monospace" } } } }
        [[menu.m.button]]
        dir = "N"
        label = { var = "i18n.copy" }
        [[menu.m.button]]
        dir = "S"
        items = [{ number = 1, label = { var = "i18n.bold", default = "B" } }]
    "##;
    let m = catalog(&[("base.toml", base)]).build("m").unwrap();
    assert_eq!(m.title.as_deref(), Some("i18n.title"), "text without a default shows the name");
    assert_eq!(m.buttons[0].label, "i18n.copy");
    assert_eq!(m.buttons[1].cluster_items()[0].label, "B");
    assert_eq!(m.settings.text_color, PieMenuSettings::default().text_color, "a setting without a default keeps the library's");
    assert_eq!(m.settings.title.text_font, FontId::monospace(20.0));
    assert_eq!(m.bindings.len(), 5);
    // A later file's literal replaces the reference (later wins, key by key).
    let over = r##"
        [menu.m]
        title = "Fixed"
        settings = { text_color = "#ffffff" }
    "##;
    let m = catalog(&[("base.toml", base), ("over.toml", over)]).build("m").unwrap();
    assert_eq!(m.title.as_deref(), Some("Fixed"));
    assert_eq!(m.settings.text_color, Some(Color32::WHITE));
    assert!(m.bindings.get(&BindTarget::Title).is_none());
    assert!(m.bindings.get(&BindTarget::Field(SettingsField::TextColor)).is_none());
    assert!(m.bindings.get(&BindTarget::Field(SettingsField::TitleTextFont)).is_some(), "untouched keys keep theirs");
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
    // The same file twice: it loads (another file might still move a button), but the menu
    // does not build, and the error names both files.
    let c = catalog(&[("base.toml", base), ("again.toml", base)]);
    let err = c.build("edit").unwrap_err();
    assert_eq!(err.source, "again.toml");
    assert!(
        err.message.contains("direction N is taken twice: by \"Copy\" (N) (from base.toml) and \"Copy\" (N) (from again.toml)"),
        "{err}"
    );
    // A file with an error adds nothing.
    let mut c = catalog(&[("base.toml", base)]);
    assert!(c.add_toml("bad.toml", "[menu.edit]\ntitle = \"New\"\n[[menu.edit.button]]\ndir = \"Up\"").is_err());
    assert_eq!(c.build("edit").unwrap().title.as_deref(), Some("Edit"));
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
    assert!(add("[menu.a.settings.title]\nshow = { var = \"x\" }").contains("`title.show` cannot come from a variable"));
    assert!(add("[menu.a.settings.title]\ntext_color = { var = \"x\", defualt = \"#fff\" }").contains("defualt"));
    let red = add("[menu.a.settings.title]\ntext_color = { var = \"x\", default = \"red\" }");
    assert!(red.contains("colour"), "{red}");
    assert!(add("[[menu.a.button]]\ndir = \"N\"\nlabel = { name = \"x\" }").contains("label"));
    assert!(add("[menu.a.settings]\nradius = 1.0").contains("unknown setting `radius`"));
    assert!(add("[menu.a\n").starts_with("f.toml: "), "a TOML syntax error names the file");
    // Whether a button is enabled or visible is the program's state, not the file's.
    assert!(add("[[menu.a.button]]\ndir = \"N\"\nenabled = false").contains("unknown field `enabled`"));
    assert!(add("[[menu.a.button]]\ndir = \"N\"\nvisible = false").contains("unknown field `visible`"));
    assert!(add("[[menu.a.button]]\ndir = \"N\"\nitems = [{ number = 1, enabled = false }]").contains("enabled"));
    // Changes.
    assert!(add("[[menu.a.change]]\nlabel = \"x\"").contains("id"), "a change needs an id");
    assert!(add("[[menu.a.change]]\nid = \"x\"\ndir = \"Up\"").contains("unknown direction `Up`"));
    assert!(add("[[menu.a.change]]\nid = \"x\"\nmnemonic = \"ab\"").contains("neither one character"));
    assert!(add("[[menu.a.change]]\nid = \"x\"\nenabled = false").contains("unknown field `enabled`"));
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
                PieButton::new(PieDirection::East).with_label("Leaf"),
            ]),
            PieButton::new(PieDirection::East).with_label("Paste Special").with_children(vec![
                PieButton::back(PieDirection::West),
                PieButton::new(PieDirection::East).with_label("Same name, other menu"),
            ]),
        ]),
        PieButton::cluster(PieDirection::West)
            .with_label("Format")
            .with_items(vec![ClusterItem::new(1, "Bold").with_mnemonic('b'), ClusterItem::new(3, "Italic")]),
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
    // Files keep names, not modes: these come back as Build.
    bindings.set(BindTarget::Title, var_binding("i18n.edit"));
    bindings.set(BindTarget::Field(SettingsField::TitleTextColor), var_binding("theme.accent"));
    bindings.set(BindTarget::Field(SettingsField::CenterHighlightStroke), var_binding("theme.outline"));
    bindings.set(BindTarget::Field(SettingsField::TextColor), var_binding("theme.text"));
    bindings.set(BindTarget::Label(vec![1, 0]), var_binding("i18n.values"));
    bindings.set(BindTarget::ItemLabel(vec![3], 1), var_binding("i18n.bold"));
    let mut settings = rich_settings();
    settings.sub_pies.style = SubPieStyle::Replace; // the tree's sub-pies are not all legal fan-outs
    MenuDefinition { name: "edit".into(), title: Some("Edit".into()), settings, buttons, ids, bindings, ..MenuDefinition::default() }
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
    // References are written in place, with the current value as the default.
    assert!(text.contains("title = { var = \"i18n.edit\", default = \"Edit\" }"), "{text}");
    assert!(text.contains("text_color = { var = \"theme.accent\", default = \"#ffcc00\" }"), "{text}");
    assert!(text.contains("highlight_stroke = { var = \"theme.outline\", default = { width = 2.5, color = \"#0a141e\" } }"), "{text}");
    assert!(text.contains("label = { var = \"i18n.values\", default = \"Values\" }"), "{text}");
    assert!(!text.contains("live") && !text.contains("bind"), "{text}");
    // A Live binding is written as a plain reference (modes are for generated code).
    let mut live = rich_definition();
    live.bindings.set(BindTarget::Label(vec![1, 0]), Binding::new("i18n.values", BindingMode::Live));
    assert_eq!(live.to_toml(), text);
    // One line per value, several tables one per line.
    assert!(text.contains("hover_emphasis = { shift = 2.0, scale = 1.1 }"), "{text}");
    assert!(text.contains("items = [\n    { number = 1, label = { var = \"i18n.bold\", default = \"Bold\" }"), "{text}");
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
    assert_eq!(edit.ids.find("plugin.share.print"), Some(&IdKey { path: vec![6, 4], item: None }));
    assert_eq!(edit.ids.button(&[1, 3]), Some("plugin.paste_image"));
    assert_eq!(edit.ids.selected(&edit.buttons, &[5, 2]), Some("format.italic"));
    assert!(edit.buttons[1].children().len() == 4 && edit.buttons[1].children()[3].label == "As Image");
    // Saved and read back, the merged menu is the same.
    let again = catalog(&[("saved.toml", &edit.to_toml())]).build("edit").unwrap();
    assert_eq!(format!("{:?}", again.buttons), format!("{:?}", edit.buttons));
    assert_eq!(again.ids, edit.ids);
    assert_eq!(again.bindings, edit.bindings);
    // The user's file: in any order with the others, the same menu.
    let user = include_str!("../../examples/menus/user.toml");
    // (The order of the buttons in the tree is the order the files define them.)
    let first = catalog(&[("user.toml", user), ("edit.toml", base), ("plugin.toml", plugin)]).build("edit").unwrap();
    let last = catalog(&[("edit.toml", base), ("plugin.toml", plugin), ("user.toml", user)]).build("edit").unwrap();
    let by_direction = |buttons: &[PieButton]| {
        let mut v: Vec<String> = buttons.iter().map(|b| format!("{b:?}")).collect();
        v.sort();
        v
    };
    assert_eq!(by_direction(&first.buttons), by_direction(&last.buttons));
    for (_, id) in last.ids.iter() {
        let (a, b) = (first.ids.find(id).unwrap(), last.ids.find(id).unwrap());
        let dir = |m: &MenuDefinition, k: &IdKey| button_at(&m.buttons, &k.path).map(|b| b.direction);
        assert_eq!(dir(&first, a), dir(&last, b), "{id}");
    }
    assert!(last.notes.is_empty(), "{:?}", last.notes);
    assert!(last.ids.find("edit.paste_special").is_none() && last.ids.find("paste.values").is_none(), "removed with its sub-pie");
    assert!(last.ids.find("format.underline").is_none() && last.ids.find("format.bold").is_some(), "an item removed from its cluster");
    let cut = last.ids.find("edit.cut").unwrap();
    assert_eq!(button_at(&last.buttons, &cut.path).map(|b| (b.label.as_str(), b.mnemonic)), Some(("Snip", Mnemonic::Key('s'))));
    let share = last.ids.find("plugin.share").unwrap();
    assert_eq!(button_at(&last.buttons, &share.path).map(|b| b.direction), Some(PieDirection::SouthEast), "moved");
    let all = last.ids.find("edit.select_all").unwrap();
    assert_eq!(button_at(&last.buttons, &all.path).map(|b| b.direction), Some(PieDirection::South), "in the place Share left");
    // Without the plugin, its change is only noted.
    let no_plugin = catalog(&[("edit.toml", base), ("user.toml", user)]).build("edit").unwrap();
    let notes: Vec<String> = no_plugin.notes.iter().map(ToString::to_string).collect();
    assert_eq!(notes, ["`change` in menu `edit` (from user.toml): `plugin.share` is not in the menu (nothing changed)"]);
}

fn actions(ids: &[&str]) -> Actions {
    let mut a = Actions::new();
    for id in ids {
        a.add(*id);
    }
    a
}

#[test]
fn changes_and_removals() {
    let base = r##"
        [[menu.m.button]]
        dir = "N"
        label = "Copy"
        id = "copy"
        [[menu.m.button]]
        dir = "S"
        label = "More"
        id = "more"
        opens = "more"
        [[menu.m.button]]
        dir = "W"
        label = "Format"
        id = "format"
        items = [{ number = 1, label = "B", id = "bold" }, { number = 2, label = "I", id = "italic" }]
        [[menu.more.button]]
        dir = "N"
        back = true
        [[menu.more.button]]
        dir = "S"
        label = "Deep"
        id = "deep"
    "##;
    // A change in a file loaded before the button is defined still applies.
    let early = r##"
        [[menu.m.change]]
        id = "copy"
        label = { var = "i18n.dup", default = "Duplicate" }
        dir = "E"
        [[menu.m.change]]
        id = "italic"
        label = "Italic"
        mnemonic = "t"
        [[menu.more.change]]
        id = "deep"
        label = "Deeper"
        [menu.m]
        remove = ["nowhere"]
    "##;
    let late = r##"
        [[menu.m.change]]
        id = "copy"
        mnemonic = "d"
        [[menu.m.change]]
        id = "more"
        items = [{ number = 1, label = "X", id = "x" }]
        [[menu.m.button]]
        dir = "N"
        label = "New in the North"
        id = "new"
        [menu.m]
        remove = ["bold"]
    "##;
    let m = catalog(&[("early.toml", early), ("base.toml", base), ("late.toml", late)]).build("m").unwrap();
    let copy = &m.buttons[0];
    assert_eq!(
        (copy.label.as_str(), copy.direction, copy.mnemonic),
        ("Duplicate", PieDirection::East, Mnemonic::Key('d')),
        "both changes, in file order"
    );
    assert_eq!(m.bindings.get(&BindTarget::Label(vec![0])), Some(&var_binding("i18n.dup")), "a changed label can be a reference");
    assert!(m.buttons[1].is_cluster() && m.buttons[1].children().is_empty(), "a sub-pie changed into a cluster");
    assert_eq!(m.ids.item(&[1], 1), Some("x"));
    let format = &m.buttons[2];
    assert_eq!(format.cluster_items().len(), 1, "bold removed");
    assert_eq!((format.cluster_items()[0].label.as_str(), format.cluster_items()[0].mnemonic), ("Italic", Mnemonic::Key('t')));
    assert_eq!(m.buttons[3].label, "New in the North", "North is free once Copy moved");
    let notes: Vec<String> = m.notes.iter().map(ToString::to_string).collect();
    assert_eq!(notes, ["`remove` in menu `m` (from early.toml): `nowhere` is not in the menu (nothing removed)"]);
    // The sub-pie's own change: the menu `more`, built alone, has it.
    let more = catalog(&[("early.toml", early), ("base.toml", base)]).build("m").unwrap();
    assert_eq!(more.buttons[1].children()[1].label, "Deeper");
    // A notes are reported with the references.
    let found: Vec<String> = m
        .check_references(&Variables::new(), &actions(&["copy", "format", "italic", "new", "more", "x"]))
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(found[0], "`remove` in menu `m` (from early.toml): `nowhere` is not in the menu (nothing removed)");
    // Errors at build: a change that clashes, an item changed like a button.
    let clash = catalog(&[("base.toml", base), ("c.toml", "[[menu.m.change]]\nid = \"copy\"\ndir = \"S\"")]).build("m").unwrap_err();
    assert!(clash.message.contains("direction") && clash.message.contains("taken twice"), "{clash}");
    let item = catalog(&[("base.toml", base), ("c.toml", "[[menu.m.change]]\nid = \"bold\"\ndir = \"S\"")]).build("m").unwrap_err();
    assert_eq!(item.source, "c.toml");
    assert!(item.message.contains("a cluster item can only change its `label` and `mnemonic`"), "{item}");
    let style =
        catalog(&[("base.toml", base), ("c.toml", "[[menu.m.change]]\nid = \"copy\"\nstyle = \"fan_out\"")]).build("m").unwrap_err();
    assert!(style.message.contains("`style` needs `opens`"), "{style}");
}

#[test]
fn actions_set_enabled_and_visible_by_id() {
    let text = r##"
        [[menu.m.button]]
        dir = "N"
        label = "Undo"
        id = "edit.undo"
        [[menu.m.button]]
        dir = "S"
        label = "Share"
        id = "plugin.share"
        opens = "share"
        [[menu.m.button]]
        dir = "E"
        label = "Typo"
        id = "edit.cpoy"
        [[menu.m.button]]
        dir = "W"
        label = "Format"
        items = [{ number = 1, label = "B", id = "format.bold" }, { number = 2, label = "Q", id = "format.qq" }]
        [[menu.m.button]]
        dir = "NE"
        label = "No id"
        [[menu.share.button]]
        dir = "N"
        back = true
        [[menu.share.button]]
        dir = "S"
        label = "Email"
        id = "plugin.share.email"
    "##;
    let mut m = catalog(&[("m.toml", text)]).build("m").unwrap();
    let mut a = actions(&["edit.undo", "plugin.share", "plugin.share.email", "format.bold"]);
    a.add("edit.undo").enabled(false);
    a.add("plugin.share").visible(false);
    a.add("format.bold").enabled(false);
    let fresh = m.apply_actions(&a);
    assert_eq!(fresh, ["edit.cpoy", "format.qq"], "the unregistered ids, once");
    assert!(!m.buttons[0].enabled);
    assert!(!m.buttons[1].visible && m.buttons[1].enabled);
    assert!(!m.buttons[2].enabled, "an unregistered id is disabled");
    assert!(m.is_unhandled(&[2]) && !m.is_unhandled(&[0]));
    assert!(!m.buttons[3].cluster_items()[0].enabled && !m.buttons[3].cluster_items()[1].enabled);
    assert!(m.is_item_unhandled(&[3], 2) && !m.is_item_unhandled(&[3], 1));
    assert!(m.buttons[4].enabled && m.buttons[4].visible, "a button without an id is left alone");
    // Again: nothing new to report, and the state follows the program.
    a.add("edit.undo").enabled(true);
    a.add("plugin.share").visible(true);
    assert!(m.apply_actions(&a).is_empty());
    assert!(m.buttons[0].enabled && m.buttons[1].visible);
    // Registering a typo's id clears it; a new typo is reported once.
    a.add("edit.cpoy");
    assert!(m.apply_actions(&a).is_empty());
    assert!(m.buttons[2].enabled && !m.is_unhandled(&[2]));
    // Keep: unregistered ids are left as they are.
    let mut keep = actions(&["edit.undo"]);
    keep.keep_unregistered(true);
    let mut k = catalog(&[("m.toml", text)]).build("m").unwrap();
    k.apply_actions(&keep);
    assert!(k.buttons[2].enabled && k.is_unhandled(&[2]), "still reported, not disabled");
    // Not written to the file.
    m.buttons[0].enabled = false;
    assert!(!m.to_toml().contains("enabled") && !m.to_toml().contains("visible"));
    // Reported by check_references too.
    let found: Vec<String> = m.check_references(&Variables::new(), &actions(&["edit.undo"])).iter().map(ToString::to_string).collect();
    assert!(found.contains(&"the id of \"Share\": `plugin.share` is not an action of the program".to_string()), "{found:?}");
}

#[test]
fn saving_keeps_the_files_defaults_after_applying() {
    // A program in Russian saves the menu: the file keeps its English defaults.
    let base = include_str!("../../examples/menus/edit.toml");
    let mut edit = catalog(&[("edit.toml", base)]).build("edit").unwrap();
    let mut menu = edit.to_pie_menu();
    let mut vars = Variables::new();
    vars.text("i18n.edit", "Правка").text("i18n.copy", "Копировать").color("theme.accent", Color32::RED);
    edit.apply_variables(vars, &mut menu);
    edit.title = menu.title.clone();
    edit.settings = menu.settings.clone();
    assert_eq!(edit.buttons[0].label, "Копировать");
    let out = edit.to_toml();
    assert!(out.contains("title = { var = \"i18n.edit\", default = \"Edit\" }"), "{out}");
    assert!(out.contains("label = { var = \"i18n.copy\", default = \"Copy\" }"), "{out}");
    assert!(out.contains("highlight_fill_color = { var = \"theme.accent\", default = \"#8000ff\" }"), "{out}");
    assert!(!out.contains("Правка") && !out.contains("Копировать") && !out.contains("#ff0000"), "{out}");
}

#[test]
fn references_are_checked_against_the_program() {
    let text = r##"
        [menu.m]
        title = { var = "i18n.title" }
        settings = { title = { text_font = { var = "theme.accent" } } }
        [[menu.m.button]]
        dir = "N"
        label = { var = "i18n.copy", default = "Copy" }
        id = "edit.copy"
        [[menu.m.button]]
        dir = "S"
        label = "Format"
        items = [{ number = 2, label = "B", id = "fmt.b" }]
    "##;
    let m = catalog(&[("m.toml", text)]).build("m").unwrap();
    let mut vars = Variables::new();
    vars.text("i18n.copy", "Copy").color("theme.accent", Color32::RED);
    let found: Vec<String> = m.check_references(&vars, &actions(&["edit.copy"])).iter().map(ToString::to_string).collect();
    assert_eq!(
        found,
        [
            "the title: `i18n.title` is not a variable of the program",
            "the setting `title.text_font`: `theme.accent` is a colour of the program; this needs a font",
            "the id of item 2 of \"Format\": `fmt.b` is not an action of the program",
        ]
    );
    vars.text("i18n.title", "T").font("theme.accent", FontId::default());
    let found = m.check_references(&vars, &actions(&["edit.copy", "fmt.b"]));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn numbers_and_durations_can_be_references() {
    let text = r##"
        [menu.m.settings.layout]
        radius = { var = "prefs.radius", default = 140.0 }
        intercardinal_height = { var = "prefs.diagonal" }
        [menu.m.settings.animations]
        reveal_duration = { var = "prefs.anim_ms", default = 120 }
        slide_fade = { var = "prefs.fade_ms", default = 80.5 }
        [[menu.m.button]]
        dir = "N"
        label = "A"
    "##;
    let mut m = catalog(&[("m.toml", text)]).build("m").unwrap();
    assert_eq!(m.settings.layout.radius, 140.0);
    assert_eq!(m.settings.layout.intercardinal_height, None, "no default: the library's");
    assert_eq!(m.settings.animations.reveal_duration, Duration::from_millis(120));
    assert_eq!(m.settings.animations.slide_fade, Some(Duration::from_micros(80_500)));
    let mut menu = m.to_pie_menu();
    let mut vars = Variables::new();
    vars.number("prefs.radius", 90.0).number("prefs.diagonal", 0.3).number("prefs.anim_ms", 250.0).number("prefs.fade_ms", 0.0);
    assert!(m.apply_variables(vars.clone(), &mut menu));
    assert_eq!(menu.settings.layout.radius, 90.0);
    assert_eq!(menu.settings.layout.intercardinal_height, Some(0.3));
    assert_eq!(menu.settings.animations.reveal_duration, Duration::from_millis(250));
    assert_eq!(menu.settings.animations.slide_fade, Some(Duration::ZERO));
    assert!(!m.apply_variables(vars, &mut menu), "unchanged values change nothing");
    // Saved with the file's defaults; an optional one that was off has none.
    m.settings = menu.settings.clone();
    let out = m.to_toml();
    assert!(out.contains("radius = { var = \"prefs.radius\", default = 140.0 }"), "{out}");
    assert!(out.contains("intercardinal_height = { var = \"prefs.diagonal\" }"), "{out}");
    assert!(out.contains("reveal_duration = { var = \"prefs.anim_ms\", default = 120.0 }"), "{out}");
    assert!(out.contains("slide_fade = { var = \"prefs.fade_ms\", default = 80.5 }"), "{out}");
    let again = catalog(&[("again.toml", &out)]).build("m").unwrap();
    assert_eq!(format!("{:?}", again.settings), format!("{:?}", catalog(&[("m.toml", text)]).build("m").unwrap().settings));
    // A number where text goes is reported.
    let mut wrong = Variables::new();
    wrong.text("prefs.radius", "big");
    let found: Vec<String> = m.check_references(&wrong, &Actions::new()).iter().map(ToString::to_string).collect();
    assert!(
        found.contains(&"the setting `layout.radius`: `prefs.radius` is text of the program; this needs a number".to_string()),
        "{found:?}"
    );
}
