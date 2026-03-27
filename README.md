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
- Keyboard support: numpad 1–9 and per-button mnemonic keys with underline rendering helper
- Quick-tap and double-tap detection with no timer required from the caller

## Installation

```toml
[dependencies]
egui_pie_menu = "0.1"
```

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
| Numpad 1–9 or mnemonic key | `Selected(idx)` |
| Escape or Numpad 5 | `Dismissed` |

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
