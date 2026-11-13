//! A pie menu built in code: right-click (or hold the right button and flick) to open it.
//!
//! `cargo run --example basic`. For everything else the menu can do, and to design menus
//! visually, run the designer: `cargo run -p egui_pie_menu_designer`.
use eframe::egui;
use egui_pie_menu::{ClusterItem, PieButton, PieDirection, PieMenu, PieMenuResponse};

fn main() -> eframe::Result {
    eframe::run_native("egui_pie_menu: basic", eframe::NativeOptions::default(), Box::new(|_| Ok(Box::new(App::new()))))
}

struct App {
    menu: PieMenu,
    buttons: Vec<PieButton>,
    last: String,
}

impl App {
    fn new() -> Self {
        let buttons = vec![
            PieButton::new(PieDirection::North).with_label("Copy").with_mnemonic('c'),
            PieButton::new(PieDirection::South).with_label("Paste").with_mnemonic('p'),
            PieButton::new(PieDirection::West).with_label("Undo"),
            // A sub-pie: opens a ring of its own; Back (or Backspace) returns.
            PieButton::new(PieDirection::East).with_label("More").with_children(vec![
                PieButton::back(PieDirection::West),
                PieButton::new(PieDirection::North).with_label("Select All"),
                PieButton::new(PieDirection::East).with_label("Find"),
                PieButton::new(PieDirection::South).with_label("Replace"),
            ]),
            // A cluster: one direction, several small buttons (drawn by the closure below).
            PieButton::cluster(PieDirection::NorthEast).with_label("Format").with_items(vec![
                ClusterItem::new(1, "B"),
                ClusterItem::new(2, "I"),
                ClusterItem::new(3, "U"),
            ]),
        ];
        let menu = PieMenu::new().with_title("Edit").configure(|s| s.input.auto_mnemonics = true);
        Self { menu, buttons, last: "Right-click anywhere.".to_string() }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        egui::CentralPanel::default().show(ui, |ui| ui.heading(&self.last));

        if ctx.input(|i| i.pointer.secondary_pressed())
            && let Some(pos) = ctx.input(|i| i.pointer.latest_pos())
        {
            self.menu.open(pos);
        }
        let key_down = ctx.input(|i| i.pointer.secondary_down());
        let response = self.menu.show(&ctx, &self.buttons, key_down, |ui, b| {
            if b.button.is_cluster() {
                // Each item is a widget of its own, registered with `b.item`.
                ui.horizontal(|ui| {
                    for number in 1..=3 {
                        b.item(number, ui.button(b.item_label(number)));
                    }
                });
            } else {
                ui.add(egui::Button::new(b.label_text()));
            }
        });
        if let PieMenuResponse::Selected(_) = response {
            // The route from the root: [1] is "Paste", [3, 2] is "More" › "Find", [4, 2] is the
            // second item of the cluster.
            self.last = format!("Selected {:?}", self.menu.selected_path());
        }
    }
}
