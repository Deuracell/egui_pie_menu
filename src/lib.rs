//use egui::introspection::font_id_ui;
use egui::{Color32, Key, Pos2, Rect, Ui, Vec2};
use utils::common_utils::SmartFloat; //FontDefinitions,
use std::collections::HashMap;
use std::f32::consts::{SQRT_2, TAU};
use std::time::Instant; //Duration,
mod utils {
    pub mod common_utils;
}
pub mod settings;
pub use settings::*;

pub struct PieMenuButton {
    pub label: String,
    pub direction: Vec2,
    pub is_true: bool,
    pub color: Color32,
    pub numpad_key: Key,
    pub mnemonic: Option<char>,
}

pub enum PieMenuResponse {
    Selected(String),
    Dismissed,
    None,
}

pub struct PieMenu {
    buttons: HashMap<String, PieMenuButton>,
    open_time: Instant,
    selected_index: Option<String>,
    position: Pos2,
    settings: PieMenuSettings,
    release_handled: bool,
}

impl PieMenu {
    fn get_default_settings() -> PieMenuSettings {
        PieMenuSettings::default()
    }

    /* fn direction_map() -> HashMap<&'static str, (Vec2, Key)> {
        let squareification_factor: f32 = 0.9;
        let radius = 100.0;
        let square_radius = SQRT_2 * squareification_factor;
        HashMap::from([
            ("N",   (Vec2::new((TAU * 0.75).cos(),  (TAU * 0.75).sin()),    Key::Num8)),
            ("NE",  (Vec2::new((TAU * 0.875).cos(), (TAU * 0.875).sin()),   Key::Num9)),
            ("E",   (Vec2::new((TAU).cos(),         (TAU).sin()),           Key::Num6)),
            ("SE",  (Vec2::new((TAU * 0.125).cos(), (TAU * 0.125).sin()),   Key::Num3)),
            ("S",   (Vec2::new((TAU * 0.25).cos(),  (TAU * 0.25).sin()),     Key::Num2)),
            ("SW",  (Vec2::new((TAU * 0.375).cos(), (TAU * 0.375).sin()),   Key::Num1)),
            ("W",   (Vec2::new((TAU * 0.5).cos(),   (TAU * 0.5).sin()),    Key::Num4)),
            ("NW",  (Vec2::new((TAU * 0.625).cos(), (TAU * 0.625).sin()),   Key::Num7)),
        ])
    } */

    fn squarify(vec: Vec2, factor: f32) -> Vec2 {
        let x = vec.x * (1.0 - factor) + vec.x.signum() * factor;
        let y = vec.y * (1.0 - factor) + vec.y.signum() * factor;
        Vec2::new(x, y)
    }


    // add logic for buttons to offset them selfs so that only the closest point to 0,0 is tangent with radius
    fn direction_map(squarification_factor: f32) -> HashMap<&'static str, (Vec2, Key)> {
        let directions = [
            ("N", 0.75, Key::Num8),
            ("NE", 0.875, Key::Num9),
            ("E", 1.0, Key::Num6),
            ("SE", 0.125, Key::Num3),
            ("S", 0.25, Key::Num2),
            ("SW", 0.375, Key::Num1),
            ("W", 0.5, Key::Num4),
            ("NW", 0.625, Key::Num7),
        ];
        
