use eframe::egui;
use egui::{Color32, RichText, Vec2};
use egui_pie_menu::{assign_mnemonics, mnemonic_text, HintSide, PieButton, PieDirection, PieMenu, PieMenuHighlightShape, PieMenuResponse, ShowBehavior, SmartFloat, TextFormat};

fn main() -> eframe::Result {
    eframe::run_native(
        "egui_pie_menu demo",
        eframe::NativeOptions::default(),
        Box::new(|cc| {
            let font_note = load_system_fonts(&cc.egui_ctx);
            let mut demo = Demo::new();
            demo.font_note = font_note;
            Ok(Box::new(demo))
        }),
    )
}

/// egui's built-in fonts lack Arabic/Hebrew, so try to add a system font as a fallback.
fn load_system_fonts(ctx: &egui::Context) -> String {
    const CANDIDATES: &[&str] = &[
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/freefont/FreeSans.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansArabic-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansHebrew-Regular.ttf",
        "/Library/Fonts/Arial Unicode.ttf",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
    ];
    let mut fonts = egui::FontDefinitions::default();
    let mut loaded = Vec::new();
    for path in CANDIDATES {
        if let Ok(bytes) = std::fs::read(path) {
            let name = format!("fallback{}", loaded.len());
            fonts.font_data.insert(name.clone(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts.families.entry(family).or_default().push(name.clone());
            }
            loaded.push(*path);
        }
    }
    ctx.set_fonts(fonts);
    if loaded.is_empty() {
        "No system font with Arabic/Hebrew found: those labels may show as boxes.".to_string()
    } else {
        format!("Fallback fonts: {}", loaded.join(", "))
    }
}

struct Lang {
    name: &'static str,
    title: &'static str,
    rtl: bool,
    arabic_digits: bool,
    labels: [&'static str; 8],
}

const LANGS: &[Lang] = &[
    Lang { name: "English", title: "Edit", rtl: false, arabic_digits: false,
        labels: ["Copy", "Paste Special", "Redo", "Save As…", "Delete Permanently", "Cut", "Undo Last Change", "Word wrap"] },
    Lang { name: "Русский", title: "Правка", rtl: false, arabic_digits: false,
        labels: ["Копировать", "Специальная вставка", "Повторить", "Сохранить как…", "Удалить навсегда", "Вырезать", "Отменить изменение", "Перенос слов"] },
    Lang { name: "עברית", title: "עריכה", rtl: true, arabic_digits: false,
        labels: ["העתק", "הדבק מיוחד", "בצע שוב", "שמור בשם…", "מחק לצמיתות", "גזור", "בטל שינוי", "גלישת שורות"] },
    Lang { name: "العربية", title: "تحرير", rtl: true, arabic_digits: true,
        labels: ["نسخ", "لصق خاص", "إعادة", "حفظ باسم…", "حذف نهائي", "قص", "تراجع", "التفاف النص"] },
];

struct Demo {
    lang: usize,
    auto_mnemonics: bool,
    font_note: String,
    menu: PieMenu,
    menu_open: bool,
    last_action: String,
    pinned: bool,
    /// True while the pinned menu is the open one.
    pin_active: bool,

    // Mirrored f32 values for SmartFloat settings (sliders need plain f32)
    bg_radius: f32,
    highlight_radius: f32,
    highlight_angle_deg: f32,

    buttons: Vec<PieButton>,

    // State for the checkbox button slot
    word_wrap: bool,

    // Mirror for show threshold slider (only active in OnMovement mode)
    show_threshold: f32,
}

impl Demo {
    /// English without auto keeps hand-picked mnemonics. With auto (or any other
    /// language) mnemonics are generated; English still pins 'c' on Copy to show
    /// that explicit mnemonics are respected.
    fn make_buttons(lang: usize, auto: bool) -> Vec<PieButton> {
        use PieDirection::*;
        let dirs = [North, NorthEast, East, SouthEast, South, SouthWest, West, NorthWest];
        let manual = ['c', 'p', 'r', 's', 'd', 't', 'u'];
        if lang == 0 && !auto {
            let mut it = manual.into_iter();
            return dirs
                .into_iter()
                .map(|d| match it.next() {
                    Some(c) => PieButton::new(d).with_mnemonic(c),
                    None => PieButton::new(d),
                })
                .collect();
        }
        let mut buttons: Vec<PieButton> = dirs.into_iter().map(PieButton::new).collect();
        if lang == 0 {
            buttons[0] = PieButton::new(North).with_mnemonic('c');
        }
        assign_mnemonics(&mut buttons, &LANGS[lang].labels);
        buttons
    }

    fn apply_language(&mut self) {
        let l = &LANGS[self.lang];
        self.buttons = Self::make_buttons(self.lang, self.auto_mnemonics);
        self.menu.settings.mirror_horizontal = l.rtl;
        self.menu.settings.number_hints.side = if l.rtl { HintSide::Right } else { HintSide::Left };
        self.menu.settings.number_hints.arabic_indic_digits = l.arabic_digits;
    }

    fn new() -> Self {
        let buttons = Self::make_buttons(0, false);

        let menu = PieMenu::new();
        let bg_radius = menu.settings.center_indicator.background_radius.get();
        let highlight_radius = menu.settings.center_indicator.highlight_radius.get();
        let highlight_angle_deg = menu.settings.center_indicator.highlight_angle.to_degrees();

        Self {
            lang: 0,
            auto_mnemonics: false,
            font_note: String::new(),
            menu,
            menu_open: false,
            last_action: "Right-click anywhere to open the menu.".to_string(),
            pinned: false,
            pin_active: false,
            bg_radius,
            highlight_radius,
            highlight_angle_deg,
            buttons,
            word_wrap: false,
            show_threshold: 10.0,
        }
    }
}

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
        let panel_rect = egui::Panel::left("settings").min_size(220.0).show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.checkbox(&mut self.pinned, "Pin menu at center");
                ui.label(RichText::new("While pinned, press Esc to unpin.").small());
                ui.add_space(8.0);

                // ── Layout ───────────────────────────────────────────
                ui.heading("Layout");
                ui.separator();

                ui.label("Shape factor  (−1 diamond · 0 circle · +1 square)");
                ui.add(egui::Slider::new(
                    &mut self.menu.settings.shape_factor, -1.0..=1.0,
                ).step_by(0.01));

                ui.label("Radius");
                ui.add(egui::Slider::new(
                    &mut self.menu.settings.layout_radius, 40.0..=200.0,
                ).step_by(1.0));

                ui.add_space(8.0);

                // ── Center background ────────────────────────────────
                ui.heading("Center background");
                ui.separator();

                ui.label("Radius");
                if ui.add(egui::Slider::new(&mut self.bg_radius, 0.0..=60.0).step_by(0.5)).changed() {
                    self.menu.settings.center_indicator.background_radius = SmartFloat::new(self.bg_radius);
                }

                ui.label("Fill color");
                ui.color_edit_button_srgba(&mut self.menu.settings.center_indicator.background_fill_color);

                ui.label("Stroke width");
                ui.add(egui::Slider::new(
                    &mut self.menu.settings.center_indicator.background_stroke.width, 0.0..=10.0,
                ).step_by(0.5));

                ui.label("Stroke color");
                ui.color_edit_button_srgba(&mut self.menu.settings.center_indicator.background_stroke.color);

                ui.add_space(8.0);

                // ── Highlight ────────────────────────────────────────
                ui.heading("Highlight");
                ui.separator();

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
                if ui.add(egui::Slider::new(&mut self.highlight_radius, 1.0..=60.0).step_by(0.5)).changed() {
                    self.menu.settings.center_indicator.highlight_radius = SmartFloat::new(self.highlight_radius);
                }

                ui.label("Arc angle (°)");
                if ui.add(egui::Slider::new(&mut self.highlight_angle_deg, 5.0..=360.0).step_by(1.0)).changed() {
                    self.menu.settings.center_indicator.highlight_angle = self.highlight_angle_deg.to_radians();
                }

                ui.label("Circle radius");
                ui.add(egui::Slider::new(
                    &mut self.menu.settings.center_indicator.highlight_circle_radius, 1.0..=40.0,
                ).step_by(0.5));

                ui.label("Fill color");
                ui.color_edit_button_srgba(&mut self.menu.settings.center_indicator.highlight_fill_color);

                ui.label("Stroke width");
                ui.add(egui::Slider::new(
                    &mut self.menu.settings.center_indicator.highlight_stroke.width, 0.0..=10.0,
                ).step_by(0.5));

                ui.label("Stroke color");
                ui.color_edit_button_srgba(&mut self.menu.settings.center_indicator.highlight_stroke.color);

                ui.checkbox(&mut self.menu.settings.animations.center_highlight_snapping, "Snap to button");

                ui.add_space(8.0);

                // ── Center label ─────────────────────────────────────
                ui.heading("Center label");
                ui.separator();

                ui.checkbox(&mut self.menu.settings.label.display, "Show title");
                ui.label("Text color");
                ui.color_edit_button_srgba(&mut self.menu.settings.label.text_color);
                ui.label("Background color");
                ui.color_edit_button_srgba(&mut self.menu.settings.label.background_color);
                ui.label("Stroke width");
                ui.add(egui::Slider::new(
                    &mut self.menu.settings.label.background_stroke.width, 0.0..=4.0,
                ).step_by(0.5));
                ui.label("Stroke color");
                ui.color_edit_button_srgba(&mut self.menu.settings.label.background_stroke.color);

                ui.add_space(8.0);

                // ── Show behaviour ───────────────────────────────────
                ui.heading("Show behaviour");
                ui.separator();

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
                    if ui.add(egui::Slider::new(threshold, 0.0..=50.0).step_by(0.5)).changed() {
                        self.show_threshold = *threshold;
                    }
                }

                if is_instant {
                    ui.label("QuickTap / DoubleTap not available.");
                }

                ui.add_space(8.0);

                // ── Misc ─────────────────────────────────────────────
                ui.heading("Misc");
                ui.separator();
                ui.heading("Language / RTL");
                ui.separator();
                let mut lang_changed = false;
                egui::ComboBox::from_id_salt("lang")
                    .selected_text(LANGS[self.lang].name)
                    .show_ui(ui, |ui| {
                        for (i, l) in LANGS.iter().enumerate() {
                            lang_changed |= ui.selectable_value(&mut self.lang, i, l.name).changed();
                        }
                    });
                lang_changed |= ui.checkbox(&mut self.auto_mnemonics, "Auto-generate mnemonics").changed();
                if lang_changed { self.apply_language(); }
                ui.checkbox(&mut self.menu.settings.mirror_horizontal, "Mirror layout (RTL)");
                ui.checkbox(&mut self.menu.settings.number_hints.show, "Show number hints");
                ui.checkbox(&mut self.menu.settings.input.ignore_empty_sectors, "Ignore empty sectors");
                let mut cut_disabled = !self.buttons[5].enabled;
                if ui.checkbox(&mut cut_disabled, "Disable the SW button").changed() {
                    self.buttons[5].enabled = !cut_disabled;
                }
                ui.label(RichText::new("egui has no bidi: keep labels single-script.").small());
                if !self.font_note.is_empty() { ui.label(RichText::new(&self.font_note).small()); }
                ui.add_space(8.0);

                ui.label(format!("Word wrap (NW button): {}", if self.word_wrap { "ON" } else { "OFF" }));
                ui.add_space(8.0);
                ui.separator();
                ui.label(RichText::new(&self.last_action).size(13.0));
            });
        }).response.rect;

        egui::CentralPanel::default().show(ui, |ui| {
            ui.label("Right-click to open  ·  Number keys 1–9 to pick  ·  Esc / 5 to dismiss");
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new(&self.last_action).size(18.0).color(Color32::LIGHT_GRAY));
            });

            let pin_center = ui.clip_rect().center();
            // Leaving pinned mode closes the pinned menu.
            if !self.pinned && self.pin_active {
                self.pin_active = false;
                self.menu_open = false;
            }
            self.menu.settings.block_background = !self.pinned;
            // Pinned: open once, centred and visible. Skip showing on the frame the
            // checkbox was clicked so that click is not also seen by the menu.
            let mut skip_show = false;
            if self.pinned {
                if !self.pin_active {
                    self.pin_active = true;
                    self.menu.open(pin_center);
                    self.menu.reveal();
                    self.menu_open = true;
                    skip_show = true;
                }
                self.menu.position = pin_center;
            }

            if !self.pinned && !self.menu_open {
                if let Some(pos) = ctx.input(|i| {
                    i.pointer.secondary_pressed().then(|| i.pointer.latest_pos()).flatten()
                }) {
                    self.menu.open(pos);
                    self.menu_open = true;
                }
            }

            if self.menu_open && !skip_show {
                // Pinned: use the real pointer, but treat the settings panel as "centre"
                // so clicking its widgets never selects a pie button.
                let mouse_pos = ctx.input(|i| i.pointer.latest_pos()).map(|p| {
                    if self.pinned && panel_rect.contains(p) { pin_center } else { p }
                });
                let key_down  = if self.pinned { true  } else { ctx.input(|i| i.pointer.secondary_down()) };

                let labels = LANGS[self.lang].labels;
                match self.menu.show(ctx, &self.buttons, mouse_pos, key_down, Some(LANGS[self.lang].title),
                    |ui, idx, hovered| {
                        let label = labels[idx];
                        let color = COLORS[idx];
                        let enabled = self.buttons[idx].enabled;
                        let (bg, fg) = if !enabled {
                            (color.gamma_multiply(0.25), Color32::DARK_GRAY)
                        } else if hovered {
                            (color, Color32::WHITE)
                        } else {
                            (color.gamma_multiply(0.6), Color32::LIGHT_GRAY)
                        };

                        // NW slot (idx 7): checkbox — display only, toggle happens on Selected(7)
                        if idx == 7 {
                            egui::Frame::new()
                                .fill(bg)
                                .corner_radius(6.0)
                                .inner_margin(Vec2::new(10.0, 5.0))
                                .show(ui, |ui| {
                                    let mut display = self.word_wrap;
                                    ui.checkbox(&mut display, RichText::new(label).color(fg));
                                });
                            return;
                        }

                        // South slot (idx 4): icon button — verify egui::Button works inside an Area
                        if idx == 4 {
                            ui.spacing_mut().button_padding = Vec2::new(10.0, 5.0);
                            let btn = egui::Button::new(
                                RichText::new(format!("🗑  {label}")).color(fg)
                            ).fill(bg).corner_radius(6.0);
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
                        let resp = ui.add(egui::Button::new(text).fill(bg).corner_radius(6.0));
                        if !TOOLTIPS[idx].is_empty() {
                            resp.on_hover_text(TOOLTIPS[idx]);
                        }
                    })
                {
                    PieMenuResponse::Selected(idx) => {
                        if idx == 7 { self.word_wrap = !self.word_wrap; }
                        self.last_action = format!("Selected: {}", labels[idx]);
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
