# Pie Designer

`egui_pie_menu_designer` is the companion of [egui_pie_menu](../README.md): an app to try everything the pie menu can do and to design menus visually, and the same editors as a library to embed in your own program's tools.

- Every setting, grouped and searchable, with a live menu to try them on.
- The button tree (sub-pies, fan-outs, clusters) with an inspector that only offers what is legal.
- A live preview of the selected button, sub-pie or fan-out.
- The result as TOML ([menus from TOML files](../README.md#menus-from-toml-files)) or as Rust code, with values bound to your program's variables.

## The app

```sh
cargo run -p egui_pie_menu_designer      # the `pie-designer` binary
```

Right-click anywhere in the window (or hold Space) to open the menu. The app has three panes:

- Left, in two tabs, with one search box that filters both (the tab that is not showing carries the number of hits, for example "Buttons (3)"):
  - **Menu**: one collapsing section per `SettingsGroup`, drawn with `group_editor`'s widgets (`field_editor_bound`: colours, strokes and fonts carry a ⛓ toggle to bind them to an expression of your app, see [Bindings](#bindings-values-from-your-program); the app registers `theme.*` colours from egui's Dark / Light visuals and `i18n.*` labels and title in the chosen language, read by generated code with `tr("..")`, as variables), plus a few rows of the app's own, tagged "demo" where they only change how the app draws its buttons (number hints, button height, sub-pie look, markers, sample icons) and not the menu code. Choosing a fan-out as the menu-wide sub-pie style pins the sub-pies that cannot fan out to replace style (`set_menu_sub_pie_style`) and says which.
  - **Buttons**: `button_tree` above, `button_inspector_bound` below (labels can be bound too) with the app's own "Appearance" group (icon, tooltip, colour, checkbox, cluster layout) among its controls and, at the end, the menu-wide settings that affect the selected node. The app keeps that appearance data in a table keyed by button path and follows `InspectorOutput::edits`.
- Centre: the canvas, with a toolbar (the **Demo | Configure** toggle, "Pin pie-menu", the code panel, the theme switch, the last action). The mode is always your choice; the app never switches it.
  - **Demo**: the real menu, opened with right-click or Space, so every change can be tried at once ("Pin pie-menu" keeps it open).
  - **Configure**: a preview of the node selected in the tree, on separate pinned menus: the ring that holds a button (the button outlined), the children of a replace-style sub-pie (with a breadcrumb to its parents), or a ring with the fan of a fan-out open. It uses no pointer and ignores number keys, so hover, reveal animation, sub-pie centre and hover delay, and the fan-out delay and opener action can only be judged in Demo mode (the editors say so). Click a button, or a cluster item, to select it in the tree. While a button problem exists, the preview keeps showing the last tree without one.
- Right (the "Code panel" checkbox shows or hides it): the generated code, live, with a Copy button. "Menu code" is the library-level code (buttons, sub-pies, mnemonics and a `.configure(..)` closure with the changed settings, with the bound expressions and a live block for the live ones). "Draw code" is a sketch of the render closure with the app's colours, tooltips, icons and cluster layouts. "TOML" is the menu as a [TOML file](../README.md#menus-from-toml-files), with the menu's name, files on disk to open (separated by commas, merged in order) and Open / Save (Save writes the first file). The **Examples** buttons open the example menus of [`examples/menus`](../examples/menus), built in: "Edit + plugin" (a base menu and a plugin extending it) and "+ user" (with a user's file that removes and changes buttons). The inspector's "Id" row (`id_editor`, offering the app's actions) names a button or cluster item for the file, and the last action shows the id of what was selected. Open reports the references the app does not answer (`check_references`). The app registers its actions as a program would (`Actions`): its pretend undo history, run by the ids it gets, enables Undo and Redo only when there is something to undo or redo, and the "Share plugin on" switch of the TOML tab shows and hides the plugin's buttons. The plugin's Print has an id the app does not handle, on purpose: it is disabled, marked ⚠ in the tree (`button_tree_marked`) and drawn in the error colour (`unhandled_color`).

## The editors, as a library

The library part of this crate is a set of embeddable pieces, with no window of its own. Put them in your own side panels, tune the menu live, and copy the generated code into your project.

The buttons editor is a tree plus an inspector. Everything it offers is legal: a direction that is taken (or, in a fan-out, faces the parent) is greyed out with the reason, a fan-out holds only plain and back buttons, "Remove" is hidden where it would leave an empty menu, sub-pie or cluster, and a mnemonic key or cluster item number that another button holds is refused. Turning a button into another kind keeps what it dropped aside, and converting back restores it; turning a replace-style sub-pie into a fan-out drops its back buttons that face the parent.

| Piece | What it does |
|---|---|
| `settings_editor(ui, &mut settings) -> bool` | controls for every settings field, one collapsing header per `SettingsGroup` |
| `field_editor(ui, &mut settings, SettingsField) -> bool` / `group_editor(ui, &mut settings, SettingsGroup) -> bool` | one field (drag values with units, reset to default, optional fields as a checkbox that remembers its last value), or one section (fields that do not apply, such as the slide settings without `SlideOut`, are left out); `SettingsField::ALL`, `SettingsGroup::fields()`, `field_label()` for building your own panels or a search |
| `button_tree(ui, &settings, &buttons, &mut Selection)` | collapsible tree of the buttons (`NE · Paste Special ▸`); a red warning sign marks a button whose subtree has a `check_buttons` problem |
| `button_tree_filtered(ui, &settings, &buttons, &mut Selection, &filter)` | the tree showing only the buttons a predicate accepts and the ones above them (a search); the way to every match stays open |
| `button_inspector(ui, &mut settings, &mut buttons, &mut Selection, extras) -> InspectorOutput` | identity, keys and contents of the selected node, your own controls (`extras`), and the menu-wide settings that affect it (`relevant_fields`); `InspectorOutput::edits` (`TreeEdit::Removed`/`Dropped`/`Added`) lets you keep per-button data of your own in step |
| `preview_target(&buttons, &settings, &Selection) -> PreviewTarget` | which level to show, which fan to open (`PieMenu::set_open_fan_out`) and what to highlight for a selection |
| `buttons_editor(ui, &mut buttons, auto_mnemonics) -> bool` | tree and inspector stacked, with the selection kept in egui memory |
| `set_menu_sub_pie_style(&mut settings, &mut buttons, style) -> Option<String>` | sets the menu-wide sub-pie style; going to a fan-out pins the sub-pies that cannot fan out to replace style and says which |
| `code_view(ui, &code)` | read-only monospace code area with a "Copy" button |
| `menu_to_code(title, &settings, &buttons) -> String` | a complete snippet: `use` lines, `let buttons = vec![..];`, `PieMenu::new()` with the `.configure(..)` closure, and a commented `show` skeleton |
| `buttons_to_code(&buttons)` / `settings_to_code(&settings)` | the two halves on their own; `settings_to_code` lists only what differs from the default |

```rust
use egui_pie_menu_designer::*;

egui::Panel::left("editor").show(ui, |ui| {
    egui::ScrollArea::vertical().show(ui, |ui| {
        settings_editor(ui, &mut self.menu.settings);
        buttons_editor(ui, &mut self.buttons, self.menu.settings.input.auto_mnemonics);
    });
});
egui::Panel::right("code").show(ui, |ui| {
    let code = menu_to_code(Some("Edit"), &self.menu.settings, &self.buttons);
    code_view(ui, &code);
});
```

For a live preview of the selected node (what the app's Configure mode does), compute `preview_target(&buttons, &settings, &selection)` each frame, keep one `PieMenu` per `level_path` (pinned at the centre of your canvas), call `set_open_fan_out(target.fan_trigger)` on it, and draw `show_with_pointer(ctx, level_buttons, None, true, ..)` with `target.highlight` and `target.item` outlined by your render closure. Pass the preview a clone of the settings with `sub_pies.hover_delay = None`, `input.use_mnemonic_keys = false` and no `pinned_dismiss_inputs`, and a tree that has no `check_buttons` problems (debug builds assert on them when a level opens).

Generated output (a small menu with a changed radius and mnemonics on):

```rust
use egui_pie_menu::*;

let buttons = vec![
    PieButton::new(PieDirection::North).with_label("Copy").with_mnemonic('c'),
    PieButton::new(PieDirection::East).with_label("More").with_children(vec![
        PieButton::back(PieDirection::West),
        PieButton::new(PieDirection::North).with_label("Paste"),
    ]),
];

let menu = PieMenu::new()
    .with_title("Edit")
    .configure(|s| {
        s.layout.radius = 120.0;
        s.input.auto_mnemonics = true;
    });
```

The generated code is verified by tests: the code for a rich button tree and for settings with a change in every group is compiled as real Rust (via macros) and has to rebuild exactly the edited buttons and settings. Code that needs `Duration` (durations in the settings) comes with `use std::time::Duration;`.

### Bindings: values from your program

Colours, strokes, fonts, labels and the title often come from your app rather than a literal: a theme colour, a translated label, a live count. The ⛓ button at the end of such a row binds it to a name: one of your app's variables (below), such as `theme.accent` or `i18n.edit.copy`. The value stays editable and is the fallback, and what the editor and the preview show until your app supplies the variable. A [TOML file](../README.md#menus-from-toml-files) keeps the name; generated Rust code reads the variable with its Rust expression. Without a registered variable, the name can be any Rust expression for the generated code (`format!("Undo ({n})")`).

For generated code, each binding reads its value either once or every frame (TOML files have no modes: a loaded menu takes your app's values whenever you apply them):

- **Build**: once, where the menu is built. The expression takes the literal's place: `s.title.text_color = theme.accent;`, `.with_label(tr("edit.copy"))`, `.with_title(tr("menu.edit"))`.
- **Live**: every frame. The menu is built with the value (a live settings field is left out of `configure`), and a block to run before `show` assigns the expression: `menu.settings.title.text_color = theme.accent;`, `buttons[1].label = format!("Undo ({n})");` (a button inside a sub-pie through `button_at_mut(&mut buttons, &[1, 0])`, a cluster item through `cluster_item_mut(number)`). A live label is assigned, so it has to be a `String`, and it can change its automatic mnemonic key as the text changes.

For an optional setting (`text_color`, the number hint colour) the expression is the inner value and the code wraps it in `Some(..)`. A `// needs in scope: theme, tr` line lists the names the expressions use. An empty expression is ignored (the code keeps the literal).

**Variables from your app.** Register your app's values each frame (`color`, `stroke`, `font`, `text`, `number`, and `flag` for true or false), each under a plain name, and, where generated code should read it differently from the name, with `.code(..)`: the Rust expression for the variable registered just before. Below a bound row, an "app value" picker then lists the variables of the right kind (with a colour swatch). A value bound to one shows the variable's real current value: its editor is greyed out and follows your app (switch your theme or language and the editors and the preview change with it). Typing a variable of the wrong kind (a font for a colour) is flagged.

```rust
let mut vars = Variables::new();
vars.color("theme.accent", self.theme.accent)
    .stroke("theme.outline", self.theme.outline)
    .font("theme.title_font", self.theme.title_font.clone())
    .text("i18n.edit.copy", tr("edit.copy")).code("tr(\"edit.copy\")");
self.bindings.set_variables(vars);
// Before drawing the editors and the preview:
self.bindings.apply(&mut self.menu.settings, &mut self.buttons);
self.bindings.apply_title(&mut self.title);
```

`apply` keeps the value each target had before your app's replaced it (`Bindings::fallback`), so a file saved afterwards keeps its defaults. A name that is not one of your registered variables is flagged below the row (a TOML file would keep the fallback for it). Labels of buttons and cluster items can be bound as well as the settings and the title. Whether a button is enabled or visible is not bound: it is the state of its action ([Actions](../README.md#menus-from-toml-files)); the inspector's checkboxes are for trying it out. (For generated Rust code, `BindTarget::Enabled` / `Visible` can still be set from your own code.)

The variables are not saved with the bindings (they are your app's state); register them again every frame.

| Piece | What it does |
|---|---|
| `Bindings` / `BindTarget` / `Binding` / `BindingMode` | the bindings of one menu, owned by you like the `Selection`; a target is a settings field (`SettingsField::is_bindable`: colours, strokes, fonts, numbers, durations), the title, a button's label, `enabled` or `visible` (by path) or a cluster item's label or `enabled` (by path and number). With the `serde` feature they serialize |
| `field_editor_bound` / `group_editor_bound` | `field_editor` / `group_editor` with the ⛓ toggle on bindable fields and the expression and Build / Live below a bound one |
| `button_inspector_bound(.., &mut bindings, &mut ids, &actions, extras)` | `button_inspector` with the ⛓ toggle on labels, and an "Id" row (`id_editor`) for buttons and cluster items; it keeps bindings and ids with their buttons as the tree changes (`follow`, `prune`, `renumber_item` on `Bindings` / `ButtonIds` for doing that yourself) |
| `binding_toggle` / `binding_details` | the two halves of a bound row, for your own rows (the app's "Menu title") |
| `Variables` / `VarValue` / `VarKind` | your app's values by name (`color`, `stroke`, `font`, `text`, `flag`), with `code` for the Rust expression of generated code (`code_for`); `Bindings::set_variables`, `apply` / `apply_title` (write the variable values in, keeping `fallback`s), `variable_for`, `code_expr` |
| `id_editor(ui, &mut ids, &key, &actions)` | an "Id" row for a button or cluster item, with a picker of the actions your program handles and a warning for an id that is none of them |
| `button_tree_marked(.., filter, &marked)` | `button_tree` (optionally filtered) that marks the buttons at `marked` with ⚠, e.g. those without an action (`Actions::unregistered`) |
| `menu_to_code_bound` | `menu_to_code` with the bindings; also `settings_to_code_bound`, `buttons_to_code_bound`, `live_bindings_code`, `free_identifiers` |

```rust
let out = button_inspector_bound(ui, &mut settings, &mut buttons, &mut selection, &mut bindings, &mut ids, &["edit.copy", "edit.undo"], |_, _, _| {});
// ...
let code = menu_to_code_bound(Some("Edit"), &settings, &buttons, &bindings);
```

```rust
use egui_pie_menu::*;
// needs in scope: tr, history, theme

let mut buttons = vec![
    PieButton::new(PieDirection::North).with_label(tr("copy")),
    PieButton::new(PieDirection::South).with_label("Undo"),
];

let mut menu = PieMenu::new()
    .with_title(tr("edit"));

// each frame, before show (live bindings):
menu.settings.title.text_color = theme.accent;
buttons[1].label = format!("Undo ({})", history.len());
```

The bound code is compiled in the tests too, with a stand-in `theme` and `tr`.

## License

MIT