        directions.iter().map(|&(name, fraction, key)| {
            let angle = TAU * fraction;
            let vec = Vec2::new(angle.cos(), angle.sin());
            let adjusted_vec = match name {
                "NE" | "SE" | "SW" | "NW" => Self::squarify(vec, squarification_factor),
                _ => vec,
            };
            (name, (adjusted_vec, key))
        }).collect()
        
    }    
    pub fn new_with_buttons(layout_squarification: f32, labels: Vec<(&str, &str, Color32)>) -> Self {
        let direction_map = Self::direction_map(layout_squarification);
        let buttons = labels
            .into_iter()
            .filter_map(|(label, dir, color)| {
                direction_map.get(dir).map(|&(direction, numpad_key)| {
                    (dir.to_string(), PieMenuButton {
                        label: label.to_string(),
                        direction,
                        is_true: false, // placeholder, inherits from parent
                        color,
                        numpad_key,
                        mnemonic: label.chars().next(),
                    })
                })
            })
            .collect();

        Self {
            buttons,
            open_time: Instant::now(),
            selected_index: None,
            position: Pos2::ZERO,
            settings: PieMenuSettings::default(),
            release_handled: false,
        }
    }

    pub fn with_position(mut self, pos: Pos2) -> Self {
        self.position = pos;
        self
    }

    fn draw_button_label(&self, ui: &mut Ui, pos: Pos2, label: &str, mnemonic: Option<char>) {
        let char_width = 8.0;
        let total_width = label.len() as f32 * char_width;
        let start_x = -total_width / 2.0;
    
        for (i, c) in label.chars().enumerate() {
            let char_pos = pos + Vec2::new(start_x + (i as f32 * char_width), 0.0);
            
            ui.painter().text(
                char_pos,
                egui::Align2::LEFT_CENTER,
                c.to_string(),
                egui::FontId::default(),
                Color32::BLACK,
            );
            
            if mnemonic.map_or(false, |m| m == c) {
                let underline_start = char_pos + Vec2::new(-char_width/2.0, 6.0);
                let underline_end = underline_start + Vec2::new(char_width, 0.0);
                ui.painter().line_segment(
                    [underline_start, underline_end],
                    egui::Stroke::new(1.0, Color32::BLACK),
                );
            }
        }
    }

    pub fn show(&mut self, ui: &mut Ui, current_mouse_pos: Option<Pos2>, key_down: bool) -> PieMenuResponse {
        // Early exit checks
        if ui.input(|i| {
            (self.settings.input.dismiss_on_numpad_5 && i.key_pressed(Key::Num5)) ||
            i.key_pressed(Key::Escape) || 
            i.pointer.secondary_clicked()
        }) {
            return PieMenuResponse::Dismissed;
        }
    
        let center = self.position;
    
        // Draw base menu circle
        if SmartFloat::is_enabled(&self.settings.center_indicator.background_radius) {
            ui.painter().circle(
                center,
                self.settings.center_indicator.background_radius.get(),
                self.settings.center_indicator.background_fill_color,
                self.settings.center_indicator.background_stroke,            
            );
        }
    
        // Check for numpad selection
        for (dir, button) in &self.buttons {
            if ui.input(|i| i.key_pressed(button.numpad_key)) {
                return PieMenuResponse::Selected(dir.clone());
            }
        }
    
        // Handle mouse click
        if ui.input(|i| i.pointer.primary_clicked()) {
            if let Some(mouse_pos) = current_mouse_pos {
                let mouse_vector = mouse_pos - center;
                let distance = mouse_vector.length();
                
                if distance > self.settings.mouse_trigger_threshold {
                    if let Some(dir) = &self.selected_index {
                        return PieMenuResponse::Selected(dir.clone());
                    }
                }

            }   {
                return PieMenuResponse::Dismissed;
            }
        }
    
        // Update selection based on mouse angle
        if let Some(mouse_pos) = current_mouse_pos {
            let mouse_vector = mouse_pos - center;
            let distance = mouse_vector.length();
            
            if distance > self.settings.mouse_trigger_threshold {
                let angle = mouse_vector.y.atan2(mouse_vector.x);
                self.selected_index = self.buttons.iter()
                    .map(|(dir, button)| {
                        let button_angle = button.direction.y.atan2(button.direction.x);
                        let angle_diff = ((angle - button_angle + std::f32::consts::PI) % (2.0 * std::f32::consts::PI) - std::f32::consts::PI).abs();
                        (dir.clone(), angle_diff)
                    })
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                    .map(|(dir, _)| dir);
            } else {
                self.selected_index = None;
            }
        }    
        
        // Draw selection indicator arc with animated growth and fade-in.
        if let Some(_) = self.selected_index {
            let mouse_inside_threshold = current_mouse_pos
                .map(|p| (p - center).length() <= self.settings.mouse_trigger_threshold)
                .unwrap_or(false);
                
            if !mouse_inside_threshold {
                // Compute progress from 0.0 to 1.0 based on elapsed time.
                let elapsed = self.open_time.elapsed().as_secs_f32();
                let duration = self.settings.animations.center_highlight_duration.as_secs_f32();
                let progress = (elapsed / duration).min(1.0);
                
                // Animate from 0 to full arc (360/8 radians).
                let full_arc_angle = std::f32::consts::PI * 2.0 / 8.0;
                let arc_angle = full_arc_angle * progress;
                
                // Animate alpha from 0 to full (using progress).
                let alpha = progress;
                
                // Determine the base angle (snapped or following mouse).
                let base_angle = if self.settings.animations.center_highlight_snapping {
                    if let Some(selected) = &self.selected_index {
                        self.buttons.get(selected)
                            .map(|button| button.direction.y.atan2(button.direction.x))
                            .unwrap_or(0.0)
                    } else {
                        0.0
                    }
                } else if let Some(mouse_pos) = current_mouse_pos {
                    (mouse_pos - center).y.atan2((mouse_pos - center).x)
                } else {
                    0.0
                };
                
                let start_angle = base_angle - arc_angle / 2.0;
                // Multiply the highlight color by the computed alpha.
                let highlight_color = self.settings.center_indicator.highlight_fill_color.linear_multiply(alpha);
                
                ui.painter().arc(
                    center,
                    self.settings.center_indicator.highlight_radius.get(),
                    start_angle..(start_angle + arc_angle),
                    egui::Stroke::new(3.0, highlight_color),
                );
            }
        }
    
        // Draw buttons
        for (dir, button) in &self.buttons {
            let offset = button.direction * 80.0;
            let button_pos = center + offset;
            let button_rect = Rect::from_center_size(button_pos, Vec2::new(60.0, 30.0));
    
            let color = if self.selected_index.as_ref() == Some(dir) {
                button.color.linear_multiply(0.5)
            } else {
                button.color
            };
    
            ui.painter().rect_filled(button_rect, 5.0, color);
            self.draw_button_label(ui, button_pos, &button.label, button.mnemonic);
        }
    
        // Handle key release
        if !key_down && !self.release_handled {
            if let Some(mouse_pos) = current_mouse_pos {
                let distance_to_center = (mouse_pos - center).length();
                let held_duration = self.open_time.elapsed();
                
                if distance_to_center <= self.settings.mouse_trigger_threshold {
                    if held_duration > self.settings.input.key_timeout {
                        return PieMenuResponse::Dismissed;
                    }
                    // Keep menu open if released quickly within threshold
                } else if let Some(dir) = &self.selected_index {
                    return PieMenuResponse::Selected(dir.clone());
                }
            }
            self.release_handled = true;
        }
    
        PieMenuResponse::None
    }
}

// Extension trait to add an arc drawing method to egui::Painter
pub trait ArcPainter {
    fn arc(&self, center: Pos2, radius: f32, angle_range: std::ops::Range<f32>, stroke: egui::Stroke);
}
print!ln
impl ArcPainter for egui::Painter {
    fn arc(&self, 
        center: Pos2, 
        radius: f32,
        resolution: f32,
        width_angle_radians: f32,
        stroke: egui::Stroke) {
        
        let start_angle = width_angle_radians / -2;
        let arc_length = width_angle_radians * radius;
        let n_points = (arc_length / resolution).ceil() as usize;
        let step = width_angle_radians / n_points as f32;
        let points: Vec<Pos2> = (0..=n_points)
            .map(|i| {
                let angle = start_angle + i as f32 * step;
                center + egui::vec2(angle.cos(), angle.sin()) * radius
            })
            .collect();
        self.add(egui::epaint::Shape::line(points, stroke));
    }
}

