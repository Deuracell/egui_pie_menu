# egui_pie_menu

A radial (pie) menu widget for [egui](https://github.com/emilk/egui), inspired by Blender's pie menus.

## Features

- Up to 8 buttons in cardinal and diagonal directions
- Caller-owned button content — render any egui widget inside each button slot
- Draws on the `Tooltip` layer, floating above all panels and unclipped
- Automatic screen-edge clamping: the whole menu shifts rather than clipping individual buttons
- Configurable layout shape: circle, square, or diamond via `shape_factor`, and the height of the intercardinal (diagonal) buttons via `intercardinal_height`
- Center indicator with arc, slice, and/or dot highlight driven by mouse distance
- Optional title label above the center indicator
- Keyboard support: number keys 1–9 (numpad or number row) and per-button mnemonic keys with underline rendering helper
- Quick-tap and double-tap detection with no timer required from the caller
- The menu tracks its own open state; pinned (persistent) menus are built in

## Installation

The crate is not published on crates.io. Depend on it through git and pin a release tag:

```toml
[dependencies]
egui_pie_menu = { git = "https://github.com/Deuracell/egui_pie_menu", tag = "v0.36.1" }
```

Release tags mirror the egui version they target: the minor version tracks egui (`0.36.x` targets egui 0.36).

> **Until 1.0, patch releases may contain breaking changes** — the minor version is reserved for the egui version. Read the commit log or release notes before moving to a newer tag. Since the crate is pinned by git tag, nothing updates without you changing the tag.

| Tag | egui / eframe |
|---|---|
| `v0.36.1` | 0.36 |
| `v0.36.0` | 0.36 |
| `v0.33.0` | 0.33 |

A tag is a fixed snapshot: it never moves. `main` targets the newest egui and is ahead of the latest tag, with unreleased and possibly breaking changes, so depend on a tag rather than on `main`. Pin an older tag if you are on an older egui.

## Usage

### 1. Define buttons once

Buttons carry a direction (`PieDirection` is `Copy`; `PieDirection::ALL` lists the eight, clockwise from North), a label and a mnemonic setting. Visual content is provided by a closure at render time; the label is only used to generate mnemonics.

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

### 2. Create a `PieMenu` and open it

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

### 3. Call `show` every frame

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
    PieMenuResponse::Selected(idx) => println!("selected button {idx}"),
    PieMenuResponse::QuickTap  => { /* default action */ }
    PieMenuResponse::DoubleTap => { /* alternate action */ }
    PieMenuResponse::Dismissed | PieMenuResponse::None => {}
}
```

The render closure receives a `ButtonView` (nothing to precompute before `show`):

| Field / method | Meaning |
|---|---|
| `b.idx` | Index within the button's own level (what `Selected(idx)` returns) |
| `b.path` | The full route to the button from the root level, own index last (tells a parent's buttons from an open fan-out's) |
| `b.hovered` | The pointer highlights this button |
| `b.button` | The `&PieButton` (label, direction, …) |
| `b.mnemonic` | The mnemonic key in effect (explicit or generated), if any |
| `b.number_hint` | A hint to draw inside your own button; `None` when number keys are off. `h.color` is resolved for the button's state (see "Text colour") |
| `b.scale()` | The scale to draw this button's content at: `1.0`, easing up to `HoverEmphasis::scale` while highlighted. `label_text()` uses it for the font size; multiply your own icons, paddings or minimum sizes by it |
| `b.text_color()` | The text colour in effect for this button (what `label_text()` uses), for extras you draw yourself |
| `b.opens_sub_pie()` | The button opens a sub-pie (replace style or fan-out) instead of being an action: lets you draw folder buttons differently |
| `b.opens_fan_out()` | The button opens its sub-pie as a fan-out (implies `opens_sub_pie()`) |
| `b.use_pie_hover(ui)` | Call at the top of the closure: egui widgets in this button look hovered while the pie highlights it, idle otherwise (the app's theme), ignoring the pointer over the widget itself |
| `b.label_text()` | The label with the mnemonic underlined, for `ui.label(..)` / `egui::Button::new(..)`. Colour per "Text colour" below, egui body font; chain `.color(c)`, `.font(f)` or `.prefix("🗑  ")` (an un-underlined icon) to override |

`show` signature:

```rust
pub fn show(
    &mut self,
    ctx: &egui::Context,
    buttons: &[PieButton],
    key_down: bool,                          // true while the key/button that opened the menu is held
    render_button: impl FnMut(&mut Ui, &ButtonView),  // (ui, view of the button)
) -> PieMenuResponse
```

**Plain egui widgets and the pie's highlight.** An egui widget drawn inside a pie button (an `egui::Button`, a checkbox) only reacts when the pointer is physically on it, and paints its own hover and press outline, while the pie highlights a button by direction (`b.hovered`). Call `b.use_pie_hover(ui)` at the top of the render closure, before adding widgets: every widget in that button then looks like a hovered widget while the pie highlights the button, and like an idle one otherwise, whatever the pointer does over the widget itself (no per-widget hover or press outline). It uses your app's own `widgets.hovered` / `widgets.inactive` visuals, so it follows the theme, and it changes only that button's `ui`. Disabled buttons keep egui's disabled look, and an explicit `.fill(..)` / `.stroke(..)` on a widget still wins.

```rust
b.use_pie_hover(ui);
```

Related:

| Call / field | Meaning |
|---|---|
| `menu.open(pos)` | Open at `pos` |
| `menu.pin(center)` | Open as a pinned menu (see "Pinned / persistent menus") |
| `menu.close()` | Close without a response |
| `menu.is_open()` / `menu.is_pinned()` | State queries |
| `menu.title = Some("Edit".into())` | Text above the centre indicator (`with_title` at construction) |
| `menu.show_with_pointer(ctx, &buttons, pointer, key_down, ..)` | `show` with your own pointer position instead of egui's latest one |
| `menu.pointer_area = Some(rect)` | The pointer only counts inside `rect` |

## Interaction model

| Gesture | Response |
|---|---|
| Hold key/button, move mouse, release over a button | `Selected(idx)` |
| Hold key/button, move mouse to centre, release | `Dismissed` |
| Release in a fan-out on a fan button | `Selected(..)` of that button |
| Release in a fan-out on its opening button | `BreakOut` (default): nothing, the fan becomes a replace-style sub-pie (click or keyboard driven); `Dismiss`: `Dismissed` (`settings.fan_out_opener_action`) |
| Release in a fan-out over nothing (blocked sector, dead zone, outside the window) | `Dismissed` |
| Click in a fan-out on its opening button | `BreakOut` (default): the fan becomes a replace-style sub-pie; `Dismiss`: the fan closes (`None`) |
| Click in a fan-out on nothing else | The fan closes, the menu stays open (`None`) |
| Release with nothing selectable (every button disabled, no pointer) | `Dismissed` |
| Tap and release before crossing threshold (`OnMovement` only) | `QuickTap` |
| Two quick taps within `double_tap_window` (`OnMovement` only) | `DoubleTap` |
| Number key 1–9 or mnemonic key | `Selected(idx)` |
| Click on the centre | Goes back one level in a sub-pie; at the root it dismisses the menu (`Dismissed`; not for a pinned menu) |
| Escape or number key 5 (consumed by the menu) | `Dismissed` |

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
menu.settings.intercardinal_height = Some(0.5); // NE/SE/SW/NW halfway between the N/S and E/W rows; None = from shape_factor
menu.settings.screen_margin = 8.0;   // minimum gap from screen edge in px
menu.settings.dead_zone_radius = 12.0; // no button is highlighted closer to the centre than this

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

Settings structs, `PieButton` and `PieMenuResponse` derive `Clone`/`Debug` (the response is also `Copy + PartialEq`). Degenerate highlight settings (a `NaN` radius, zero-width strokes or arcs) draw nothing instead of panicking.


### Sub-pies

```rust
// Selecting a button with children opens them as a sub-pie.
let root = vec![PieButton::new(PieDirection::North).with_children(vec![/* ... */])];
```

`Selected(idx)` is the chosen button's index within its own level; `menu.selected_path()` is the full route to it from the root. In the render closure `b.path` is each button's route (own index last) and `b.button` the button itself, so you never need to look the level up.

There are two styles, which are different designs and not meant to be mixed lightly. Choose one for the menu; a single button can override it:

```rust
menu.settings.sub_pie_style = SubPieStyle::Replace;   // default; or SubPieStyle::FanOut
PieButton::new(dir).with_fan_out(children);            // this button fans out whatever the menu says
PieButton::new(dir).with_children(children).with_sub_pie_style(Some(SubPieStyle::Replace));
```

| `SubPieStyle` | What happens | Depth | Way back |
|---|---|---|---|
| `Replace` (default) | The sub-pie takes its parent's place: one level on screen at a time | any | Backspace, a click on the centre, or a back button |
| `FanOut` | The sub-pie opens around its button while the parent stays visible, greyed out | one (a fan's buttons cannot have sub-pies) | move the pointer back to the parent, or Backspace |

#### Fan-out

```rust
let dir = PieDirection::East;
let root = vec![PieButton::new(dir).with_label("More").with_fan_out(vec![
    PieButton::new(PieDirection::North).with_label("Above"),   // ordinary directions …
    PieButton::new(PieDirection::East).with_label("Beyond"),
    PieButton::new(PieDirection::South).with_label("Under"),
])];
dir.fan_out_slots();                                    // … but only these five: [North, NorthEast, East, SouthEast, South]

