use eframe::egui;
use egui::Color32;
use egui_pie_menu::{PieButton, PieDirection, PieMenu, PieMenuHighlightShape, PieMenuResponse, SmartFloat};

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
    hotkey_down: bool,
    last_action: String,
    pinned: bool,

    // Locally mirrored f32 values for SmartFloat settings (sliders need plain f32)
    bg_radius: f32,
    highlight_radius: f32,
    highlight_angle_deg: f32,

    buttons: Vec<PieButton>,
}

impl Demo {
    fn new() -> Self {
        let buttons = vec![
            PieButton::new(PieDirection::North,     "Copy")  .with_color(Color32::from_rgb(70, 130, 180)).with_mnemonic('c'),
            PieButton::new(PieDirection::NorthEast, "Paste") .with_color(Color32::from_rgb(70, 130, 180)).with_mnemonic('v'),
            PieButton::new(PieDirection::East,      "Redo")  .with_color(Color32::from_rgb(100, 160, 100)).with_mnemonic('r'),
            PieButton::new(PieDirection::SouthEast, "Save")  .with_color(Color32::from_rgb(180, 140, 60)).with_mnemonic('s'),
            PieButton::new(PieDirection::South,     "Delete").with_color(Color32::from_rgb(180, 70, 70)).with_mnemonic('d'),
            PieButton::new(PieDirection::SouthWest, "Cut")   .with_color(Color32::from_rgb(180, 100, 60)).with_mnemonic('x'),
            PieButton::new(PieDirection::West,      "Undo")  .with_color(Color32::from_rgb(100, 160, 100)).with_mnemonic('u'),
            PieButton::new(PieDirection::NorthWest, "Open")  .with_color(Color32::from_rgb(130, 90, 180)).with_mnemonic('o'),
        ];

        let menu = PieMenu::new();
        let bg_radius = menu.settings.center_indicator.background_radius.get();
        let highlight_radius = menu.settings.center_indicator.highlight_radius.get();
        let highlight_angle_deg = menu.settings.center_indicator.highlight_angle.to_degrees();

        Self {
            menu,
            menu_open: false,
            hotkey_down: false,
            last_action: "Right-click anywhere to open the menu.".to_string(),
            pinned: false,
            bg_radius,
            highlight_radius,
            highlight_angle_deg,
            buttons,
        }
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
        PieMenuHighlightShape::None          => "None",
        PieMenuHighlightShape::Arc           => "Arc",
        PieMenuHighlightShape::Slice         => "Slice",
        PieMenuHighlightShape::Circle        => "Circle",
        PieMenuHighlightShape::ArcSlice      => "Arc + Slice",
        PieMenuHighlightShape::ArcCircle     => "Arc + Circle",
        PieMenuHighlightShape::SliceCircle   => "Slice + Circle",
        PieMenuHighlightShape::ArcSliceCircle=> "Arc + Slice + Circle",
    }
}

impl eframe::App for Demo {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("settings").min_width(220.0).show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // ── Layout ───────────────────────────────────────────
                ui.heading("Layout");
                ui.separator();

                ui.label("Squarification  (−1 diamond · 0 circle · +1 square)");
                ui.add(egui::Slider::new(
                    &mut self.menu.settings.layout_squarification,
                    -1.0..=1.0,
                ).step_by(0.01));

                ui.label("Radius");
                ui.add(egui::Slider::new(
                    &mut self.menu.settings.layout_radius,
                    40.0..=200.0,
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
                let mut fill = self.menu.settings.center_indicator.background_fill_color;
                if ui.color_edit_button_srgba(&mut fill).changed() {
                    self.menu.settings.center_indicator.background_fill_color = fill;
                }

                ui.label("Stroke width");
                ui.add(egui::Slider::new(
                    &mut self.menu.settings.center_indicator.background_stroke.width,
                    0.0..=10.0,
                ).step_by(0.5));

                ui.label("Stroke color");
                let mut stroke_color = self.menu.settings.center_indicator.background_stroke.color;
                if ui.color_edit_button_srgba(&mut stroke_color).changed() {
                    self.menu.settings.center_indicator.background_stroke.color = stroke_color;
                }

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
                                s,
                                shape_label(s),
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
                    &mut self.menu.settings.center_indicator.highlight_circle_radius,
                    1.0..=40.0,
                ).step_by(0.5));

                ui.label("Fill color");
                let mut fill = self.menu.settings.center_indicator.highlight_fill_color;
                if ui.color_edit_button_srgba(&mut fill).changed() {
                    self.menu.settings.center_indicator.highlight_fill_color = fill;
                }

                ui.label("Stroke width");
                ui.add(egui::Slider::new(
                    &mut self.menu.settings.center_indicator.highlight_stroke.width,
                    0.0..=10.0,
                ).step_by(0.5));

                ui.label("Stroke color");
                let mut stroke_color = self.menu.settings.center_indicator.highlight_stroke.color;
                if ui.color_edit_button_srgba(&mut stroke_color).changed() {
                    self.menu.settings.center_indicator.highlight_stroke.color = stroke_color;
                }

                ui.label("Snapping");
                ui.checkbox(&mut self.menu.settings.animations.center_highlight_snapping, "Snap to button");

                ui.add_space(8.0);

                // ── Center label ─────────────────────────────────────
                ui.heading("Center label");
                ui.separator();

                ui.checkbox(&mut self.menu.settings.label.display, "Show hovered label");

                ui.label("Text color");
                let mut text_color = self.menu.settings.label.text_color;
                if ui.color_edit_button_srgba(&mut text_color).changed() {
                    self.menu.settings.label.text_color = text_color;
                }

                ui.label("Background color");
                let mut bg_color = self.menu.settings.label.background_color;
                if ui.color_edit_button_srgba(&mut bg_color).changed() {
                    self.menu.settings.label.background_color = bg_color;
                }

                ui.label("Stroke width");
                ui.add(egui::Slider::new(
                    &mut self.menu.settings.label.background_stroke.width,
                    0.0..=4.0,
                ).step_by(0.5));

                ui.label("Stroke color");
                let mut stroke_color = self.menu.settings.label.background_stroke.color;
                if ui.color_edit_button_srgba(&mut stroke_color).changed() {
                    self.menu.settings.label.background_stroke.color = stroke_color;
                }

                ui.add_space(8.0);

                // ── Misc ─────────────────────────────────────────────
                ui.heading("Misc");
                ui.separator();
                ui.checkbox(&mut self.pinned, "Pin menu at center");
                ui.add_space(8.0);
                ui.separator();
                ui.label(egui::RichText::new(&self.last_action).size(13.0));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Right-click to open  ·  Numpad 1–9 to pick  ·  Esc / Numpad 5 to dismiss");

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

                match self.menu.show(ui, &self.buttons, mouse_pos, self.hotkey_down, Some("Edit")) {
                    PieMenuResponse::Selected(idx) => {
                        self.last_action = format!("Selected: {}", self.buttons[idx].label);
                        if !self.pinned { self.menu_open = false; }
                    }
                    PieMenuResponse::Dismissed => {
                        if !self.pinned {
                            self.last_action = "Dismissed.".to_string();
                            self.menu_open = false;
                        }
                    }
                    PieMenuResponse::None => {}
                }
            }
        });
    }
}
