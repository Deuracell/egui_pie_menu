// #![warn(clippy::pedantic)]
use eframe::egui;
use egui::{Color32, RichText, Vec2};
use egui_pie_menu::{mnemonic_text, PieButton, PieDirection, PieMenu, PieMenuHighlightShape, PieMenuResponse, ShowBehavior, SmartFloat, TextFormat};

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
            show_threshold: 10.0,
        }
    }
}

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
        egui::Panel::left("settings").min_size(220.0).show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
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
                ui.checkbox(&mut self.pinned, "Pin menu at center");
                ui.label(format!("Word wrap (NW button): {}", if self.word_wrap { "ON" } else { "OFF" }));
                ui.add_space(8.0);
                ui.separator();
                ui.label(RichText::new(&self.last_action).size(13.0));
            });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.label("Right-click to open  ·  Numpad 1–9 to pick  ·  Esc / Numpad 5 to dismiss");
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new(&self.last_action).size(18.0).color(Color32::LIGHT_GRAY));
            });

            if self.pinned {
                self.menu.position = ui.clip_rect().center();
                self.menu_open = true;
            }

            if !self.pinned && !self.menu_open
                && let Some(pos) = ui.ctx().input(|i| {
                    i.pointer.secondary_pressed().then(|| i.pointer.latest_pos()).flatten()
                }) {
                    self.menu.open(pos);
                    self.menu_open = true;
                }

            if self.menu_open {
                let mouse_pos = if self.pinned { None } else { ui.ctx().input(|i| i.pointer.latest_pos()) };
                let key_down  = if self.pinned { true  } else { ui.ctx().input(|i| i.pointer.secondary_down()) };

                match self.menu.show(ui, &self.buttons, mouse_pos, key_down, Some("Edit"),
                    |ui, idx, hovered| {
                        let label = LABELS[idx];
                        let color = COLORS[idx];
                        let (bg, fg) = if hovered {
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
                        self.last_action = format!("Selected: {}", LABELS[idx]);
                        if !self.pinned { self.menu_open = false; }
                    }
                    PieMenuResponse::Dismissed => {
                        if !self.pinned {
                            self.last_action = "Dismissed.".to_string();
                            self.menu_open = false;
                        }
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
