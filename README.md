# egui_pie_menu

A radial (pie) menu widget for [egui](https://github.com/emilk/egui), inspired by Blender's pie menus.

## Features

- Up to 8 buttons in cardinal and diagonal directions
- Caller-owned button content — render any egui widget inside each button slot
- Draws on the `Tooltip` layer, floating above all panels and unclipped
- Automatic screen-edge clamping: the whole menu shifts rather than clipping individual buttons
- Configurable layout shape: circle, square, or diamond via `shape_factor`, and the height of the diagonal buttons via `diagonal_height`
- Center indicator with arc, slice, and/or dot highlight driven by mouse distance
- Optional title label above the center indicator
- Keyboard support: number keys 1–9 (numpad or number row) and per-button mnemonic keys with underline rendering helper
- Quick-tap and double-tap detection with no timer required from the caller

## Installation

The crate is not published on crates.io. Depend on it through git and pin a release tag:

```toml
[dependencies]
egui_pie_menu = { git = "https://github.com/Deuracell/egui_pie_menu", tag = "v0.36.0" }
```

Release tags mirror the egui version they target: the minor version tracks egui (`0.36.x` targets egui 0.36).

> **Until 1.0, patch releases may contain breaking changes** — the minor version is reserved for the egui version. Read the commit log or release notes before moving to a newer tag. Since the crate is pinned by git tag, nothing updates without you changing the tag.

| Tag | egui / eframe |
|---|---|
| `v0.36.0` (`main`) | 0.36 |
| `v0.33.0` | 0.33 |

`main` always targets the newest egui. Pin a tag if you are on an older egui.

## Usage

### 1. Define buttons once

Buttons carry a direction, a label and a mnemonic setting. Visual content is provided by a closure at render time; the label is only used to generate mnemonics.

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

Mnemonic keys are consumed by the menu while it is open, so they won't trigger other shortcuts in the same frame. Duplicate mnemonics produce a warning on stderr in debug builds.

### 2. Create a `PieMenu` and open it

```rust
use egui_pie_menu::PieMenu;

struct MyApp {
    menu: PieMenu,
    menu_open: bool,
}
```

Open the menu when a trigger fires (e.g. right-click):

```rust
if ctx.input(|i| i.pointer.secondary_pressed()) {
    if let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) {
        self.menu.open(pos);
        self.menu_open = true;
    }
}
```

### 3. Call `show` every frame

```rust
use egui_pie_menu::PieMenuResponse;
use egui::Color32;

if self.menu_open {
    let mouse_pos = ctx.input(|i| i.pointer.latest_pos());
    let key_down = ctx.input(|i| i.pointer.secondary_down());

    match self.menu.show(ctx, &buttons, mouse_pos, key_down, Some("Edit"), |ui, b| {
        // `b` is a `ButtonView`: index, hover state, resolved mnemonic and number hint.
        // The label comes from the button; its mnemonic (if any) is underlined, and its
        // colour follows egui's widget states for the pie's hover (see "Text colour").
        ui.label(b.label_text());
    }) {
        PieMenuResponse::Selected(idx) => {
            println!("selected button {idx}");
            self.menu_open = false;
        }
        PieMenuResponse::Dismissed => { self.menu_open = false; }
        PieMenuResponse::QuickTap  => { /* default action */ self.menu_open = false; }
        PieMenuResponse::DoubleTap => { /* alternate action */ self.menu_open = false; }
        PieMenuResponse::None      => {}
    }
}
```

The render closure receives a `ButtonView` (nothing to precompute before `show`):

| Field / method | Meaning |
|---|---|
| `b.idx` | Index within the level shown (what `Selected(idx)` returns) |
| `b.hovered` | The pointer highlights this button |
| `b.button` | The `&PieButton` (label, direction, …) |
| `b.mnemonic` | The mnemonic key in effect (explicit or generated), if any |
| `b.number_hint` | A hint to draw inside your own button; `None` when number keys are off. `h.color` is resolved for the button's state (see "Text colour") |
| `b.label_text()` | The label with the mnemonic underlined, for `ui.label(..)` / `egui::Button::new(..)`. Colour per "Text colour" below, egui body font; chain `.color(c)`, `.font(f)` or `.prefix("🗑  ")` (an un-underlined icon) to override |

`show` signature:

```rust
pub fn show(
    &mut self,
    ctx: &egui::Context,
    buttons: &[PieButton],
    current_mouse_pos: Option<Pos2>,
    key_down: bool,                          // true while the hotkey that opened the menu is held
    title: Option<&str>,                     // label shown above the center indicator
    render_button: impl FnMut(&mut Ui, &ButtonView),  // (ui, view of the button)
) -> PieMenuResponse
```

## Interaction model

| Gesture | Response |
|---|---|
| Hold key/button, move mouse, release over a button | `Selected(idx)` |
| Hold key/button, move mouse to centre, release | `Dismissed` |
| Tap and release before crossing threshold (`OnMovement` only) | `QuickTap` |
| Two quick taps within `double_tap_window` (`OnMovement` only) | `DoubleTap` |
| Number key 1–9 or mnemonic key | `Selected(idx)` |
| Escape or number key 5 | `Dismissed` |

`show_behavior` (default `OnMovement { threshold: 10.0 }`) controls when the menu first appears:

- **`ShowBehavior::OnMovement { threshold }`** — the menu is not drawn until the mouse moves `threshold` pixels from the open position. Releasing before crossing that distance returns `QuickTap` or `DoubleTap`, so a plain tap feels instant with no visual flicker.
- **`ShowBehavior::Instant`** — the menu appears immediately on key hold. `QuickTap` and `DoubleTap` are never returned in this mode.

## Mnemonic labels

In the render closure use `ui.label(b.label_text())` (see above). For full control, `mnemonic_text` builds an egui `LayoutJob` that renders a string with the mnemonic character underlined. Pass it directly to `ui.label()`.

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

## Settings

All settings live on `PieMenu::settings: PieMenuSettings` and can be changed at any time.

```rust
use egui_pie_menu::ShowBehavior;

menu.settings.layout_radius = 120.0;
menu.settings.shape_factor  = 0.0;   // -1 diamond · 0 circle · +1 square
menu.settings.diagonal_height = Some(0.5); // NE/SE/SW/NW halfway between the N/S and E/W rows; None = from shape_factor
menu.settings.screen_margin = 8.0;   // minimum gap from screen edge in px

// Show immediately — QuickTap/DoubleTap are never returned
menu.settings.show_behavior = ShowBehavior::Instant;

// Or wait for mouse movement (default, enables tap detection)
menu.settings.show_behavior = ShowBehavior::OnMovement { threshold: 10.0 };

// Center background circle
menu.settings.center_indicator.background_radius     = SmartFloat::new(15.0);
menu.settings.center_indicator.background_fill_color = Color32::DARK_GRAY;

// Directional highlight
menu.settings.center_indicator.highlight_shape      = PieMenuHighlightShape::Slice;
menu.settings.center_indicator.highlight_fill_color = Color32::PURPLE;

// Title label
menu.settings.label.display    = true;
menu.settings.label.text_color = Color32::LIGHT_GRAY;

// Input
menu.settings.input.use_numpad_keys    = true;
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

### Pinned / persistent menus

```rust
menu.open(center);
menu.reveal();                              // visible immediately, regardless of ShowBehavior
menu.settings.block_background = false;     // keep the rest of the UI clickable while it is open
```

Keyboard: a blocking menu (`block_background = true`) takes keyboard focus away from any focused widget, so digits and mnemonics never end up in a text field. A non-blocking menu leaves typing alone: while egui wants keyboard input (a text field is focused), number keys and mnemonics go to that widget instead of the menu.

Settings structs, `PieButton` and `PieMenuResponse` derive `Clone`/`Debug` (the response is also `Copy + PartialEq`). Degenerate highlight settings (a `NaN`/disabled radius, zero-width strokes or arcs) no longer panic.


### Sub-pies

```rust
// Selecting a button with children opens them. Backspace or a click on the centre goes back.
// Inside the render closure use `menu.current_buttons(&root)` for labels; after `Selected`,
// `menu.selected_path()` is the full route to the leaf.
// Mnemonics are computed per level: `menu.mnemonics(menu.current_buttons(&root))`.
let root = vec![PieButton::new(PieDirection::North).with_children(vec![/* ... */])];
```

### Disabled buttons and hover query

```rust
PieButton::new(PieDirection::North).with_label("Cut").with_enabled(false)
// skipped by mouse, number keys and mnemonics; no generated mnemonic.
// The render closure runs with the Ui disabled (egui's greyed look, no hover outline);
// `b.enabled` is there if you want your own disabled styling too.

menu.hovered(); // Option<usize>: the currently highlighted button
```

Pointing at a direction with no (enabled) button snaps to the closest available one.

Number keys are consumed by the menu, so the digit does not also reach other handlers or text fields.

In debug builds the menu asserts, once per open, that the buttons of the level being shown are well-formed: two buttons in the same direction, or the same mnemonic on two enabled buttons, panic with a message. Buttons in different sub-pie levels are never compared, so levels may reuse directions and keys.

To check in any build, or to show the problems in an editor, call `menu.check_buttons(&buttons)`: it returns a `Vec<ButtonProblem>` (`SharedDirection` / `DuplicateMnemonic`, each with the two button indices, and a readable `Display`). The demo uses it to mark the offending buttons red, open a warning popup, and keep the menu closed until the config is fixed.

### Right-to-left and non-Latin scripts

- `menu.settings.mirror_horizontal = true` flips the layout (East↔West, diagonals follow). Number keys and number hints follow the displayed positions, and `HintSide::Trailing` then means left of the label. Off by default.
- `number_hints.digits = HintDigits::ArabicIndic` shows ٠١٢…, `HintDigits::Persian` shows ۰۱۲… (Persian/Urdu), instead of 0123….
- Mnemonics work for any alphabetic script: they match typed characters, so they work with the active keyboard layout, and mnemonic generation and `mnemonic_text` are Unicode-aware (case folded; Hebrew final forms, Arabic alef variants, and Persian/Arabic yeh and kaf count as the same key; combining marks are skipped).
- **Text rendering is egui's job.** As of egui 0.36 text is shaped (Arabic letters join), but there is **no bidi** support, so mixed left-to-right/right-to-left labels display in the wrong order, and egui's default fonts do not include Arabic or Hebrew glyphs — add a font via `ctx.set_fonts` (the demo loads system fonts). Keep each label single-script until egui gains bidi.

### Highlight shapes

`PieMenuHighlightShape` controls what is drawn at the center indicator when a direction is active. Components can be combined:

| Variant | Draws |
|---|---|
| `None` | nothing |
| `Arc` | arc along the indicator radius |
| `Slice` | filled wedge |
| `Circle` | dot offset in the hovered direction |
| `ArcSlice`, `ArcCircle`, `SliceCircle`, `ArcSliceCircle` | combinations |

## Running the demo

```sh
cargo run --example demo
```

  Right-click anywhere in the window to open the menu. The left panel exposes all settings as live sliders and color pickers.

## License

MIT
