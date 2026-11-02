use eframe::egui;
use egui::{Color32, RichText, Vec2};
use std::time::Duration;
use egui_pie_menu::{mnemonic_text, PieButton, PieDirection, PieMenu, PieMenuHighlightShape, HintSide, NumberHint, PieMenuResponse, PieMenuSettings, RevealAnimation, ShowBehavior, SmartFloat, TextFormat};

fn main() -> eframe::Result {
    eframe::run_native(
        "egui_pie_menu demo",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(Demo::new()))),
    )
}

struct Demo {
    menu: PieMenu,
    menu_open: bool,
    last_action: String,
    pinned: bool,

    // Mirrored f32 values for SmartFloat settings (sliders need plain f32)
    bg_radius: f32,
    highlight_radius: f32,
    highlight_angle_deg: f32,

    buttons: Vec<PieButton>,

    // State for the checkbox button slot
    word_wrap: bool,

    // Mirror for show threshold slider (only active in OnMovement mode)
    show_threshold: f32,

    // Side-panel search query
    search: String,

    // Mirror for the slide-out start shape slider (kept while "Own start shape" is off)
    slide_start_shape: f32,
    // Mirror for the slide-out fade duration slider (kept while "Fade while sliding" is off)
    slide_fade_ms: u64,
    // Colour picker value for number hints (kept while "Custom colour" is off)
    hint_color: Color32,
    // Number hints on/off, and whether they go inside the buttons or are painted beside
    show_hints: bool,
    hints_inside: bool,
}

impl Demo {
    fn new() -> Self {
        let buttons = vec![
            PieButton::new(PieDirection::North)    .with_mnemonic('c'),
            PieButton::new(PieDirection::NorthEast).with_mnemonic('p'),
            PieButton::new(PieDirection::East)     .with_mnemonic('r'),
            PieButton::new(PieDirection::SouthEast).with_mnemonic('s'),
            PieButton::new(PieDirection::South)    .with_mnemonic('d'),
            PieButton::new(PieDirection::SouthWest).with_mnemonic('t'),
            PieButton::new(PieDirection::West)     .with_mnemonic('u'),
            PieButton::new(PieDirection::NorthWest),
        ];

        let menu = PieMenu::new();
        let bg_radius = menu.settings.center_indicator.background_radius.get();
        let highlight_radius = menu.settings.center_indicator.highlight_radius.get();
        let highlight_angle_deg = menu.settings.center_indicator.highlight_angle.to_degrees();

        Self {
            menu,
            menu_open: false,
            last_action: "Right-click anywhere to open the menu.".to_string(),
            pinned: false,
            bg_radius,
            highlight_radius,
            highlight_angle_deg,
            buttons,
            word_wrap: false,
            show_threshold: DEFAULT_SHOW_THRESHOLD,
            search: String::new(),
            slide_start_shape: DEFAULT_SLIDE_START_SHAPE,
            slide_fade_ms: 200,
            hint_color: Color32::GRAY,
            show_hints: false,
            hints_inside: true,
        }
    }
}

/// Demo-side defaults for values the library settings don't hold directly.
const DEFAULT_SHOW_THRESHOLD: f32 = 10.0;
const DEFAULT_SLIDE_START_SHAPE: f32 = -1.0; // start the slide as a diamond

const LABELS: &[&str] = &[
    "Copy", "Paste Special", "Redo", "Save As…",
    "Delete Permanently", "Cut", "Undo Last Change", "Word wrap",
];

const COLORS: &[Color32] = &[
    Color32::from_rgb( 70, 130, 180),
    Color32::from_rgb( 70, 130, 180),
    Color32::from_rgb(100, 160, 100),
    Color32::from_rgb(180, 140,  60),
    Color32::from_rgb(180,  70,  70),
    Color32::from_rgb(180, 100,  60),
    Color32::from_rgb(100, 160, 100),
    Color32::from_rgb(130,  90, 180),
];

