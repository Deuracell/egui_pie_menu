//! Menus defined in TOML files (`examples/menus`), loaded at runtime: a base menu, a plugin
//! extending it and a user's changes, merged. The program supplies values by name
//! (`Variables`), says which actions it handles and their state (`Actions`), and matches on
//! the id of what was picked.
//!
//! `cargo run --example toml_menus`. Right-click to open the menu.
use eframe::egui;
use egui_pie_menu::catalog::{MenuCatalog, MenuDefinition};
use egui_pie_menu::values::Variables;
use egui_pie_menu::{Actions, PieMenu, PieMenuResponse, unhandled_color};

fn main() -> eframe::Result {
    eframe::run_native("egui_pie_menu: TOML menus", eframe::NativeOptions::default(), Box::new(|_| Ok(Box::new(App::new()))))
}

struct App {
    edit: MenuDefinition,
    menu: PieMenu,
    /// Things done, for Undo; things undone, for Redo.
    done: usize,
    undone: usize,
    last: String,
}

impl App {
    fn new() -> Self {
        let mut catalog = MenuCatalog::new();
        // A program would use `catalog.add_file(path)` for each file.
        for (name, text) in [
            ("edit.toml", include_str!("menus/edit.toml")),
            ("plugin.toml", include_str!("menus/plugin.toml")),
            ("user.toml", include_str!("menus/user.toml")),
        ] {
            catalog.add_toml(name, text).expect("the example files load");
        }
        let edit = catalog.build("edit").expect("the edit menu builds");
        let app = Self { menu: edit.to_pie_menu(), edit, done: 0, undone: 0, last: "Right-click anywhere.".to_string() };
        // What a program would log after loading: names and ids it does not answer.
        for problem in app.edit.check_references(&app.variables(&egui::Visuals::dark()), &app.actions()) {
            println!("menu: {problem}");
        }
        app
    }

    /// The values the files refer to by name (`{ var = "theme.accent" }`, `i18n.*`).
    fn variables(&self, visuals: &egui::Visuals) -> Variables {
        let mut vars = Variables::new();
        vars.color("theme.accent", visuals.selection.bg_fill);
        for (key, text) in
            [("edit", "Edit"), ("copy", "Copy"), ("redo", "Redo"), ("cut", "Cut"), ("undo", format!("Undo ({})", self.done).as_str())]
        {
            vars.text(format!("i18n.{key}"), text);
        }
        vars
    }

    /// The actions this program handles, and their state right now. (It does not handle
    /// the plugin's `plugin.share.print`: that button is disabled and drawn in red.)
    fn actions(&self) -> Actions {
        let mut a = Actions::new();
        for id in ["edit.copy", "edit.cut", "edit.select_all", "format.bold", "format.italic", "plugin.paste_image"] {
            a.add(id);
        }
        a.add("edit.undo").enabled(self.done > 0);
        a.add("edit.redo").enabled(self.undone > 0);
        for id in ["plugin.share", "plugin.share.email", "plugin.share.chat", "plugin.share.link"] {
            a.add(id);
        }
        a
    }

    fn run(&mut self, id: &str) {
        match id {
            "edit.undo" => (self.done, self.undone) = (self.done - 1, self.undone + 1),
            "edit.redo" => (self.done, self.undone) = (self.done + 1, self.undone - 1),
            _ => (self.done, self.undone) = (self.done + 1, 0),
        }
        self.last = format!("Ran `{id}`");
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        egui::CentralPanel::default().show(ui, |ui| ui.heading(&self.last));

        // Each frame: the program's values and the state of its actions.
        let vars = self.variables(ui.visuals());
        self.edit.apply_variables(vars, &mut self.menu);
        let actions = self.actions();
        self.edit.apply_actions(&actions);

        if ctx.input(|i| i.pointer.secondary_pressed())
            && let Some(pos) = ctx.input(|i| i.pointer.latest_pos())
        {
            self.menu.open(pos);
        }
        let key_down = ctx.input(|i| i.pointer.secondary_down());
        let edit = &self.edit;
        let response = self.menu.show(&ctx, &edit.buttons, key_down, |ui, b| {
            if b.button.is_cluster() {
                ui.horizontal(|ui| {
                    for item in b.button.cluster_items() {
                        b.item(item.number, ui.button(b.item_label(item.number)));
                    }
                });
                return;
            }
            let mut text = b.label_text();
            if edit.is_unhandled(b.path) {
                text = text.color(unhandled_color(ui));
            }
            ui.add(egui::Button::new(text)).on_disabled_hover_text("not available");
        });
        if let PieMenuResponse::Selected(_) = response
            && let Some(id) = self.edit.ids.selected(&self.edit.buttons, self.menu.selected_path())
        {
            let id = id.to_string();
            self.run(&id);
        }
    }
}
