# egui_pie_menu

A radial (pie) menu widget for [egui](https://github.com/emilk/egui), inspired by Blender's pie menus.

## Features

- Up to 8 buttons in cardinal and diagonal directions
- Caller-owned button content — render any egui widget inside each button slot
- Draws on the `Tooltip` layer, floating above all panels and unclipped
- Automatic screen-edge clamping: the whole menu shifts rather than clipping individual buttons
- Configurable layout shape: circle, square, or diamond via `layout.shape_factor`, and the height of the intercardinal (diagonal) buttons via `layout.intercardinal_height`
- Center indicator with arc, slice, and/or dot highlight driven by mouse distance
- Optional title label above the center indicator
- Keyboard support: number keys 1–9 (numpad or number row) and per-button mnemonic keys with underline rendering helper
- Quick-tap and double-tap detection with no timer required from the caller
- The menu tracks its own open state; pinned (persistent) menus are built in
- Menus defined in TOML files at runtime, merged across files (`toml` feature)

## Installation

The crate is not published on crates.io. Depend on it through git and pin a release tag:

```toml
[dependencies]
egui_pie_menu = { git = "https://github.com/Deuracell/egui_pie_menu", tag = "v0.36.1" }
```

Release tags mirror the egui version they target: the minor version tracks egui (`0.36.x` targets egui 0.36).

> **Until 1.0, patch releases may contain breaking changes** — the minor version is reserved for the egui version. Read the commit log or release notes before moving to a newer tag. Since the crate is pinned by git tag, nothing updates without you changing the tag.

> **Coming from v0.36.0?** v0.36.1 changes the API a lot (the `show` call, button and settings types, many renames). [Upgrading from v0.36.0](#upgrading-from-v0360) lists every breaking change with before and after.

| Tag | egui / eframe |
|---|---|
| `v0.36.1` | 0.36 |
| `v0.36.0` | 0.36 |
| `v0.33.0` | 0.33 |

A tag is a fixed snapshot: it never moves. `main` targets the newest egui and is ahead of the latest tag, with unreleased and possibly breaking changes, so depend on a tag rather than on `main`. Pin an older tag if you are on an older egui.

## Quick start

```rust
use egui_pie_menu::{PieButton, PieDirection, PieMenu, PieMenuResponse};

// `menu: PieMenu` lives in your app state (`PieMenu::new()`).
let buttons = [
    PieButton::new(PieDirection::North).with_label("Copy"),
    PieButton::new(PieDirection::South).with_label("Paste"),
];
// Open on right-click, at the pointer.
if ctx.input(|i| i.pointer.secondary_pressed())
    && let Some(pos) = ctx.input(|i| i.pointer.latest_pos())
{
    menu.open(pos);
}
// Show every frame; it does nothing while the menu is closed.
let key_down = ctx.input(|i| i.pointer.secondary_down());
match menu.show(ctx, &buttons, key_down, |ui, b| {
    ui.label(b.label_text());
}) {
    PieMenuResponse::Selected(idx) => println!("selected {idx}, route {:?}", menu.selected_path()),
    PieMenuResponse::QuickTap => println!("tapped without opening"),
    _ => {}
}
```

What you get with no configuration:

- Hold the right mouse button, flick towards a button and release to select it, or click a button with the left mouse button.
- Number keys 1–9 select the button in that direction (numpad or number row; 5 is the centre, so it dismisses). Buttons get a mnemonic key when you give them one with `with_mnemonic`.
- The menu appears once the mouse has moved 10 px; a quick tap without moving returns `QuickTap` (two taps in a row: `DoubleTap`).
- Escape, number key 5 and a right click dismiss the menu; so does releasing or clicking in the centre.
- A directional highlight in the centre follows the pointer, buttons fade in over 200 ms, and the menu shifts to stay on screen.
- Buttons with children open a sub-pie that replaces its parent at the pointer.
- The widgets behind the menu do not receive pointer input while it is open; the layout is not mirrored (right-to-left layouts are opt-in).

## Configuring the menu

Every setting lives in `menu.settings` (see [Settings](#settings) for the full list). There are three ways to set them; they combine freely.

```rust
use egui_pie_menu::{PieMenu, PieMenuSettings};

// 1. Assign fields on a menu you already have.
let mut menu = PieMenu::new().with_title("Edit");
menu.settings.layout.radius = 120.0;
menu.settings.input.auto_mnemonics = true;

// 2. `configure`: change the settings in a closure, in a single expression.
let menu = PieMenu::new().with_title("Edit").configure(|s| {
    s.layout.radius = 120.0;
    s.input.auto_mnemonics = true;
});

// 3. Struct update syntax, then `with_settings`.
let menu = PieMenu::new().with_settings(PieMenuSettings { mirror_horizontal: true, ..Default::default() });
```

`configure` edits the settings the menu has at that point, so it composes with `with_settings` and `with_title` in any order; a later `with_settings` replaces what an earlier `configure` changed.

### The `blender_like()` preset

`PieMenuSettings::blender_like()` is a ready-made starting point for a Blender-style feel. It changes exactly three things from the default:

- `input.select_on_primary_click = false`: a selection happens by releasing the key that opened the menu, and primary clicks are ignored.
- `text_color = Some(Color32::WHITE)`: white labels, hovered or not.
- `number_hints.color = Some(Color32::WHITE)`: white number hints, hovered or not.

Everything else is the default, and the result is an ordinary value you can keep changing:

```rust
let menu = PieMenu::new()
    .with_settings(PieMenuSettings::blender_like())
    .configure(|s| s.layout.radius = 110.0);
```

### Saving settings (`serde` feature)

Off by default. Enable it to (de)serialise the settings, for example to keep the user's choices in a config file:

```toml
egui_pie_menu = { git = "https://github.com/Deuracell/egui_pie_menu", tag = "v0.36.1", features = ["serde"] }
```

All settings types implement `Serialize` and `Deserialize`: `PieMenuSettings` and the groups inside it, `HoverEmphasis`, `HighlightShape` and the enums they use (`ShowBehavior`, `RevealAnimation`, `HintSide`, `HintDigits`, `SubPieStyle`, `SubPieCenter`, `FanOpenerAction`, `PieMenuDismissInput`). `PieButton` and `PieMenu` do not: buttons are built in code, and the menu holds runtime state. The feature also turns on egui's own `serde` feature, which makes the egui types in the settings (`Color32`, `Stroke`, `Margin`, `FontId`, `Key`, `PointerButton`) serialisable.

The settings structs use `#[serde(default)]`: a field missing from the saved data takes its default value. Settings saved by an older version therefore keep loading when a later version adds settings.

The crate does not pick a format. An example with `serde_json`, which you add to your own project (this snippet is not compiled here):

```rust
// Example using your own `serde_json` dependency.
let json = serde_json::to_string_pretty(&menu.settings)?;
std::fs::write("pie_menu.json", json)?;
// Later:
let json = std::fs::read_to_string("pie_menu.json")?;
menu.settings = serde_json::from_str(&json)?;
```

## Going further

- [Configuring the menu](#configuring-the-menu): `configure`, the `blender_like()` preset, saving settings with the `serde` feature
- [Buttons](#buttons): labels, mnemonics, disabled buttons, the hover query and `check_buttons`
- [Drawing your buttons](#drawing-your-buttons): the render closure, `ButtonView`, number hints, text colour, mnemonic labels
- [Reading the result](#reading-the-result): opening, `show`, `PieMenuResponse` and the interaction model
- [Sub-pies](#sub-pies): replace style, fan-out, back buttons, centre modes, hover navigation, break-out
- [Clusters](#clusters): one direction holding several clickable items
- [Pinned / persistent menus](#pinned--persistent-menus): menus that stay open
- [Menus from TOML files](#menus-from-toml-files): define and extend menus after compile time, ids for the buttons
- [Settings](#settings): every setting group, reveal animation, hover emphasis, highlight shapes
- [Right-to-left and non-Latin scripts](#right-to-left-and-non-latin-scripts): mirroring, digits, mnemonics
- [Examples and Pie Designer](#examples-and-pie-designer)
- [Upgrading from v0.36.0](#upgrading-from-v0360): every breaking change with before and after

## Buttons

Buttons carry a direction (`PieDirection` is `Copy`; `PieDirection::ALL` lists the eight, clockwise from North), a label and a mnemonic setting. Visual content is provided by a closure at render time (see "Drawing your buttons"); the label is only used to generate mnemonics.

```rust
use egui_pie_menu::{PieButton, PieDirection};

let buttons = vec![
    PieButton::new(PieDirection::North).with_label("Copy").with_mnemonic('c'), // explicit key
    PieButton::new(PieDirection::East).with_label("Redo"),                     // Auto (follows the menu)
    PieButton::new(PieDirection::South).with_label("Delete").without_mnemonic(), // never one
    PieButton::new(PieDirection::West).with_label("Undo"),
];
```

Each button's `mnemonic` is a `Mnemonic`:

| Value | Meaning |
|---|---|
| `Mnemonic::Auto` (default) | Follows the menu: generated from the label when `menu.settings.input.auto_mnemonics` is on, otherwise none. |
| `Mnemonic::Key(c)` (`with_mnemonic`) | This key, case-insensitive. Reserved, so generated mnemonics never duplicate it. |
| `Mnemonic::None` (`without_mnemonic`) | Never has a mnemonic, even with auto mnemonics on. |

With `auto_mnemonics` on (off by default), all `Auto` buttons are assigned together (with reshuffling) so as many as possible get a distinct letter from their label, preferring word initials, then the remaining letters left to right. Only a button with no possible letter gets none. Generated letters can move when other buttons or labels change (a button may give up its letter so another one gets any at all), so **if a particular letter matters, set it with `with_mnemonic`**. The keys in effect are on the `ButtonView` in the render closure (`b.mnemonic`), or from `menu.mnemonics(&buttons)` outside it.

Mnemonic keys are consumed by the menu while it is open, so they won't trigger other shortcuts in the same frame. A Latin mnemonic does not fire as part of a Ctrl/Cmd chord (Ctrl+C is not the mnemonic `c`). Duplicate mnemonics on one level panic in debug builds (see `check_buttons` below).

### Disabled buttons and hover query

```rust
PieButton::new(PieDirection::North).with_label("Cut").with_enabled(false)
// skipped by mouse, number keys and mnemonics; no generated mnemonic.
// The render closure runs with the Ui disabled (egui's greyed look, no hover outline);
// `b.enabled` is there if you want your own disabled styling too.

menu.hovered(); // Option<usize>: the currently highlighted button
```

Pointing at a direction with no (enabled) button snaps to the closest available one.

A hidden button (`.with_visible(false)`, or `button.visible = false`) is not drawn at all, and is skipped like a disabled one: no pointer pick, number key or mnemonic, and its mnemonic does not clash with another's. It keeps its place in the tree, so paths and ids stay the same, and its direction, which no other button of the level may take. Use it for buttons that come and go, such as a plugin's that is switched off.

Number keys are consumed by the menu, so the digit does not also reach other handlers or text fields.

In debug builds the menu asserts, once per open, that the buttons of the level being shown are well-formed: two buttons in the same direction, or the same mnemonic on two enabled buttons, panic with a message. Buttons in different sub-pie levels are never compared, so levels may reuse directions and keys.

To check in any build, or to show the problems in an editor, call `menu.check_buttons(&buttons)`: it returns a `Vec<ButtonProblem>` (`SharedDirection` / `DuplicateMnemonic` with the two button indices, `DuplicateItemNumber` / `DuplicateItemMnemonic` for cluster items (see "Clusters"; `MnemonicOwner` names a button or an item), plus the fan-out problems above, each with a readable `Display`; `p.buttons()` lists the indices involved). Pie Designer uses it to mark the offending buttons red, open a warning popup, and keep the menu closed until the config is fixed. `menu.check_tree(&buttons)` checks every level of a tree at once (the root and each sub-pie), with the path of each level that has a problem.

## Drawing your buttons

The render closure passed to `show` draws the content of each button; the library only positions it. It receives a `ButtonView` (nothing to precompute before `show`):

The render closure receives a `ButtonView` (nothing to precompute before `show`):

| Field / method | Meaning |
|---|---|
| `b.idx` | Index within the button's own level (what `Selected(idx)` returns for a button; for a cluster's item it is the number you gave it) |
| `b.path` | The full route to the button from the root level, own index last (tells a parent's buttons from an open fan-out's) |
| `b.hovered` | The pointer highlights this button |
| `b.button` | The `&PieButton` (label, direction, …) |
| `b.mnemonic` | The mnemonic key in effect (explicit or generated), if any |
| `b.number_hint` | A hint to draw inside your own button; `None` when number keys are off. `h.color` is resolved for the button's state (see "Text colour") |
| `b.scale()` | The scale to draw this button's content at: `1.0`, easing up to `HoverEmphasis::scale` while highlighted. `label_text()` uses it for the font size; multiply your own icons, paddings or minimum sizes by it |
| `b.text_color()` | The text colour in effect for this button (what `label_text()` uses), for extras you draw yourself |
| `b.opens_sub_pie()` | The button opens a sub-pie (replace style or fan-out) instead of being an action: lets you draw folder buttons differently |
| `b.opens_fan_out()` | The button opens its sub-pie as a fan-out (implies `opens_sub_pie()`) |
| `b.item(n, response)` | Inside a cluster button's closure: registers the widget behind `response` as item number `n` and returns the response unchanged (see "Clusters") |
| `b.item_label(n)` / `b.item_mnemonic(n)` | A declared cluster item's label with its mnemonic underlined (like `label_text()`), and the key in effect |
| `b.use_pie_hover(ui)` | Call at the top of the closure: egui widgets in this button look hovered while the pie highlights it, idle otherwise (the app's theme), ignoring the pointer over the widget itself |
| `b.label_text()` | The label with the mnemonic underlined, for `ui.label(..)` / `egui::Button::new(..)`. Colour per "Text colour" below, egui body font; chain `.color(c)`, `.font(f)` or `.prefix("🗑  ")` (an un-underlined icon) to override |

### Plain egui widgets and the pie's highlight

An egui widget drawn inside a pie button (an `egui::Button`, a checkbox) only reacts when the pointer is physically on it, and paints its own hover and press outline, while the pie highlights a button by direction (`b.hovered`). Call `b.use_pie_hover(ui)` at the top of the render closure, before adding widgets: every widget in that button then looks like a hovered widget while the pie highlights the button, and like an idle one otherwise, whatever the pointer does over the widget itself (no per-widget hover or press outline). It uses your app's own `widgets.hovered` / `widgets.inactive` visuals, so it follows the theme, and it changes only that button's `ui`. Disabled buttons keep egui's disabled look, and an explicit `.fill(..)` / `.stroke(..)` on a widget still wins.

```rust
b.use_pie_hover(ui);
```

### Number hints

```rust
// Inside your button (it grows to fit): use `b.number_hint` in the render closure.
let hint_gap = menu.settings.number_hints.spacing;
// … in the render closure:
// let label = b.label_text();
// let btn = match &b.number_hint {
//     Some(h) => {
//         let digit = egui::RichText::new(&h.text).color(h.color); // resolved for hover/disabled
//         if h.on_right { egui::Button::new((label, digit)) } else { egui::Button::new((digit, label)) }
//     }
//     None => egui::Button::new(label),
// }.gap(hint_gap);

menu.settings.number_hints.side = HintSide::Trailing;  // default: after the label in reading order; or Outside / Left / Right
menu.settings.number_hints.spacing = 4.0;              // suggested gap between label and digit (px)
menu.settings.number_hints.color = Some(Color32::WHITE); // force the digit colour (Blender-style); default None = egui's widget colours
menu.settings.number_hints.digits = HintDigits::ArabicIndic; // ٠١٢… (Persian: ۰۱۲…)
```

### Text colour

By default, labels (`b.label_text()`) and digits (`h.color`) use egui's widget text colours, so the app's theme applies: `visuals.widgets.inactive` normally and `visuals.widgets.hovered` on the button the pie highlights — the same dim/bright step as an egui button under the mouse.

To force a colour whether hovered or not (Blender-style white text), set it, independently for labels and digits:

```rust
menu.settings.text_color = Some(Color32::WHITE);              // labels
menu.settings.number_hints.color = Some(Color32::WHITE);      // digits
```

Disabled buttons are greyed out with egui's `Visuals::gray_out` either way. (Their render closure also runs in a disabled `Ui`, but egui does not fade text that carries an explicit colour, which `label_text()` does.)

### Icons and images

The library draws no icons: the render closure does, so a button can show whatever egui can draw. Three ways (Pie Designer's "Icons" section shows all of them):

```rust
// 1. Text or emoji prefix, un-underlined and in the label's colour.
ui.label(b.label_text().prefix("🗑  "));

// 2. A painted vector icon: allocate a square, paint in the label's colour.
let size = ui.text_style_height(&egui::TextStyle::Body) * b.scale();
let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(size), egui::Sense::hover());
ui.painter().line_segment([rect.left_top(), rect.right_bottom()], egui::Stroke::new(1.5, b.text_color()));

// 3. An image (a texture you uploaded once with `ctx.load_texture`), tinted like the text.
ui.add(egui::Image::from_texture(&texture).fit_to_exact_size(egui::Vec2::splat(size)).tint(b.text_color()));
```

Put the icon after the label when `b.rtl` is true (the layout is mirrored), and size it by `b.scale()` so hover emphasis enlarges it along with the text and stays crisp.

### Mnemonic labels

In the render closure use `ui.label(b.label_text())` (see "Drawing your buttons"). For full control, `mnemonic_text` builds an egui `LayoutJob` that renders a string with the mnemonic character underlined. Pass it directly to `ui.label()`.

```rust
use egui_pie_menu::{mnemonic_text, FontId, TextFormat};
use egui::Color32;

// Renders "Copy" with the 'C' underlined
ui.label(mnemonic_text("Copy", 'c', TextFormat {
    color: Color32::WHITE,
    font_id: FontId::default(),
    ..Default::default()
}));
```

- The match is **case-insensitive** — mnemonic `'c'` underlines `'C'` in `"Copy"`
- If the character isn't found in the string, the text is rendered unstyled with no underline
- `LayoutJob`, `TextFormat`, and `FontId` are re-exported from the crate root for convenience

For the hover growth that `b.scale()` follows, see "Hover emphasis" under Settings.

## Reading the result

### Creating, opening and showing the menu

Keep the menu in your app state. It tracks whether it is open, so you need no flag of your own.

```rust
use egui_pie_menu::PieMenu;

struct MyApp {
    menu: PieMenu, // e.g. PieMenu::new().with_title("Edit")
}
```

Open the menu when a trigger fires (e.g. right-click):

```rust
if ctx.input(|i| i.pointer.secondary_pressed())
    && let Some(pos) = ctx.input(|i| i.pointer.latest_pos())
{
    self.menu.open(pos);
}
```

`show` does nothing while the menu is closed, and closes it again on any response other than `None`, so call it unconditionally and handle only the responses you care about.

```rust
use egui_pie_menu::PieMenuResponse;

let key_down = ctx.input(|i| i.pointer.secondary_down());

match self.menu.show(ctx, &buttons, key_down, |ui, b| {
    // `b` is a `ButtonView`: index, hover state, resolved mnemonic and number hint.
    // The label comes from the button; its mnemonic (if any) is underlined, and its
    // colour follows egui's widget states for the pie's hover (see "Text colour").
    ui.label(b.label_text());
}) {
    // A button's index within its level, or the number of a cluster item (see "Clusters").
    PieMenuResponse::Selected(idx) => println!("selected {idx}, route {:?}", self.menu.selected_path()),
    PieMenuResponse::QuickTap  => { /* default action */ }
    PieMenuResponse::DoubleTap => { /* alternate action */ }
    PieMenuResponse::Dismissed | PieMenuResponse::None => {}
}
```

### `show` signature

```rust
pub fn show(
    &mut self,
    ctx: &egui::Context,
    buttons: &[PieButton],
    key_down: bool,                          // true while the key/button that opened the menu is held
    render_button: impl FnMut(&mut Ui, &ButtonView),  // (ui, view of the button)
) -> PieMenuResponse
```

### Related calls

| Call / field | Meaning |
|---|---|
| `menu.open(pos)` | Open at `pos` |
| `menu.pin(center)` | Open as a pinned menu (see "Pinned / persistent menus") |
| `menu.close()` | Close without a response |
| `menu.is_open()` / `menu.is_pinned()` | State queries |
| `menu.title = Some("Edit".into())` | Text above the centre indicator (`with_title` at construction) |
| `PieMenu::new().configure(\|s\| ..)` | Change the settings in a closure while building the menu (see "Configuring the menu") |
| `menu.show_with_pointer(ctx, &buttons, pointer, key_down, ..)` | `show` with your own pointer position instead of egui's latest one |
| `menu.pointer_area = Some(rect)` | The pointer only counts inside `rect` |

### Interaction model

| Gesture | Response |
|---|---|
| Hold key/button, move mouse, release over a button | `Selected(idx)` |
| Release with the pointer on a cluster item | `Selected(n)`, `n` = the item's number (whatever sector the item is in) |
| Release in a cluster's sector but not on one of its items | `Dismissed` (the cluster's direction highlights nothing) |
| Click on a cluster item (`select_on_primary_click`) | `Selected(n)` |
| Hold key/button, move mouse to centre, release | `Dismissed` |
| Release in a fan-out on a fan button | `Selected(..)` of that button |
| Release in a fan-out on its opening button | `BreakOut` (default): nothing, the fan becomes a replace-style sub-pie (click or keyboard driven); `Dismiss`: `Dismissed` (`settings.sub_pies.fan_out.opener_action`) |
| Release in a fan-out over nothing (blocked sector, dead zone, outside the window) | `Dismissed` |
| Click in a fan-out on its opening button | `BreakOut` (default): the fan becomes a replace-style sub-pie; `Dismiss`: the fan closes (`None`) |
| Click in a fan-out on nothing else | The fan closes, the menu stays open (`None`) |
| Release with nothing selectable (every button disabled, no pointer) | `Dismissed` |
| Tap and release before crossing threshold (`OnMovement` only) | `QuickTap` |
| Two quick taps within `double_tap_window` (`OnMovement` only) | `DoubleTap` |
| Number key 1–9 or mnemonic key | `Selected(idx)` (never for a cluster's number key; a cluster item's mnemonic selects that item: `Selected(n)`) |
| Click on the centre | Goes back one level in a sub-pie; at the root it dismisses the menu (`Dismissed`; not for a pinned menu) |
| Escape or number key 5 (consumed by the menu) | `Dismissed` |

`show_behavior` (default `OnMovement { threshold: 10.0 }`) controls when the menu first appears:

- **`ShowBehavior::OnMovement { threshold }`** — the menu is not drawn until the mouse moves `threshold` pixels from the open position. Releasing before crossing that distance returns `QuickTap` or `DoubleTap`, so a plain tap feels instant with no visual flicker.
- **`ShowBehavior::Instant`** — the menu appears immediately on key hold. `QuickTap` and `DoubleTap` are never returned in this mode.

## Sub-pies

```rust
// Selecting a button with children opens them as a sub-pie.
let root = vec![PieButton::new(PieDirection::North).with_children(vec![/* ... */])];
```

`Selected(idx)` is the chosen button's index within its own level (for a cluster's item: the item's number, see "Clusters"); `menu.selected_path()` is the full route to it from the root. In the render closure `b.path` is each button's route (own index last) and `b.button` the button itself, so you never need to look the level up.

There are two styles, which are different designs and not meant to be mixed lightly. Choose one for the menu; a single button can override it:

```rust
menu.settings.sub_pies.style = SubPieStyle::Replace;   // default; or SubPieStyle::FanOut
PieButton::new(dir).with_fan_out(children);            // this button fans out whatever the menu says
PieButton::new(dir).with_children(children).with_sub_pie_style(Some(SubPieStyle::Replace));
```

| `SubPieStyle` | What happens | Depth | Way back |
|---|---|---|---|
| `Replace` (default) | The sub-pie takes its parent's place: one level on screen at a time | any | Backspace, a click on the centre, or a back button |
| `FanOut` | The sub-pie opens around its button while the parent stays visible, greyed out | one (a fan's buttons cannot have sub-pies) | move the pointer back to the parent, or Backspace |

### Fan-out

```rust
let dir = PieDirection::East;
let root = vec![PieButton::new(dir).with_label("More").with_fan_out(vec![
    PieButton::new(PieDirection::North).with_label("Above"),   // ordinary directions …
    PieButton::new(PieDirection::East).with_label("Beyond"),
    PieButton::new(PieDirection::South).with_label("Under"),
])];
dir.fan_out_slots();                                    // … but only these five: [North, NorthEast, East, SouthEast, South]

menu.settings.sub_pies.fan_out.delay = Duration::ZERO;           // how long the pointer rests on the button first (default: none)
menu.settings.sub_pies.fan_out.radius = None;                    // the fan's own radius; None = layout.radius
menu.settings.sub_pies.fan_out.opener_action = FanOpenerAction::BreakOut; // selecting the fan's own button: break out (default) or Dismiss
```

- A fan holds up to five buttons. The three directions facing the parent (`dir.opposite()` and its two neighbours) are blocked: `check_buttons` reports a child there as `BlockedDirection`, and a child that is a sub-pie or cluster of its own as `NestedInFanOut`, both wrapped in `InFanOut { parent, problem }`.
- It opens when the pointer reaches its button (as far out as the ring of buttons) or when the button is selected by number key, mnemonic, click or release. It closes when the pointer comes back in towards the centre.
- While it is open it has the keyboard: number keys and mnemonics reach only the fan's buttons. The parent's centre and its other buttons are greyed out, and those buttons cannot be highlighted.
- The fan's highlighted button moves outward and its content grows slightly (see "Hover emphasis").
- The pointer picks among the fan's buttons by angle from the opening button's centre. On the opening button itself, or pointing back at the parent, nothing is highlighted.
- Selecting the fan's own opening button (releasing the key on it, or clicking it) is set by `settings.sub_pies.fan_out.opener_action`. `BreakOut` (default): the fan becomes a sub-pie of the same children, in place (Backspace or a centre click goes back). `Dismiss`: releasing on it dismisses the menu, and clicking it just closes the fan.
- The break-out transition: the sub-pie opens exactly where the fan was (same centre, radius and buttons, no jump and no re-reveal), the main ring, including the opening button, fades out where it was (still drawn through your closure while it fades, then gone), the old centre fades out and the new one fades in where it was (over `animations.reveal_duration`; instantly with `RevealAnimation::None`), and the children stay in place. A click on the centre or Backspace goes back and restores the main pie where it was.
- Releasing over nothing else in a fan dismisses the menu. A click on nothing else closes the fan (the menu stays open) and, like Backspace or a back button in the fan, it does not pop back open while the pointer rests on its button: move off the button and back to reopen it. The same holds after going back from a break-out.
- `menu.open_fan_out()` returns the opening button's index and the highlighted fan button, if a fan is open. A fan is not a level: `menu.path()` and `menu.current_buttons(&root)` still describe the parent.

### Replace

A replace-style sub-pie can carry its own way back, and you choose where it opens and whether hovering opens it:

```rust
// A back button: selecting it returns to the parent level instead of `Selected`.
// Opposite the opening button it points back the way the sub-pie was entered.
let dir = PieDirection::NorthEast;
let root = vec![PieButton::new(dir).with_label("Paste Special").with_children(vec![
    PieButton::back(dir.opposite()),                        // labelled "Back"; or set `kind: PieButtonKind::Back` on any button
    PieButton::new(PieDirection::North).with_label("As Text"),
])];

menu.settings.sub_pies.center = SubPieCenter::Button;        // Pointer (default) · Parent · Button
menu.settings.sub_pies.hover_delay = Some(Duration::from_millis(400)); // follow sub-pie and back buttons on hover; None (default) = off
```

| `SubPieCenter` | The sub-pie opens | Going back |
|---|---|---|
| `Pointer` (default) | at the pointer (a pinned menu stays put) | re-centres the parent on the pointer |
| `Parent` | in place of its parent | stays there |
| `Button` | at the button that opened it | returns to the parent's own centre |

With `settings.sub_pies.hover_delay`, resting the pointer on any button that leads to another pie follows it without a click, key or release: a sub-pie button opens its sub-pie, and a back button inside a sub-pie returns to the parent. Navigation flows: after a sub-pie opens, a sub-pie button under the resting pointer starts its own timer straight away, so you can rest on button after button without moving away in between. So that a resting pointer never bounces between levels, the button it points at after going back is ignored until the pointer has left it once, and so is a back button right after descending. Leaf buttons are never selected by hovering.

If the buttons change while a sub-pie is open so that its level no longer exists or has no buttons, the menu falls back to the root level. A pinned menu returns to its root level after a leaf selection.

Pie Designer's "Paste Special" button opens a sub-pie that works in both styles; the "Sub-pies" section switches the menu's style, and the button editor can give any button a sub-pie (to any depth) and its own style.

## Clusters

A cluster is a pie button that occupies one direction but holds **several independently clickable items**, laid out however you like: in a row, a column, a grid, with or without space between, inside frames. The library does not lay anything out. You declare the items on the button (number, label, mnemonic), draw whatever egui layout you want in the render closure, and tell the library which widget is which item with `b.item(n, response)`.

```rust
use egui_pie_menu::ClusterItem;

let buttons = vec![
    PieButton::cluster(PieDirection::North).with_items(vec![
        ClusterItem::new(7, "Cut"),               // numbers are yours, any order
        ClusterItem::new(3, "Copy"),
        ClusterItem::new(9, "Paste").with_mnemonic('a'),
    ]),
    PieButton::new(PieDirection::East).with_label("More"),
];
menu.show(ctx, &buttons, key_down, |ui, b| {
    if b.button.is_cluster() {
        ui.horizontal(|ui| {                       // a row; set `item_spacing.x = 30.0` for a spaced one
            b.item(7, ui.button(b.item_label(7)));
            b.item(3, ui.button(b.item_label(3)));
            b.item(9, ui.button(b.item_label(9)));
        });
        // A grid works the same: egui::Grid::new("g").show(ui, |ui| { b.item(1, ui.button("x")); ui.end_row(); .. });
    } else {
        ui.label(b.label_text());
    }
});
```

**Result.** Selecting item `n` returns `Selected(n)`, and `menu.selected_path()` is `[.., cluster_button_index, n]` (the same convention as a fan child: `Selected(i)` is the index within its own "level", here the number you defined; the second-to-last path element is the cluster button's index in its level). Keep numbers unique within one cluster (`check_buttons` reports repeats as `DuplicateItemNumber`).

**Behaviour.**
- The cluster's direction is dead: the pointer pointing at it highlights nothing (`menu.hovered()` is `None`, no emphasis, no centre highlight), and releasing the key there, not on an item, is a miss (`Dismissed`). The pointer never snaps past a cluster to a neighbouring button.
- An item is selected by a click on it (only when `settings.input.select_on_primary_click` is on) or by releasing the key while the pointer is on it. Both win over the angle-based pick: an item may extend into another button's sector.
- A disabled item (a disabled egui response, `ClusterItem::with_enabled(false)`) or an item of a disabled cluster button is never selected.
- Number keys never select a cluster, and it has no number hint (`b.number_hint` is `None`).
- Items have **mnemonics**: they compete with the level's buttons for keys, so no key is handed out twice. Auto items get a letter from their label (word initials first), `with_mnemonic(c)` reserves a key, `without_mnemonic()` and disabled items get none and reserve nothing. Pressing a mnemonic selects the item (`Selected(n)`) under the same rules as for buttons (typed characters, no Ctrl/Cmd chords, a focused text field keeps its keys in a non-blocking or pinned menu). `menu.cluster_mnemonics(&buttons)` lists them per cluster, `b.item_mnemonic(n)` / `b.item_label(n)` inside the closure. A number you draw with `b.item` but never declared still works by click and release, without a mnemonic.
- `check_buttons` reports `DuplicateItemNumber` and `DuplicateItemMnemonic` (an explicit key shared by an enabled item and another enabled button or item of the level).
- A cluster is not allowed inside a fan-out (`NestedInFanOut`); it works at the root, inside a replace-style sub-pie and in pinned menus. It has no children.
- A cluster is never highlighted by the pie, so the widgets inside use egui's native hover and press look; do not call `b.use_pie_hover` there.

Pie Designer's button editor has a "Cluster" checkbox on every button (layout, spacing, item labels and mnemonic keys); tick it and open the menu to try it.

## Pinned / persistent menus

```rust
menu.pin(center);                       // instead of open(): visible immediately
menu.pointer_area = Some(canvas_rect);  // optional: the pointer only counts in here
```

A pinned menu:

- is visible immediately, regardless of `ShowBehavior`, and ignores `key_down`;
- stays open after a selection (`Selected` is still returned each time);
- never blocks the widgets behind it;
- is dismissed only by `settings.input.pinned_dismiss_inputs` (default: Escape) or `menu.close()`.

Any direction highlights a button, so without a `pointer_area` a click anywhere in your UI would select the button the pointer points towards. Set it to the part of the window the menu owns. Move a pinned menu by updating `menu.position`.

Keyboard: a blocking menu (`block_background = true`, not pinned) takes keyboard focus away from any focused widget, so digits and mnemonics never end up in a text field. A non-blocking or pinned menu leaves typing alone: while egui wants keyboard input (a text field is focused), number keys and mnemonics go to that widget instead of the menu.

## Menus from TOML files

With the `toml` feature, menus can be defined in TOML files and loaded at runtime, so they can change without a rebuild, and other files (plugins, user overrides) can add to them.

```toml
[dependencies]
egui_pie_menu = { git = "https://github.com/Deuracell/egui_pie_menu", features = ["toml"] }
```

A file holds any number of menus, each under its name. A button opens another menu by naming it (`opens`), so a sub-pie can be defined in another file, and one menu can be the sub-pie of several buttons:

```toml
[menu.edit]
title = "Edit"

[menu.edit.settings.layout]
radius = 120.0

[[menu.edit.button]]
dir = "N"
label = "Copy"
id = "edit.copy"
mnemonic = "c"

[[menu.edit.button]]
dir = "E"
label = "Paste Special"
opens = "paste_special"   # another menu, in this file or any other
style = "fan_out"         # optional: "replace" or "fan_out"

[[menu.edit.button]]
dir = "W"
label = "Format"
items = [
    { number = 1, label = "B", id = "format.bold" },
    { number = 2, label = "I", id = "format.italic" },
]

[[menu.paste_special.button]]
dir = "N"
label = "Values Only"
id = "paste.values"
```

A second file can extend it. Here a plugin adds a button to `edit` that opens a menu of its own:

```toml
[[menu.edit.button]]
dir = "S"
label = "Share"
opens = "share"

[[menu.share.button]]
dir = "N"
back = true

[[menu.share.button]]
dir = "S"
label = "Email"
id = "plugin.share.email"
```

```rust
use egui_pie_menu::catalog::MenuCatalog;

let mut catalog = MenuCatalog::new();
catalog.add_file("menus/edit.toml")?;
catalog.add_file("plugins/share.toml")?;     // adds to `edit`
let mut edit = catalog.build("edit")?;        // MenuDefinition
let mut menu = edit.to_pie_menu();

// each frame:
edit.apply_actions(&actions(&app));           // enabled / visible, by id (below)
if let PieMenuResponse::Selected(_) = menu.show(ctx, &edit.buttons, key_down, |ui, b| { ui.label(b.label_text()); }) {
    match edit.ids.selected(&edit.buttons, menu.selected_path()) {
        Some("edit.copy") => copy(),
        Some("plugin.share.email") => share_by_email(),
        _ => {}
    }
}
```

**Buttons.** `dir` is `N`, `NE`, `E`, `SE`, `S`, `SW`, `W` or `NW` (or `North`, `NorthEast`, ..). A button is a plain button, a sub-pie (`opens = "menu"`), a back button (`back = true`, labelled "Back" unless given a `label`) or a cluster (`items`, each with a `number`). Optional: `label`, `id`, `mnemonic` (one character, or `"none"`; left out: automatic), and on a sub-pie `style`. A menu's own `style` says how it opens as a sub-pie; the button's `style` wins.

**Ids.** `id` names a button or cluster item for your code. `MenuDefinition::ids` (a `ButtonIds`) maps paths to ids: `selected(&buttons, menu.selected_path())` after a selection, `find(id)` for the path, `button(path)` / `item(path, number)`. Unlike a path, an id stays the same when another file adds or reorders buttons.

**Merging.** `add_toml` / `add_file` in order. A menu defined more than once (in several files, or twice in one) gets the buttons of every definition, in the order the files define them. Its `title`, `style` and settings (key by key within a group) are replaced by later definitions. A file that fails to load (bad TOML, an unknown key or setting) adds nothing.

**Removing and changing buttons.** A file can also take buttons out of a menu or change them, wherever they were defined, by their `id`. This is how a user's own file adjusts the base menu and the plugins':

```toml
# user/menus.toml
[menu.edit]
remove = ["edit.paste_special", "format.underline"]   # a button (with its sub-pie), a cluster item

[[menu.edit.change]]
id = "edit.cut"
label = "Snip"                # only the fields given change
mnemonic = "s"

[[menu.edit.change]]
id = "plugin.share"
dir = "SE"                    # move the plugin's button, to free South

[[menu.edit.button]]
dir = "S"
label = "Select All"
id = "edit.select_all"        # an action the program has but its menus did not show
```

A `change` can set `dir`, `label`, `mnemonic`, `opens`, `style` and `items` of a button, and `label` and `mnemonic` of a cluster item. `remove` and `change` are per menu: a sub-pie's buttons are changed in that sub-pie's own menu (`[[menu.paste_special.change]]`).

They apply when the menu is built, after every file is in: first every `change` (in file order, so a later file wins on the same field), then every `remove`. Only then are the buttons checked: two in one direction is an error naming both files. So the order of the files does not matter for removals and changes: the user's file can be loaded before or after the plugin it changes. A `remove` or `change` whose id is not in the menu (the plugin is not installed) is not an error; it is noted in `MenuDefinition::notes` and reported by `check_references`.

**Settings** go under `[menu.<name>.settings]`, with the paths of `PieMenuSettings`, and only what differs from the default. Colours are `"#rrggbb"` or `"#rrggbbaa"`, durations are milliseconds, and `false` switches off an optional value (`background_radius = false`). Compound values are inline tables: `highlight_stroke = { width = 2.0, color = "#ffffff" }`, `text_font = { size = 18.0, family = "Monospace" }`. An unknown setting is an error.

**Values from your program.** A file can't hold code, so a value your program supplies is a reference by name: a label, a cluster item's label, the title, or a colour, stroke, font, number or duration of the settings can be a literal or `{ var = "name", default = .. }`.

```toml
[menu.edit]
title = { var = "i18n.edit", default = "Edit" }

[menu.edit.settings.center_indicator]
highlight_fill_color = { var = "theme.accent", default = "#8000ff" }
highlight_stroke = { var = "theme.outline", default = { width = 2.0, color = "#ffffff" } }

[[menu.edit.button]]
dir = "N"
label = { var = "i18n.copy", default = "Copy" }
id = "edit.copy"

```

The names are plain keys of your app's [`Variables`](#variables-values-from-your-program), not Rust code. `edit.apply_variables(vars, &mut menu)` puts their current values in; call it when they change (or every frame). `default` is used until then and when your app has no such variable; without one, text shows the name itself (a missing translation stays visible) and a setting keeps the library default. A later file's literal replaces an earlier reference, like any other value.

A number is the value as written in the file (pixels, factors, angles in radians) and a duration is in milliseconds:

```toml
[menu.edit.settings.layout]
radius = { var = "prefs.pie_radius", default = 120.0 }

[menu.edit.settings.animations]
reveal_duration = { var = "prefs.animation_ms", default = 150 }
```

The rest (switches such as `title.show`, choices such as `reveal`) are literals only; a reference there is an error naming the setting.

#### Variables: values from your program

`egui_pie_menu::values::Variables` holds your program's values by name, each of a kind: `color`, `stroke`, `font`, `text`, `number` (a duration in milliseconds) and `flag` (true or false). Build it from your program's state and hand it to the menu; the parts of the menu that refer to a name take its value:

```rust
use egui_pie_menu::values::Variables;

let mut vars = Variables::new();
vars.color("theme.accent", theme.accent)
    .font("theme.title_font", theme.title_font.clone())
    .text("i18n.copy", tr("copy"))
    .number("prefs.pie_radius", prefs.pie_radius);
edit.apply_variables(vars, &mut menu);   // labels into edit.buttons; settings and title into menu
```

Calling it every frame is fine: it writes only what changed. The value a part had before your program's replaced it is kept as its fallback (`Bindings::fallback`), so a menu written back with `to_toml` keeps the file's own defaults. For menus built in code, `Bindings` (which part takes which variable) and `Bindings::apply` do the same without the `toml` feature.

**Actions: what the program handles, and their state.** Whether a button is enabled or shown is not something a file can know: it is the state of the *action* behind it, which only the program has. So files never say `enabled` or `visible`; the program registers the actions it handles, by id, with their state, and every button and cluster item with that id follows:

```rust
use egui_pie_menu::Actions;

/// The actions this program handles, and their state right now.
fn actions(app: &App) -> Actions {
    let mut a = Actions::new();
    a.add("edit.copy").enabled(app.selection.is_some());
    a.add("edit.undo").enabled(app.history.can_undo());
    a.add("edit.redo").enabled(app.history.can_redo());
    a.add("edit.select_all");
    a.add("paste.values");
    // A plugin's actions: shown only while it is switched on.
    let share_on = app.plugins.share.enabled();
    a.add("plugin.share").visible(share_on);
    a.add("plugin.share.email").visible(share_on).enabled(app.document.is_saved());
    a
}

// each frame (it only writes what changed):
edit.apply_actions(&actions(&app));
```

An action is enabled and visible unless the program says otherwise. A disabled button is greyed and cannot be picked; a hidden one is not drawn and keeps its place ([hidden buttons](#disabled-buttons-and-hover-query)). A cluster item cannot hide: a hidden action disables it. Buttons without an `id` are left alone. The `match` on the selected id then never sees an action that is disabled or hidden.

**Buttons without an action.** A button whose id the program does not register (a typo in a plugin's file, a plugin made for a newer version) is disabled, so it can never be picked and do nothing; it shows egui's disabled look. `apply_actions` returns those ids, each the first time it finds it, for you to log if you like. Whether such buttons look more alarming than that is up to your program, which knows who is looking (an end user cannot fix someone else's file): `edit.is_unhandled(path)` (`is_item_unhandled` for a cluster item) tells your render closure, and `unhandled_color(ui)` is the theme's error colour to draw it in:

```rust
menu.show(ctx, &edit.buttons, key_down, |ui, b| {
    let mut text = b.label_text();
    if edit.is_unhandled(b.path) {
        text = text.color(unhandled_color(ui));   // wins over the label's own colour
    }
    ui.label(text).on_disabled_hover_text("no action in the program");
});
```

`actions.keep_unregistered(true)` leaves such buttons as they are, for programs that pass ids they do not know on to someone else.

**Checking references.** Names and ids are only text to the file, so a typo goes unnoticed until a value stays at its default or a button is disabled. `check_references(&vars, &actions)` lists what your program does not answer: what building noted (a `remove` or `change` that found nothing), names it registers no variable for (or one of another kind, such as a font where a colour goes) and ids that are not among its actions. Run it after loading, and log or show the result:

```rust
let mut vars = Variables::new();
vars.text("i18n.copy", tr("copy"));
for problem in edit.check_references(&vars, &actions(&app)) {
    log::warn!("{problem}");
    // the label of "Copy": `i18n.cpy` is not a variable of the program
    // the id of "Share" › "Emial": `plugin.share.emial` is not an action of the program
    // `remove` in menu `edit` (from user/menus.toml): `format.italik` is not in the menu (nothing removed)
}
```

A menu's `title` and `settings` apply when it is built as a menu of its own. Where it opens as a sub-pie, only its buttons and `style` count.

**Errors.** `build` fails, with the file and the button named, if two buttons of a menu share a direction (after changes and removals), a `change` gives a cluster item a direction or the like, a button opens a menu no file defines, menus open each other in a circle, or the tree has a problem `check_buttons` reports (a mnemonic used twice, a fan-out child facing the parent, ..). Debug builds therefore never assert on a menu that built.

**Writing.** `MenuDefinition::to_toml()` writes a definition back as a file: the menu first, then one menu per sub-pie, named after its label (`paste_special`), with only the settings that differ from the default. Building it again gives the same definition. Whether buttons are enabled or visible is the program's state and is not written. A reference keeps the file's own default, not the value your program put in: a menu saved while the program runs in Russian keeps `default = "Copy"`. Pie Designer's "TOML" tab uses it to show, open and save menus.

## Settings

All settings live on `PieMenu::settings: PieMenuSettings` and can be changed at any time. Settings structs, `PieButton` and `PieMenuResponse` derive `Clone`/`Debug` (the response is also `Copy + PartialEq`). Degenerate highlight settings (a `NaN` radius, zero-width strokes or arcs) draw nothing instead of panicking.

At a glance:

| Path | Controls |
|---|---|
| `settings.layout` | Geometry of the ring: `radius`, `shape_factor`, `intercardinal_height`, `screen_margin`, `dead_zone_radius` |
| `settings.input` | Number keys, mnemonics, double-tap window, click-to-select, dismiss inputs |
| `settings.title` | Appearance of the title above the centre indicator |
| `settings.center_indicator` | Background circle and directional highlight (see "Highlight shapes") |
| `settings.animations` | Reveal animation (see "Reveal animation") and hover emphasis (see "Hover emphasis") |
| `settings.sub_pies` | How sub-pies are shown and navigated: `style`, `center`, `hover_delay`, and `fan_out` (`delay`, `radius`, `opener_action`) |
| `settings.number_hints` | Number-key hints: side, spacing, colour, digit shapes |
| `settings.show_behavior` | When the menu first appears (`Instant` or `OnMovement`) |
| `settings.block_background` | Whether the menu swallows pointer input meant for widgets behind it |
| `settings.mirror_horizontal` | Mirror the layout for right-to-left UIs |
| `settings.text_color` | Force the label colour |

```rust
use egui_pie_menu::ShowBehavior;

menu.settings.layout.radius = 120.0;
menu.settings.layout.shape_factor  = 0.0;   // -1 diamond · 0 circle · +1 square
menu.settings.layout.intercardinal_height = Some(0.5); // NE/SE/SW/NW halfway between the N/S and E/W rows; None = from layout.shape_factor
menu.settings.layout.screen_margin = 8.0;   // minimum gap from screen edge in px
menu.settings.layout.dead_zone_radius = 12.0; // no button is highlighted closer to the centre than this

// Show immediately — QuickTap/DoubleTap are never returned
menu.settings.show_behavior = ShowBehavior::Instant;

// Or wait for mouse movement (default, enables tap detection)
menu.settings.show_behavior = ShowBehavior::OnMovement { threshold: 10.0 };

// Center background circle
menu.settings.center_indicator.background_radius     = Some(15.0);   // None = no circle
menu.settings.center_indicator.background_fill_color = Color32::DARK_GRAY;

// Directional highlight
menu.settings.center_indicator.highlight_shape      = HighlightShape::SLICE;
menu.settings.center_indicator.highlight_fill_color = Color32::PURPLE;
menu.settings.center_indicator.highlight_snapping   = false; // true: snap to the hovered button's direction

// Title (the text itself is `menu.title`)
menu.settings.title.show       = true;
menu.settings.title.text_color = Color32::LIGHT_GRAY;
menu.settings.title.padding    = egui::Margin::same(5);

// Input
menu.settings.input.use_number_keys    = true;
menu.settings.input.use_mnemonic_keys  = true;
menu.settings.input.auto_mnemonics     = true;   // generate keys for Mnemonic::Auto buttons (default false)
menu.settings.input.double_tap_window  = Duration::from_millis(400);
```

### Reveal animation

```rust
// None (instant) · Fade (default) · SlideOut
menu.settings.animations.reveal = RevealAnimation::SlideOut;
menu.settings.animations.reveal_duration = Duration::from_millis(200);   // Fade / SlideOut (default 200 ms)
menu.settings.animations.slide_start_radius = 0.0;                       // 0 = from the centre
menu.settings.animations.slide_start_shape_factor = Some(-1.0);          // start as a diamond; None = final shape
menu.settings.animations.slide_fade = Some(Duration::from_millis(100));  // fade while sliding; None = no fade
```

### Hover emphasis

The highlighted button can move outward and/or grow, with separate amounts for the root pie and for sub-pies and fan-outs:

```rust
// Root pie: off by default (its centre highlight already shows the direction).
menu.settings.animations.hover_emphasis = HoverEmphasis { shift: 4.0, scale: 1.05 };
// Sub-pies and fan-outs: 6 px outward and 8% larger by default; a fan-out has no centre highlight of its own.
menu.settings.animations.sub_pie_hover_emphasis = HoverEmphasis::NONE;   // turn it off
menu.settings.animations.hover_duration = Duration::from_millis(100);    // easing, for both
```

`shift` is in pixels outward: the button's layer moves, in whole physical pixels. `scale` is the growth (`1.0` = none): it is applied to the content, never by scaling the layer (egui draws text as glyph images, so a layer scale blurs it). `b.label_text()` already uses it for the font size, which keeps the text crisp, and the button grows with its text. Anything else only grows if your closure scales it by `b.scale()` (icons, paddings, minimum sizes); custom fixed-size widgets stay as they are otherwise.

### Highlight shapes

`HighlightShape` controls what is drawn at the center indicator when a direction is active. It has three independent components:

| Field | Draws |
|---|---|
| `arc` | arc along the indicator radius |
| `slice` | filled wedge |
| `circle` | dot offset in the hovered direction |

```rust
use egui_pie_menu::HighlightShape;

menu.settings.center_indicator.highlight_shape = HighlightShape::SLICE; // default; also NONE, ARC, CIRCLE
menu.settings.center_indicator.highlight_shape = HighlightShape { arc: true, circle: true, slice: false };
```

## Right-to-left and non-Latin scripts

- `menu.settings.mirror_horizontal = true` flips the layout (East↔West, diagonals follow). Number keys and number hints follow the displayed positions, and `HintSide::Trailing` then means left of the label. Off by default.
- The mnemonic underline works in right-to-left words: egui (no bidi) lays separately formatted text pieces out left to right, so the library feeds the split word in reverse and, for Arabic-script labels, adds zero-width joiners at the cuts so letters keep their joined forms.
- In the render closure `b.rtl` is true while mirrored: lay out your own widgets in reverse (Pie Designer puts a checkbox's box right of its text). `b.label_text().prefix(icon)` already puts the icon on the right.
- `number_hints.digits = HintDigits::ArabicIndic` shows ٠١٢…, `HintDigits::Persian` shows ۰۱۲… (Persian/Urdu), instead of 0123….
- Mnemonics work for any alphabetic script: they match typed characters, so they work with the active keyboard layout, and mnemonic generation and `mnemonic_text` are Unicode-aware (case folded; Hebrew final forms, Arabic alef variants, and Persian/Arabic yeh and kaf count as the same key; combining marks are skipped).
- **Text rendering is egui's job.** As of egui 0.36 text is shaped (Arabic letters join), but there is **no bidi** support, so mixed left-to-right/right-to-left labels display in the wrong order, and egui's default fonts do not include Arabic or Hebrew glyphs — add a font via `ctx.set_fonts` (the Pie Designer embeds them). Keep each label single-script until egui gains bidi.
- **Status: basic RTL support, further work tabled.** Mirroring, digits, mnemonic matching and underlining, icon and checkbox side are handled. Multi-word right-to-left labels still show their words in left-to-right order (egui has no bidi). Contributions welcome: the [`unicode-bidi`](https://crates.io/crates/unicode-bidi) crate could compute the visual word order so the library feeds egui the words already reordered (the same trick `append_mnemonic_text` uses for the mnemonic's word). Once egui moves to a bidi-aware text stack (Parley / cosmic-text), the workarounds should be removed instead; the test `rtl_mnemonics_render_like_the_unsplit_label` compares against egui's own layout and should flag that change.

## Examples and Pie Designer

Two short examples show the library in code:

```sh
cargo run --example basic        # a menu built in code: sub-pie, cluster, handling the selection
cargo run --example toml_menus   # menus from TOML files: variables, actions, ids
```

**Pie Designer** (the `egui_pie_menu_designer` crate in this repository) is the place to try everything the menu can do and to design menus visually: every setting, the button tree with an inspector, a live preview, and the result as TOML or as Rust code. Its editors are also a library you can embed in your own program's tools. It runs on the desktop and in the browser (the repository's GitHub Pages site). See [designer/README.md](designer/README.md).

```sh
cargo run -p egui_pie_menu_designer
```

## Upgrading from v0.36.0

v0.36.1 changes the API a lot. Pre-1.0 patch releases may break (see [Installation](#installation)); the compiler points at every item below and each has a short fix. Everything not listed here keeps working as before.

### The `show` loop

Before (v0.36.0):

```rust
// app state: menu: PieMenu, menu_open: bool
if self.menu_open {
    let mouse_pos = ctx.input(|i| i.pointer.latest_pos());
    let key_down = ctx.input(|i| i.pointer.secondary_down());
    match self.menu.show(ctx, &buttons, mouse_pos, key_down, Some("Edit"), |ui, idx, hovered| {
        ui.label(LABELS[idx]);
    }) {
        PieMenuResponse::Selected(idx) => { /* … */ self.menu_open = false; }
        PieMenuResponse::Dismissed | PieMenuResponse::QuickTap | PieMenuResponse::DoubleTap => self.menu_open = false,
        PieMenuResponse::None => {}
    }
}
```

After:

```rust
// app state: menu: PieMenu::new().with_title("Edit")   (no menu_open flag)
let key_down = ctx.input(|i| i.pointer.secondary_down());
match self.menu.show(ctx, &buttons, key_down, |ui, b| {
    ui.label(b.label_text());
}) {
    PieMenuResponse::Selected(idx) => { /* … */ }
    _ => {}
}
```

- **The menu owns its open state.** `open(pos)` opens it; `show` does nothing while it is closed and closes it on any response other than `None` (a pinned menu stays open after a selection). Drop your `menu_open` flag.
- **The pointer comes from egui**; `show_with_pointer` takes one you supply. The title moved from the `show` argument to `menu.title` (or `with_title`).
- **The render closure receives a `&ButtonView`** (`b.idx`, `b.hovered`, `b.label_text()`, ...) instead of `(ui, idx, hovered)`.

### Buttons

- `PieButton` was `{ direction, mnemonic: Option<char> }`. It now also has a label, an enabled flag and a kind, and `mnemonic` is a `Mnemonic` (`Auto`, `Key(c)`, `None`). `with_mnemonic(c)` works as before; `b.mnemonic` in the closure is the key in effect (an `Option<char>`).
- Labels live on the buttons: `PieButton::new(dir).with_label("Copy")`. `b.label_text()` draws the label with its mnemonic underlined in the theme's colours, replacing hand-made `mnemonic_text` calls (which still exist).

### Settings

| v0.36.0 | v0.36.1 |
|---|---|
| `settings.layout_radius` | `settings.layout.radius` |
| `settings.shape_factor` | `settings.layout.shape_factor` |
| `settings.screen_margin` | `settings.layout.screen_margin` |
| `settings.mouse_trigger_threshold` | `settings.layout.dead_zone_radius` |
| `settings.input.use_numpad_keys` | `settings.input.use_number_keys` (numpad and number row both work) |
| `settings.label` (`PieMenuLabelSettings`) | `settings.title` (`PieMenuTitleSettings`) |
| `label.display`, `label.padding: Padding` | `title.show`, `title.padding: egui::Margin` |
| `label.background_stroke` (width `NAN` = off) | `title.background_stroke` (width `0` = off, the default) |
| `animations.center_highlight_snapping` | `center_indicator.highlight_snapping` |
| `animations.center_highlight_show` | removed: set `highlight_shape = HighlightShape::NONE` |

### Types

- `SmartFloat<f32>` (`background_radius`, `highlight_radius`; `NAN` = off) is now `Option<f32>` (`None` = off). `SmartFloat` and the `num-traits` dependency are gone.
- `PieMenuHighlightShape` (eight variants) is now `HighlightShape { arc, slice, circle }` with the constants `NONE`, `ARC`, `SLICE` and `CIRCLE`; for example `ArcSlice` becomes `HighlightShape { arc: true, slice: true, circle: false }`.
- `Padding` is replaced by `egui::Margin`.
- `PieDirection` is `Copy` now (drop the `.clone()` calls).

### Behaviour changes

- Dismiss keys (Escape, `5`) are consumed by the menu, and number keys, mnemonics and dismiss keys are ignored while Ctrl or Cmd is held (Ctrl+C is not the mnemonic `c`).
- The menu no longer changes your app's `selectable_labels` setting (v0.36.0 switched it globally while the menu was open); the labels inside the pie's buttons are simply not selectable.
- Timing uses egui's clock instead of `std::time::Instant`, so the crate works on wasm.
- Debug builds assert (panic) when two buttons of one level share a direction or an enabled mnemonic (v0.36.0 printed a warning for duplicate mnemonics); `check_buttons` returns the same problems in any build.
- A click in the centre of the main pie dismisses the menu (releasing there always did), and so does releasing with nothing selectable.

### New in v0.36.1

Nothing below needs changes in existing code:

- [Sub-pies](#sub-pies) (replace style and fan-out, back buttons, centre modes, hover navigation) and [clusters](#clusters).
- [Pinned menus](#pinned--persistent-menus), disabled buttons, auto-generated and Unicode mnemonics, `check_buttons`.
- Number hints, right-to-left mirroring and Arabic-Indic or Persian digits.
- Reveal animations (fade, slide out) and [hover emphasis](#hover-emphasis).
- `ButtonView` helpers: `path`, `text_color()`, `scale()`, `use_pie_hover()`, `opens_sub_pie()`.
- The `layout` and `sub_pies` setting groups, and demo icons ([Icons and images](#icons-and-images)).

### New in v0.36.2

One change can need an edit in existing code:

- `PieButton` has a new public field, `visible` (below). Code that builds buttons with the builders (`PieButton::new(..).with_label(..)`) is unaffected; a `PieButton { .. }` struct literal needs `visible: true` (or `..` from another button).

Everything else is additive:

- `PieMenu::configure`: change the settings in a closure.
- `PieMenuSettings::blender_like()`: a Blender-style preset ([Configuring the menu](#configuring-the-menu)).
- The optional `serde` feature: `Serialize` and `Deserialize` for the settings types, with `#[serde(default)]` on the structs.
- The optional `toml` feature: menus in TOML files, loaded and merged at runtime (`catalog::MenuCatalog`, `MenuDefinition`), with ids for the buttons (`ButtonIds`), `remove` / `change` by id, values from the program by name, and the program's actions setting buttons' state (`Actions`) ([Menus from TOML files](#menus-from-toml-files)).
- `values`: `Variables` (the program's values by name) and `Bindings` (which parts of a menu take them); `settings_fields`: `SettingsField` / `SettingsGroup`, every setting by path.
- `PieButton::visible` / `with_visible`: hidden buttons keep their place but are not drawn or picked.
- `button_at` / `button_at_mut` (a button by path), `PieButton::children_mut` / `cluster_item_mut`.
- `PieMenu::check_tree` (every level of a tree), `PieMenu::fan_out_problems` (could this sub-pie open as a fan-out), and the `mnemonic` module's `fold_char` / `is_mnemonic_letter`.
- `PieMenu::set_open_fan_out(Option<usize>)`: open (or close) a fan-out from code as if the pointer had reached its button, for previews and editors. It has no effect unless the menu is open; a real pointer elsewhere in the parent closes the fan again at once, while `show_with_pointer(.., None, ..)` leaves it open.
- Pie Designer, a crate of its own in this repository (`egui_pie_menu_designer`): the editors and the app ([Examples and Pie Designer](#examples-and-pie-designer)). The demo of v0.36.1 is now part of it; the core has two short examples instead.

## License

MIT