// Keyboard shortcut hints shown in tooltips
const TOOLTIPS: &[&str] = &[
    "Ctrl+C", "Ctrl+Shift+V", "Ctrl+Y", "Ctrl+Shift+S",
    "Del", "Ctrl+X", "Ctrl+Z", "",
];

const ALL_REVEALS: &[RevealAnimation] = &[
    RevealAnimation::None,
    RevealAnimation::Fade,
    RevealAnimation::SlideOut,
];

fn reveal_label(r: RevealAnimation) -> &'static str {
    match r {
        RevealAnimation::None     => "None (instant)",
        RevealAnimation::Fade     => "Fade",
        RevealAnimation::SlideOut => "Slide out",
    }
}

/// Draws a settings section, unless a search query is active and matches
/// neither the section title nor its keywords (both case-insensitive).
fn section(
    ui: &mut egui::Ui,
    query: &str,
    shown: &mut usize,
    title: &str,
    keywords: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    if !query.is_empty()
        && !title.to_lowercase().contains(query)
        && !keywords.to_lowercase().contains(query)
    {
        return;
    }
    *shown += 1;
    ui.heading(title);
    ui.separator();
    add_contents(ui);
    ui.add_space(8.0);
}

/// Small "⟲" button that resets a value; disabled while the value is already the default.
fn reset_button(ui: &mut egui::Ui, is_default: bool) -> bool {
    ui.add_enabled(!is_default, egui::Button::new("⟲").small())
        .on_hover_text("Reset to default")
        .clicked()
}

/// A slider with a reset button beside it. Returns true if the value changed.
fn slider_reset<N: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    value: &mut N,
    default: N,
    range: std::ops::RangeInclusive<N>,
    step: f64,
) -> bool {
    ui.horizontal(|ui| {
        let mut changed = ui.add(egui::Slider::new(value, range).step_by(step)).changed();
        if reset_button(ui, *value == default) {
            *value = default;
            changed = true;
        }
        changed
    })
    .inner
}

/// A button with an optional number hint before or after its label (`true` = after),
/// so the button grows to fit the digit.
fn hint_button(label: impl Into<egui::WidgetText>, hint: Option<(RichText, bool)>, gap: f32) -> egui::Button<'static> {
    let label: egui::WidgetText = label.into();
    match hint {
        Some((h, true))  => egui::Button::new((label, h)).gap(gap),
        Some((h, false)) => egui::Button::new((h, label)).gap(gap),
        None             => egui::Button::new(label),
    }
}

const ALL_HINT_SIDES: &[HintSide] = &[HintSide::Trailing, HintSide::Outside, HintSide::Left, HintSide::Right];

fn hint_side_label(s: HintSide) -> &'static str {
    match s {
        HintSide::Trailing => "After label (reading order)",
        HintSide::Outside => "Outside (away from centre)",
        HintSide::Left    => "Left",
        HintSide::Right   => "Right",
    }
}

const ALL_SHAPES: &[PieMenuHighlightShape] = &[
    PieMenuHighlightShape::None,
    PieMenuHighlightShape::Arc,
    PieMenuHighlightShape::Slice,
    PieMenuHighlightShape::Circle,
    PieMenuHighlightShape::ArcSlice,
    PieMenuHighlightShape::ArcCircle,
    PieMenuHighlightShape::SliceCircle,
    PieMenuHighlightShape::ArcSliceCircle,
];

fn shape_label(s: PieMenuHighlightShape) -> &'static str {
    match s {
        PieMenuHighlightShape::None           => "None",
        PieMenuHighlightShape::Arc            => "Arc",
        PieMenuHighlightShape::Slice          => "Slice",
        PieMenuHighlightShape::Circle         => "Circle",
        PieMenuHighlightShape::ArcSlice       => "Arc + Slice",
        PieMenuHighlightShape::ArcCircle      => "Arc + Circle",
        PieMenuHighlightShape::SliceCircle    => "Slice + Circle",
        PieMenuHighlightShape::ArcSliceCircle => "Arc + Slice + Circle",
    }
}