menu.settings.fan_out_delay = Duration::ZERO;           // how long the pointer rests on the button first (default: none)
menu.settings.fan_out_radius = None;                    // the fan's own radius; None = layout_radius
menu.settings.fan_out_opener_action = FanOpenerAction::BreakOut; // selecting the fan's own button: break out (default) or Dismiss
```

- A fan holds up to five buttons. The three directions facing the parent (`dir.opposite()` and its two neighbours) are blocked: `check_buttons` reports a child there as `BlockedDirection`, and a child with a sub-pie of its own as `NestedInFanOut`, both wrapped in `InFanOut { parent, problem }`.
- It opens when the pointer reaches its button (as far out as the ring of buttons) or when the button is selected by number key, mnemonic, click or release. It closes when the pointer comes back in towards the centre.
- While it is open it has the keyboard: number keys and mnemonics reach only the fan's buttons. The parent's centre and its other buttons are greyed out, and those buttons cannot be highlighted.
- The fan's highlighted button moves outward and its content grows slightly (see "Hover emphasis").
- The pointer picks among the fan's buttons by angle from the opening button's centre. On the opening button itself, or pointing back at the parent, nothing is highlighted.
- Selecting the fan's own opening button (releasing the key on it, or clicking it) is set by `settings.fan_out_opener_action`. `BreakOut` (default): the fan becomes a sub-pie of the same children, in place (Backspace or a centre click goes back). `Dismiss`: releasing on it dismisses the menu, and clicking it just closes the fan.
- The break-out transition: the sub-pie opens exactly where the fan was (same centre, radius and buttons, no jump and no re-reveal), the main ring, including the opening button, fades out where it was (still drawn through your closure while it fades, then gone), the old centre fades out and the new one fades in where it was (over `animations.reveal_duration`; instantly with `RevealAnimation::None`), and the children stay in place. A click on the centre or Backspace goes back and restores the main pie where it was.
- Releasing over nothing else in a fan dismisses the menu. A click on nothing else closes the fan (the menu stays open) and, like Backspace or a back button in the fan, it does not pop back open while the pointer rests on its button: move off the button and back to reopen it. The same holds after going back from a break-out.
- `menu.open_fan_out()` returns the opening button's index and the highlighted fan button, if a fan is open. A fan is not a level: `menu.path()` and `menu.current_buttons(&root)` still describe the parent.

#### Replace

A replace-style sub-pie can carry its own way back, and you choose where it opens and whether hovering opens it:

```rust
// A back button: selecting it returns to the parent level instead of `Selected`.
// Opposite the opening button it points back the way the sub-pie was entered.
let dir = PieDirection::NorthEast;
let root = vec![PieButton::new(dir).with_label("Paste Special").with_children(vec![
    PieButton::back(dir.opposite()),                        // labelled "Back"; or set `kind: PieButtonKind::Back` on any button
    PieButton::new(PieDirection::North).with_label("As Text"),
])];

