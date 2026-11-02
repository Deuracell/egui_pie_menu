# egui_pie_menu

A radial (pie) menu widget for [egui](https://github.com/emilk/egui), inspired by Blender's pie menus.

## Features

- Up to 8 buttons in cardinal and diagonal directions
- Caller-owned button content — render any egui widget inside each button slot
- Draws on the `Tooltip` layer, floating above all panels and unclipped
- Automatic screen-edge clamping: the whole menu shifts rather than clipping individual buttons
- Configurable layout shape: circle, square, or diamond via `shape_factor`
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

Release tags mirror the egui version they target:

| Tag | egui / eframe |
|---|---|
| `v0.36.0` (`main`) | 0.36 |
| `v0.33.0` | 0.33 |

`main` always targets the newest egui. Pin a tag if you are on an older egui.

## Usage

### 1. Define buttons once

Buttons carry a direction and an optional mnemonic key. Visual content is provided by a closure at render time.

```rust
use egui_pie_menu::{PieButton, PieDirection};

let buttons = vec![
    PieButton::new(PieDirection::North).with_mnemonic('c'),
    PieButton::new(PieDirection::East).with_mnemonic('r'),
    PieButton::new(PieDirection::South).with_mnemonic('d'),
    PieButton::new(PieDirection::West).with_mnemonic('u'),
];
```

To generate mnemonics automatically (opt-in), call `assign_mnemonics(&mut buttons, &labels)` once, or whenever labels change. Explicit `with_mnemonic` keys are never altered and are reserved, so generated ones never duplicate them; all buttons are assigned together (with reshuffling) so as many as possible get a distinct letter from their label, preferring word initials. Only a button with no possible letter gets none.

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
use egui_pie_menu::{mnemonic_text, FontId, PieMenuResponse, TextFormat};
use egui::Color32;

if self.menu_open {
    let mouse_pos = ctx.input(|i| i.pointer.latest_pos());
    let key_down = ctx.input(|i| i.pointer.secondary_down());

    match self.menu.show(ctx, &buttons, mouse_pos, key_down, Some("Edit"),
        |ui, idx, hovered| {
            let label = ["Copy", "Redo", "Delete", "Undo"][idx];
            let color = if hovered { Color32::WHITE } else { Color32::LIGHT_GRAY };
            // mnemonic_text underlines the mnemonic character in the label
            if let Some(c) = buttons[idx].mnemonic {
                ui.label(mnemonic_text(label, c, TextFormat {
                    color,
                    font_id: FontId::default(),
                    ..Default::default()
                }));
            } else {
                ui.label(label);
            }
        })
    {
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

`show` signature:

```rust
pub fn show(
    &mut self,
    ctx: &egui::Context,
    buttons: &[PieButton],
    current_mouse_pos: Option<Pos2>,
    key_down: bool,                          // true while the hotkey that opened the menu is held
    title: Option<&str>,                     // label shown above the center indicator
    render_button: impl FnMut(&mut Ui, usize, bool), // (ui, index, is_hovered)
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

`mnemonic_text` builds an egui `LayoutJob` that renders a string with the mnemonic character underlined. Pass it directly to `ui.label()`.

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
menu.settings.input.double_tap_window  = Duration::from_millis(400);
```

### Reveal animation

```rust
menu.settings.animations.reveal = RevealAnimation::SlideOut;  // or Fade / None (default)
menu.settings.animations.reveal_duration = Duration::from_millis(150);
```

### Number hints

```rust
menu.settings.number_hints.show = true;      // gray digit beside each button
menu.settings.number_hints.side = HintSide::Right;
menu.settings.number_hints.arabic_indic_digits = true; // ٠١٢…
```

### Right-to-left and non-Latin scripts

- `menu.settings.mirror_horizontal = true` flips the layout (East↔West, diagonals follow). Number keys and number hints follow the displayed positions. Off by default.
- `menu.settings.number_hints.side` / `.arabic_indic_digits` place the hint digit on the right and use ٠١٢…
- Mnemonics work for any alphabetic script: they match typed characters, so they work with the active keyboard layout, and `assign_mnemonics` / `mnemonic_text` are Unicode-aware (case folded; Hebrew final forms and Arabic alef variants are treated as the same key; combining marks are skipped).
- **Text rendering is egui's job.** As of egui 0.36 text is shaped (Arabic letters join), but there is **no bidi** support, so mixed left-to-right/right-to-left labels display in the wrong order, and egui's default fonts do not include Arabic or Hebrew glyphs — add a font via `ctx.set_fonts`. Keep each label single-script until egui gains bidi.

### Sub-pies

```rust
// Selecting a button with children opens them. Backspace or a click on the centre goes back.
// Inside the render closure use `menu.current_buttons(&root)` for labels; after `Selected`,
// `menu.selected_path()` is the full route to the leaf.
let root = vec![PieButton::new(PieDirection::North).with_children(vec![/* ... */])];
```

### Disabled buttons, empty sectors, hover query

```rust
PieButton::new(PieDirection::North).with_enabled(false) // skipped by mouse, number keys, mnemonics
// in your closure: `menu.current_buttons(&root)[idx].enabled` to grey it out

menu.settings.input.ignore_empty_sectors = true; // pointing at a direction with no button selects nothing
menu.hovered(); // Option<usize>: the currently highlighted button
```

Settings structs, `PieButton` and `PieMenuResponse` derive `Clone`/`Debug` (the response is also `Copy + PartialEq`). Degenerate highlight settings (a `NaN`/disabled radius, zero-width strokes or arcs) no longer panic.

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