impl eframe::App for Demo {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let ctx = &ctx;
        let d = PieMenuSettings::default(); // defaults for the reset buttons
        egui::Panel::left("settings").min_size(220.0).show(ui, |ui| {
            // ── Search ───────────────────────────────────────────────
            ui.add_space(4.0);
            // One fixed-height row laid out right-to-left: the clear button takes its
            // space first, then the text field fills exactly what is left, so the row
            // never overflows (overflow made the resizable panel grow every frame).
            // The button is always present (just disabled when empty) and the text
            // field has a fixed id, so typing doesn't shift widget ids and drop focus.
            let row = egui::vec2(ui.available_width(), ui.spacing().interact_size.y);
            ui.allocate_ui_with_layout(row, egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let has_text = !self.search.is_empty();
                if ui
                    .add_enabled(has_text, egui::Button::new("✖").small())
                    .on_hover_text("Clear search")
                    .clicked()
                {
                    self.search.clear();
                }
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .id(egui::Id::new("settings_search"))
                        .hint_text("🔍 Search settings…")
                        .desired_width(f32::INFINITY),
                );
            });
            ui.separator();

            let query = self.search.trim().to_lowercase();
            let mut shown = 0;

            egui::ScrollArea::vertical().show(ui, |ui| {
                section(ui, &query, &mut shown, "Layout", "shape factor diamond circle square radius", |ui| {
                    ui.label("Shape factor  (−1 diamond · 0 circle · +1 square)");
                    slider_reset(ui, &mut self.menu.settings.shape_factor, d.shape_factor, -1.0..=1.0, 0.01);

                    ui.label("Radius");
                    slider_reset(ui, &mut self.menu.settings.layout_radius, d.layout_radius, 40.0..=200.0, 1.0);
                });

                section(ui, &query, &mut shown, "Center background", "radius fill color colour stroke width", |ui| {
                    ui.label("Radius");
                    let def = d.center_indicator.background_radius.get();
                    if slider_reset(ui, &mut self.bg_radius, def, 0.0..=60.0, 0.5) {
                        self.menu.settings.center_indicator.background_radius = SmartFloat::new(self.bg_radius);
                    }

                    ui.label("Fill color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.center_indicator.background_fill_color);

                    ui.label("Stroke width");
                    slider_reset(ui, &mut self.menu.settings.center_indicator.background_stroke.width,
                        d.center_indicator.background_stroke.width, 0.0..=10.0, 0.5);

                    ui.label("Stroke color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.center_indicator.background_stroke.color);
                });

                section(ui, &query, &mut shown, "Highlight",
                    "shape arc slice circle radius angle fill color colour stroke width snap button", |ui| {
                    ui.label("Shape");
                    let current = self.menu.settings.center_indicator.highlight_shape;
                    egui::ComboBox::from_id_salt("shape")
                        .selected_text(shape_label(current))
                        .show_ui(ui, |ui| {
                            for &s in ALL_SHAPES {
                                ui.selectable_value(
                                    &mut self.menu.settings.center_indicator.highlight_shape,
                                    s, shape_label(s),
                                );
                            }
                        });

                    ui.label("Radius");
                    let def = d.center_indicator.highlight_radius.get();
                    if slider_reset(ui, &mut self.highlight_radius, def, 1.0..=60.0, 0.5) {
                        self.menu.settings.center_indicator.highlight_radius = SmartFloat::new(self.highlight_radius);
                    }

                    ui.label("Arc angle (°)");
                    let def = d.center_indicator.highlight_angle.to_degrees();
                    if slider_reset(ui, &mut self.highlight_angle_deg, def, 5.0..=360.0, 1.0) {
                        self.menu.settings.center_indicator.highlight_angle = self.highlight_angle_deg.to_radians();
                    }

                    ui.label("Circle radius");
                    slider_reset(ui, &mut self.menu.settings.center_indicator.highlight_circle_radius,
                        d.center_indicator.highlight_circle_radius, 1.0..=40.0, 0.5);

                    ui.label("Fill color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.center_indicator.highlight_fill_color);

                    ui.label("Stroke width");
                    slider_reset(ui, &mut self.menu.settings.center_indicator.highlight_stroke.width,
                        d.center_indicator.highlight_stroke.width, 0.0..=10.0, 0.5);

                    ui.label("Stroke color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.center_indicator.highlight_stroke.color);

                    ui.checkbox(&mut self.menu.settings.animations.center_highlight_snapping, "Snap to button");
                });

                section(ui, &query, &mut shown, "Reveal animation",
                    "animation fade none instant slide slideout duration start radius shape factor diamond square opacity", |ui| {
                    let current = self.menu.settings.animations.reveal;
                    egui::ComboBox::from_id_salt("reveal")
                        .selected_text(reveal_label(current))
                        .show_ui(ui, |ui| {
                            for &r in ALL_REVEALS {
                                ui.selectable_value(&mut self.menu.settings.animations.reveal, r, reveal_label(r));
                            }
                        });

                    let reveal = self.menu.settings.animations.reveal;
                    if matches!(reveal, RevealAnimation::Fade | RevealAnimation::SlideOut) {
                        ui.label("Duration (ms)");
                        let mut ms = self.menu.settings.animations.reveal_duration.as_millis() as u64;
                        let def = d.animations.reveal_duration.as_millis() as u64;
                        if slider_reset(ui, &mut ms, def, 0..=1000, 1.0) {
                            self.menu.settings.animations.reveal_duration = Duration::from_millis(ms);
                        }
                    }

                    if reveal == RevealAnimation::SlideOut {
                        // Fade first, right under the main duration.
                        let mut fade = self.menu.settings.animations.slide_fade.is_some();
                        if ui.checkbox(&mut fade, "Fade while sliding").changed() {
                            self.menu.settings.animations.slide_fade =
                                fade.then(|| Duration::from_millis(self.slide_fade_ms));
                        }
                        ui.add_enabled_ui(fade, |ui| {
                            ui.label("Fade duration (ms)");
                            let def = d.animations.slide_fade.map_or(200, |f| f.as_millis() as u64);
                            if slider_reset(ui, &mut self.slide_fade_ms, def, 0..=1000, 1.0) {
                                self.menu.settings.animations.slide_fade = Some(Duration::from_millis(self.slide_fade_ms));
                            }
                        });

                        ui.label("Start radius  (0 = from the centre)");
                        slider_reset(ui, &mut self.menu.settings.animations.slide_start_radius,
                            d.animations.slide_start_radius, 0.0..=300.0, 1.0);

                        let mut own_shape = self.menu.settings.animations.slide_start_shape_factor.is_some();
                        if ui.checkbox(&mut own_shape, "Own start shape").changed() {
                            self.menu.settings.animations.slide_start_shape_factor =
                                own_shape.then_some(self.slide_start_shape);
                        }
                        ui.add_enabled_ui(own_shape, |ui| {
                            ui.label("Start shape  (−1 diamond · 0 circle · +1 square)");
                            if slider_reset(ui, &mut self.slide_start_shape, DEFAULT_SLIDE_START_SHAPE, -1.0..=1.0, 0.01) {
                                self.menu.settings.animations.slide_start_shape_factor = Some(self.slide_start_shape);
                            }
                        });
                    }
                });

                section(ui, &query, &mut shown, "Center label",
                    "title text color colour background stroke width", |ui| {
                    ui.checkbox(&mut self.menu.settings.label.display, "Show title");
                    ui.label("Text color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.label.text_color);
                    ui.label("Background color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.label.background_color);
                    ui.label("Stroke width");
                    slider_reset(ui, &mut self.menu.settings.label.background_stroke.width,
                        d.label.background_stroke.width, 0.0..=4.0, 0.5);
                    ui.label("Stroke color");
                    ui.color_edit_button_srgba(&mut self.menu.settings.label.background_stroke.color);
                });

                section(ui, &query, &mut shown, "Show behaviour",
                    "behavior instant movement threshold quick tap double tap", |ui| {
                    let is_instant = matches!(self.menu.settings.show_behavior, ShowBehavior::Instant);
                    egui::ComboBox::from_id_salt("show_behavior")
                        .selected_text(if is_instant { "Instant" } else { "On movement" })
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(is_instant, "Instant").clicked() {
                                self.menu.settings.show_behavior = ShowBehavior::Instant;
                            }
                            if ui.selectable_label(!is_instant, "On movement").clicked() {
                                self.menu.settings.show_behavior =
                                    ShowBehavior::OnMovement { threshold: self.show_threshold };
                            }
                        });

                    if let ShowBehavior::OnMovement { ref mut threshold } = self.menu.settings.show_behavior {
                        ui.label("Threshold (px)");
                        if slider_reset(ui, threshold, DEFAULT_SHOW_THRESHOLD, 0.0..=50.0, 0.5) {
                            self.show_threshold = *threshold;
                        }
                    }

                    if is_instant {
                        ui.label("QuickTap / DoubleTap not available.");
                    }
                });

                section(ui, &query, &mut shown, "Number hints",
                    "number hints digits keys inside beside painted side trailing after outside left right spacing gap color colour", |ui| {
                    ui.checkbox(&mut self.show_hints, "Show number hints");
                    ui.add_enabled_ui(self.show_hints, |ui| {
                        ui.horizontal(|ui| {
                            ui.radio_value(&mut self.hints_inside, true, "Inside button");
                            ui.radio_value(&mut self.hints_inside, false, "Beside (painted)");
                        });

                        ui.label("Side");
                        let current = self.menu.settings.number_hints.side;
                        egui::ComboBox::from_id_salt("hint_side")
                            .selected_text(hint_side_label(current))
                            .show_ui(ui, |ui| {
                                for &s in ALL_HINT_SIDES {
                                    ui.selectable_value(&mut self.menu.settings.number_hints.side, s, hint_side_label(s));
                                }
                            });

                        ui.label("Spacing (px)");
                        slider_reset(ui, &mut self.menu.settings.number_hints.spacing,
                            d.number_hints.spacing, 0.0..=40.0, 0.5);

                        let mut custom = self.menu.settings.number_hints.color.is_some();
                        if ui.checkbox(&mut custom, "Custom colour")
                            .on_hover_text("Off: inside uses a dimmed label colour; beside uses egui's weak text colour.")
                            .changed()
                        {
                            self.menu.settings.number_hints.color = custom.then_some(self.hint_color);
                        }
                        ui.add_enabled_ui(custom, |ui| {
                            if ui.color_edit_button_srgba(&mut self.hint_color).changed() {
                                self.menu.settings.number_hints.color = Some(self.hint_color);
                            }
                        });
                    });
                });

                section(ui, &query, &mut shown, "Misc", "pin center centre unpin esc word wrap", |ui| {
                    ui.checkbox(&mut self.pinned, "Pin menu at center");
                    ui.label("While pinned, press Esc to unpin.");
                    ui.label(format!("Word wrap (NW button): {}", if self.word_wrap { "ON" } else { "OFF" }));
                });

                if shown == 0 {
                    ui.label(RichText::new("No matching settings.").italics().color(Color32::GRAY));
                }

                ui.separator();
                ui.label(RichText::new(&self.last_action).size(13.0));
            });
        });

        egui::CentralPanel::default().show(ui, |ui| {
            ui.label("Right-click to open  ·  Number keys 1–9 to pick  ·  Esc / 5 to dismiss");
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new(&self.last_action).size(18.0).color(Color32::LIGHT_GRAY));
            });

            if self.pinned {
                self.menu.position = ui.clip_rect().center();
                self.menu_open = true;
            }

            if !self.pinned && !self.menu_open {
                if let Some(pos) = ctx.input(|i| {
                    i.pointer.secondary_pressed().then(|| i.pointer.latest_pos()).flatten()
                }) {
                    self.menu.open(pos);
                    self.menu_open = true;
                }
            }

            if self.menu_open {
                let mouse_pos = if self.pinned { None } else { ctx.input(|i| i.pointer.latest_pos()) };
                let key_down  = if self.pinned { true  } else { ctx.input(|i| i.pointer.secondary_down()) };

                // Number hints inside the buttons are fetched before `show`, which borrows the
                // menu mutably; "beside" mode lets the menu paint them itself.
                self.menu.settings.number_hints.show = self.show_hints && !self.hints_inside;
                let inside_hints: Vec<Option<NumberHint>> = self.buttons.iter()
                    .map(|b| if self.show_hints && self.hints_inside { self.menu.number_hint(b) } else { None })
                    .collect();
                let hint_gap = self.menu.settings.number_hints.spacing;
                let hint_color = self.menu.settings.number_hints.color;

                match self.menu.show(ctx, &self.buttons, mouse_pos, key_down, Some("Edit"),
                    |ui, idx, hovered| {
                        let label = LABELS[idx];
                        let color = COLORS[idx];
                        let (bg, fg) = if hovered {
                            (color, Color32::WHITE)
                        } else {
                            (color.gamma_multiply(0.6), Color32::LIGHT_GRAY)
                        };
                        // Hint digit in a dimmed version of the label colour unless overridden.
                        let hint = inside_hints[idx].as_ref().map(|h| {
                            (RichText::new(&h.text).color(hint_color.unwrap_or(fg.gamma_multiply(0.55))), h.on_right)
                        });

                        // NW slot (idx 7): checkbox — display only, toggle happens on Selected(7)
                        if idx == 7 {
                            egui::Frame::new()
                                .fill(bg)
                                .corner_radius(6.0)
                                .inner_margin(Vec2::new(10.0, 5.0))
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing.x = hint_gap;
                                    ui.horizontal(|ui| {
                                        if let Some((h, false)) = &hint { ui.label(h.clone()); }
                                        let mut display = self.word_wrap;
                                        ui.checkbox(&mut display, RichText::new(label).color(fg));
                                        if let Some((h, true)) = &hint { ui.label(h.clone()); }
                                    });
                                });
                            return;
                        }

                        // South slot (idx 4): icon button — verify egui::Button works inside an Area
                        if idx == 4 {
                            ui.spacing_mut().button_padding = Vec2::new(10.0, 5.0);
                            let btn = hint_button(RichText::new(format!("🗑  {label}")).color(fg), hint, hint_gap)
                                .fill(bg).corner_radius(6.0);
                            ui.add(btn).on_hover_text(TOOLTIPS[idx]);
                            return;
                        }

                        // All other slots: mnemonic label with tooltip
                        ui.spacing_mut().button_padding = Vec2::new(10.0, 5.0);
                        let text: egui::WidgetText = if let Some(mnemonic) = self.buttons[idx].mnemonic {
                            mnemonic_text(label, mnemonic, TextFormat {
                                color: fg,
                                font_id: ui.style().text_styles[&egui::TextStyle::Body].clone(),
                                ..Default::default()
                            }).into()
                        } else {
                            RichText::new(label).color(fg).into()
                        };
                        let resp = ui.add(hint_button(text, hint, hint_gap).fill(bg).corner_radius(6.0));
                        if !TOOLTIPS[idx].is_empty() {
                            resp.on_hover_text(TOOLTIPS[idx]);
                        }
                    })
                {
                    PieMenuResponse::Selected(idx) => {
                        if idx == 7 { self.word_wrap = !self.word_wrap; }
                        self.last_action = format!("Selected: {}", LABELS[idx]);
                        if !self.pinned { self.menu_open = false; }
                    }
                    PieMenuResponse::Dismissed => {
                        // Esc unpins: the full-screen input blocker would otherwise
                        // make the "Pin menu at center" checkbox unreachable.
                        self.pinned = false;
                        self.last_action = "Dismissed.".to_string();
                        self.menu_open = false;
                    }
                    PieMenuResponse::QuickTap => {
                        self.last_action = "Quick tap — default action.".to_string();
                        if !self.pinned { self.menu_open = false; }
                    }
                    PieMenuResponse::DoubleTap => {
                        self.last_action = "Double tap — alternate action.".to_string();
                        if !self.pinned { self.menu_open = false; }
                    }
                    PieMenuResponse::None => {}
                }
            }
        });
    }
}