menu.settings.sub_pie_center = SubPieCenter::Button;        // Pointer (default) · Parent · Button
menu.settings.sub_pie_hover_delay = Some(Duration::from_millis(400)); // follow sub-pie and back buttons on hover; None (default) = off
```

| `SubPieCenter` | The sub-pie opens | Going back |
|---|---|---|
| `Pointer` (default) | at the pointer (a pinned menu stays put) | re-centres the parent on the pointer |
| `Parent` | in place of its parent | stays there |
| `Button` | at the button that opened it | returns to the parent's own centre |

With `sub_pie_hover_delay`, resting the pointer on any button that leads to another pie follows it without a click, key or release: a sub-pie button opens its sub-pie, and a back button inside a sub-pie returns to the parent. Navigation flows: after a sub-pie opens, a sub-pie button under the resting pointer starts its own timer straight away, so you can rest on button after button without moving away in between. So that a resting pointer never bounces between levels, the button it points at after going back is ignored until the pointer has left it once, and so is a back button right after descending. Leaf buttons are never selected by hovering.

If the buttons change while a sub-pie is open so that its level no longer exists or has no buttons, the menu falls back to the root level. A pinned menu returns to its root level after a leaf selection.

The demo's "Paste Special" button opens a sub-pie that works in both styles; the "Sub-pies" section switches the menu's style, and the button editor can give any button a sub-pie (to any depth) and its own style.

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

To check in any build, or to show the problems in an editor, call `menu.check_buttons(&buttons)`: it returns a `Vec<ButtonProblem>` (`SharedDirection` / `DuplicateMnemonic` with the two button indices, plus the fan-out problems above, each with a readable `Display`; `p.buttons()` lists the indices involved). The demo uses it to mark the offending buttons red, open a warning popup, and keep the menu closed until the config is fixed.

### Right-to-left and non-Latin scripts

- `menu.settings.mirror_horizontal = true` flips the layout (East↔West, diagonals follow). Number keys and number hints follow the displayed positions, and `HintSide::Trailing` then means left of the label. Off by default.
- The mnemonic underline works in right-to-left words: egui (no bidi) lays separately formatted text pieces out left to right, so the library feeds the split word in reverse and, for Arabic-script labels, adds zero-width joiners at the cuts so letters keep their joined forms.
- In the render closure `b.rtl` is true while mirrored: lay out your own widgets in reverse (the demo puts a checkbox's box right of its text). `b.label_text().prefix(icon)` already puts the icon on the right.
- `number_hints.digits = HintDigits::ArabicIndic` shows ٠١٢…, `HintDigits::Persian` shows ۰۱۲… (Persian/Urdu), instead of 0123….
- Mnemonics work for any alphabetic script: they match typed characters, so they work with the active keyboard layout, and mnemonic generation and `mnemonic_text` are Unicode-aware (case folded; Hebrew final forms, Arabic alef variants, and Persian/Arabic yeh and kaf count as the same key; combining marks are skipped).
- **Text rendering is egui's job.** As of egui 0.36 text is shaped (Arabic letters join), but there is **no bidi** support, so mixed left-to-right/right-to-left labels display in the wrong order, and egui's default fonts do not include Arabic or Hebrew glyphs — add a font via `ctx.set_fonts` (the demo loads system fonts). Keep each label single-script until egui gains bidi.
- **Status: basic RTL support, further work tabled.** Mirroring, digits, mnemonic matching and underlining, icon and checkbox side are handled. Multi-word right-to-left labels still show their words in left-to-right order (egui has no bidi). Contributions welcome: the [`unicode-bidi`](https://crates.io/crates/unicode-bidi) crate could compute the visual word order so the library feeds egui the words already reordered (the same trick `append_mnemonic_text` uses for the mnemonic's word). Once egui moves to a bidi-aware text stack (Parley / cosmic-text), the workarounds should be removed instead; the test `rtl_mnemonics_render_like_the_unsplit_label` compares against egui's own layout and should flag that change.

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

## Running the demo

```sh
cargo run --example demo
```

  Right-click anywhere in the window to open the menu. The left panel exposes all settings as live drag fields and colour pickers.

## License

MIT
