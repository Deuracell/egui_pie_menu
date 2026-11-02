pub use egui::FontId;
pub use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, Key, Pos2, Rect, Stroke, Ui, Vec2};
use std::f32::consts::{PI, TAU};
use std::time::{Duration, Instant};

mod utils;
pub use utils::common_utils::SmartFloat;

pub mod settings;
pub use settings::*;

pub(crate) mod highlight_shapes;
use highlight_shapes::{ArcValues, CircleValues, HighlightPainter, SliceValues};

/// How a button gets its mnemonic key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mnemonic {
    /// Follow the menu (default): generated from the button's label when
    /// [`PieMenuInput::auto_mnemonics`] is on, otherwise none.
    #[default]
    Auto,
    /// This key (case-insensitive). Reserved: generated mnemonics never use it.
    Key(char),
    /// Never has a mnemonic, even when [`PieMenuInput::auto_mnemonics`] is on.
    None,
}

/// A single button slot in the pie menu.
///
/// Defines *where* the button sits (`direction`), its keyboard mnemonic and the
/// label text used to generate one. All visual content is rendered by the caller
/// via the closure passed to [`PieMenu::show`].
pub struct PieButton {
    /// Which of the eight directions this button occupies.
    pub direction: PieDirection,
    /// How this button gets its mnemonic key. Use [`PieMenu::mnemonics`] for the
    /// keys actually in effect (e.g. to underline them with [`mnemonic_text`]).
    pub mnemonic: Mnemonic,
    /// The text shown on the button, used to generate a mnemonic for
    /// [`Mnemonic::Auto`]. Rendering is still up to the closure.
    pub label: String,
}

impl PieButton {
    /// Creates a button at `direction` with an empty label and [`Mnemonic::Auto`].
    pub fn new(direction: PieDirection) -> Self {
        Self {
            direction,
            mnemonic: Mnemonic::Auto,
            label: String::new(),
        }
    }

    /// Sets the label text used to generate a mnemonic.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Sets an explicit mnemonic key for this button. Case-insensitive.
    pub fn with_mnemonic(mut self, c: char) -> Self {
        self.mnemonic = Mnemonic::Key(c);
        self
    }

    /// This button never gets a mnemonic, even when auto mnemonics are on.
    pub fn without_mnemonic(mut self) -> Self {
        self.mnemonic = Mnemonic::None;
        self
    }
}

/// Resolves the mnemonic key in effect for each button.
///
/// - [`Mnemonic::Key`] is used as given, and its key is reserved.
/// - [`Mnemonic::None`] never gets one.
/// - [`Mnemonic::Auto`] gets one generated from its label when `auto` is on. All
///   buttons are considered together: a button whose letters are all claimed makes
///   another button move to one of its alternative letters, so as many buttons as
///   possible get a distinct mnemonic. Word initials are preferred, then the other
///   letters left to right. Only a button with no possible letter is left without one.
///   Any alphabetic letter of any script can be generated (combining marks are skipped;
///   Hebrew final forms and Arabic alef variants are folded to one key).
fn resolve_mnemonics(buttons: &[PieButton], auto: bool) -> Vec<Option<char>> {
    let mut resolved: Vec<Option<char>> = buttons
        .iter()
        .map(|b| match b.mnemonic {
            Mnemonic::Key(c) => Some(fold_char(c)),
            Mnemonic::Auto | Mnemonic::None => None,
        })
        .collect();
    if !auto {
        return resolved;
    }
    let taken: Vec<char> = resolved.iter().flatten().copied().collect();

    // Candidate letters per Auto button, best first: word initials, then every other letter.
    let candidates: Vec<Vec<char>> = buttons
        .iter()
        .map(|button| {
            let mut list: Vec<char> = Vec::new();
            if button.mnemonic == Mnemonic::Auto {
                let label = &button.label;
                let letters = |c: &char| is_mnemonic_letter(*c);
                let initials = label
                    .split(|c: char| !c.is_alphanumeric())
                    .filter_map(|w| w.chars().next())
                    .filter(letters);
                for c in initials.chain(label.chars().filter(letters)) {
                    let c = fold_char(c);
                    if !taken.contains(&c) && !list.contains(&c) {
                        list.push(c);
                    }
                }
            }
            list
        })
        .collect();

    // Maximum bipartite matching (buttons <-> letters) via augmenting paths, so a
    // button whose only letters are claimed can make an earlier button move to
    // its alternative. Free letters are always preferred over displacing anyone.
    fn place(
        i: usize,
        candidates: &[Vec<char>],
        owner: &mut std::collections::HashMap<char, usize>,
        visited: &mut Vec<char>,
    ) -> bool {
        for &c in &candidates[i] {
            if !owner.contains_key(&c) {
                owner.insert(c, i);
                return true;
            }
        }
        for &c in &candidates[i] {
            if visited.contains(&c) {
                continue;
            }
            visited.push(c);
            let other = owner[&c];
            if place(other, candidates, owner, visited) {
                owner.insert(c, i);
                return true;
            }
        }
        false
    }

    // Most constrained buttons first, so they are settled before flexible ones.
    let mut order: Vec<usize> = (0..buttons.len()).filter(|&i| !candidates[i].is_empty()).collect();
    order.sort_by_key(|&i| candidates[i].len());
    let mut owner = std::collections::HashMap::new();
    for i in order {
        place(i, &candidates, &mut owner, &mut Vec::new());
    }
    for (c, i) in owner {
        resolved[i] = Some(c);
    }
    resolved
}

/// Outcome of a [`PieMenu::show`] call.
pub enum PieMenuResponse {
    /// A button was activated; value is its index in the slice passed to `show`.
    Selected(usize),
    /// The menu was dismissed without a selection.
    Dismissed,
    /// Key was tapped quickly without moving the mouse past the threshold.
    /// The caller decides what default action to perform.
    QuickTap,
    /// Two quick taps within [`PieMenuInput::double_tap_window`].
    /// The caller decides what alternate action to perform.
    DoubleTap,
    /// No action this frame.
    None,
}

/// What the menu knows about one button, handed to your render closure by
/// [`PieMenu::show`]. Everything is resolved for the level currently shown, so you never
/// have to precompute mnemonics or hints.
pub struct ButtonView<'a> {
    /// Index of the button within the level being shown (what `Selected(idx)` returns).
    pub idx: usize,
    /// Whether the pointer currently highlights this button.
    pub hovered: bool,
    /// The button itself (label, direction, …).
    pub button: &'a PieButton,
    /// The mnemonic key in effect for this button (lower-case), if any.
    pub mnemonic: Option<char>,
    /// A number-key hint to draw inside your own button. `None` when number keys are
    /// off, or when [`PieMenuNumberHints::show`] is on (the menu then paints the digit
    /// beside the button itself).
    pub number_hint: Option<NumberHint>,
    text_color: Color32,
    body_font: FontId,
}

impl ButtonView<'_> {
    /// The button's label with its mnemonic (if any) underlined, ready for
    /// `ui.label(..)` or `egui::Button::new(..)`. Colour and font follow the current
    /// egui style; override with [`LabelText::color`] / [`LabelText::font`].
    pub fn label_text(&self) -> LabelText {
        LabelText {
            prefix: String::new(),
            text: self.button.label.clone(),
            mnemonic: self.mnemonic,
            color: self.text_color,
            font: self.body_font.clone(),
        }
    }
}

/// A button label with its mnemonic underlined; see [`ButtonView::label_text`].
/// Converts into [`egui::WidgetText`], so pass it to `ui.label(..)` directly.
#[derive(Clone, Debug)]
pub struct LabelText {
    prefix: String,
    text: String,
    mnemonic: Option<char>,
    color: Color32,
    font: FontId,
}

impl LabelText {
    /// Text (and underline) colour.
    pub fn color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }

    /// Font.
    pub fn font(mut self, font: FontId) -> Self {
        self.font = font;
        self
    }

    /// Text placed before the label, never underlined (e.g. an icon: `"🗑  "`).
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    fn to_job(&self) -> LayoutJob {
        let format = TextFormat { color: self.color, font_id: self.font.clone(), ..Default::default() };
        let mut job = LayoutJob::default();
        if !self.prefix.is_empty() {
            job.append(&self.prefix, 0.0, format.clone());
        }
        match self.mnemonic {
            Some(m) => append_mnemonic_text(&mut job, &self.text, m, format),
            None => job.append(&self.text, 0.0, format),
        }
        job
    }
}

impl From<LabelText> for egui::WidgetText {
    fn from(l: LabelText) -> Self {
        l.to_job().into()
    }
}

/// Radial (pie) menu widget.
///
/// Owns interaction state but not button content. Construct once, then call
/// [`PieMenu::show`] every frame with the buttons for the current context.
/// A number-key hint to draw inside your own button; see [`ButtonView::number_hint`]
/// (or [`PieMenu::number_hint`] outside the render closure).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NumberHint {
    /// The digit, already converted to Arabic-Indic if that setting is on.
    pub text: String,
    /// Whether the digit goes after (right of) the label rather than before it,
    /// resolved from [`PieMenuNumberHints::side`].
    pub on_right: bool,
}

pub struct PieMenu {
    pub id: egui::Id,
    open_time: Instant,
    selected_index: Option<usize>,
    pub position: Pos2,
    pub settings: PieMenuSettings,
    release_handled: bool,
    /// Whether the show condition has been met since `open()` was called.
    mouse_shown: bool,
    /// Timestamp of the last QuickTap, used for double-tap detection.
    last_quick_tap: Option<Instant>,
    /// Set by `open()` when this press follows a QuickTap within the double-tap window.
    /// While set, the menu never becomes visible; releasing yields `DoubleTap`.
    double_tap_pending: bool,
    /// When the buttons became visible; drives the reveal animation.
    shown_at: Option<Instant>,
    /// Cached button sizes from the previous frame, used to centre each Area.
    button_sizes: Vec<Vec2>,
}

impl Default for PieMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl PieMenu {
    /// Creates a pie menu with default settings and a stable id.
    /// If you have more than one pie menu in the same app use [`PieMenu::with_id`].
    pub fn new() -> Self {
        Self {
            id: egui::Id::new("pie_menu"),
            open_time: Instant::now(),
            selected_index: None,
            position: Pos2::ZERO,
            settings: PieMenuSettings::default(),
            release_handled: false,
            mouse_shown: false,
            last_quick_tap: None,
            double_tap_pending: false,
            shown_at: None,
            button_sizes: Vec::new(),
        }
    }

    /// Override the egui [`egui::Id`] used for this menu's layer and Areas.
    /// Required when multiple pie menus coexist in the same app.
    pub fn with_id(mut self, id: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id = egui::Id::new(id);
        self
    }

    /// Override the default [`PieMenuSettings`] at construction time.
    pub fn with_settings(mut self, settings: PieMenuSettings) -> Self {
        self.settings = settings;
        self
    }

    /// The mnemonic key in effect for each button (lower-case), from each button's
    /// [`Mnemonic`] and [`PieMenuInput::auto_mnemonics`]. Use it in your render closure,
    /// for use outside the render closure. Inside it, prefer [`ButtonView::mnemonic`] and
    /// [`ButtonView::label_text`]. Cheap: a few microseconds for eight buttons.
    pub fn mnemonics(&self, buttons: &[PieButton]) -> Vec<Option<char>> {
        resolve_mnemonics(buttons, self.settings.input.auto_mnemonics)
    }

    /// The number-key hint for `button`, for drawing inside your own button so the button
    /// grows to fit it, e.g. `egui::Button::new((label, hint.text)).gap(spacing)`.
    ///
    /// Returns `None` when number keys are off. This is independent of
    /// [`PieMenuNumberHints::show`], which makes the menu paint the digit *beside* the
    /// button instead; use one or the other. Call it before [`PieMenu::show`] (which
    /// borrows the menu mutably) and move the results into the render closure.
    pub fn number_hint(&self, button: &PieButton) -> Option<NumberHint> {
        self.settings.input.use_numpad_keys.then(|| NumberHint {
            text: self.hint_digit(&Self::eff(&button.direction, self.settings.mirror_horizontal)).to_string(),
            on_right: self.hint_on_right(&Self::eff(&button.direction, self.settings.mirror_horizontal)),
        })
    }

    /// The hint digit for a direction, honouring `arabic_indic_digits`.
    fn hint_digit(&self, dir: &PieDirection) -> char {
        let digit = Self::direction_digit(dir);
        if self.settings.number_hints.arabic_indic_digits {
            char::from_u32('٠' as u32 + digit.to_digit(10).unwrap_or(0)).unwrap_or(digit)
        } else {
            digit
        }
    }

    /// Whether the hint goes on the right of the button, from [`PieMenuNumberHints::side`].
    fn hint_on_right(&self, dir: &PieDirection) -> bool {
        match self.settings.number_hints.side {
            // After the label in reading order: right for left-to-right, left when mirrored.
            HintSide::Trailing => !self.settings.mirror_horizontal,
            // Away from the centre; North/South have no horizontal side, so use the right.
            HintSide::Outside => !matches!(
                dir,
                PieDirection::West | PieDirection::NorthWest | PieDirection::SouthWest
            ),
            HintSide::Left => false,
            HintSide::Right => true,
        }
    }

    /// Call this when opening the menu. Resets interaction state and sets position.
    pub fn open(&mut self, pos: Pos2) {
        self.position = pos;
        self.open_time = Instant::now();
        self.release_handled = false;
        self.selected_index = None;
        self.mouse_shown = false;
        self.shown_at = None;
        self.double_tap_pending = self
            .last_quick_tap
            .map(|t| t.elapsed() <= self.settings.input.double_tap_window)
            .unwrap_or(false);
    }

    /// Converts a [`PieDirection`] to a [`Vec2`] offset, applying the shape factor.
    ///
    /// - `0.0`  → circle: all buttons equidistant (L2, unit vector)
    /// - `+1.0` → square: diagonals pushed out to the corners (L∞ norm)
    /// - `-1.0` → diamond: diagonals pulled in toward the centre (L1 norm)
    /// The direction as displayed: mirrored when [`PieMenuSettings::mirror_horizontal`] is on.
    fn eff(dir: &PieDirection, mirror: bool) -> PieDirection {
        if mirror { dir.mirrored() } else { dir.clone() }
    }

    fn direction_vec(dir: &PieDirection, shape_factor: f32) -> Vec2 {
        let fraction = match dir {
            PieDirection::North => 0.75,
            PieDirection::NorthEast => 0.875,
            PieDirection::East => 1.0,
            PieDirection::SouthEast => 0.125,
            PieDirection::South => 0.25,
            PieDirection::SouthWest => 0.375,
            PieDirection::West => 0.5,
            PieDirection::NorthWest => 0.625,
        };
        let angle = TAU * fraction;
        let circle = Vec2::new(angle.cos(), angle.sin());

        match dir {
            PieDirection::NorthEast
            | PieDirection::SouthEast
            | PieDirection::SouthWest
            | PieDirection::NorthWest => {
                if shape_factor > 0.0 {
                    let l_inf = circle / circle.x.abs().max(circle.y.abs());
                    circle * (1.0 - shape_factor) + l_inf * shape_factor
                } else if shape_factor < 0.0 {
                    let l1 = circle / (circle.x.abs() + circle.y.abs());
                    let t = -shape_factor;
                    circle * (1.0 - t) + l1 * t
                } else {
                    circle
                }
            }
            _ => circle,
        }
    }

    fn direction_digit(dir: &PieDirection) -> char {
        match dir {
            PieDirection::North => '8',
            PieDirection::NorthEast => '9',
            PieDirection::East => '6',
            PieDirection::SouthEast => '3',
            PieDirection::South => '2',
            PieDirection::SouthWest => '1',
            PieDirection::West => '4',
            PieDirection::NorthWest => '7',
        }
    }

    fn direction_numpad(dir: &PieDirection) -> Key {
        match dir {
            PieDirection::North => Key::Num8,
            PieDirection::NorthEast => Key::Num9,
            PieDirection::East => Key::Num6,
            PieDirection::SouthEast => Key::Num3,
            PieDirection::South => Key::Num2,
            PieDirection::SouthWest => Key::Num1,
            PieDirection::West => Key::Num4,
            PieDirection::NorthWest => Key::Num7,
        }
    }

    /// Show the pie menu for this frame.
    ///
    /// `render_button(ui, &button_view)` is called once per button inside a floating
    /// [`egui::Area`] centred at that button's position. Use it to draw any egui
    /// content — labels, images, custom widgets, etc. The [`ButtonView`] carries the
    /// index, hover state, resolved mnemonic and number hint; `ui.label(b.label_text())`
    /// draws the label with its mnemonic underlined.
    ///
    /// `key_down` should be `true` while the hotkey that opened the menu is held.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        buttons: &[PieButton],
        current_mouse_pos: Option<Pos2>,
        key_down: bool,
        title: Option<&str>,
        mut render_button: impl FnMut(&mut Ui, &ButtonView),
    ) -> PieMenuResponse {
        // Ensure cached size vec is long enough
        if self.button_sizes.len() < buttons.len() {
            self.button_sizes
                .resize(buttons.len(), Vec2::new(50.0, 24.0));
        }

        // Always restore selectable labels. If the menu is visible we disable them below,
        // but any early return (dismissed, QuickTap, DoubleTap, not yet shown) leaves this
        // restored so labels are selectable again the frame after the menu closes.
        ctx.all_styles_mut(|s| s.interaction.selectable_labels = true);

        let center = self.position;

        // Update mouse_shown: latch on once the show condition is met.
        // (shown_at is stamped below, once mouse_shown has latched)
        // A double-tap-and-hold must not reveal the menu (see `double_tap_pending`).
        if !self.mouse_shown && !self.double_tap_pending {
            match self.settings.show_behavior {
                ShowBehavior::Instant => self.mouse_shown = true,
                ShowBehavior::OnMovement { threshold } => {
                    if let Some(p) = current_mouse_pos {
                        if (p - center).length() > threshold {
                            self.mouse_shown = true;
                        }
                    }
                }
            }
        }

        if self.mouse_shown && self.shown_at.is_none() {
            self.shown_at = Some(Instant::now());
        }

        // Before the menu is visible, handle key release as QuickTap/DoubleTap only.
        if !self.mouse_shown {
            if !key_down && !self.release_handled {
                self.release_handled = true;
                if self.double_tap_pending {
                    self.double_tap_pending = false;
                    self.last_quick_tap = None;
                    return PieMenuResponse::DoubleTap;
                } else {
                    self.last_quick_tap = Some(Instant::now());
                    return PieMenuResponse::QuickTap;
                }
            }
            return PieMenuResponse::None;
        }

        // Early exit checks
        let dismissed = ctx.input(|i| {
            self.settings.input.dismiss_inputs.iter().any(|d| match d {
                PieMenuDismissInput::Key(k) => i.key_pressed(*k),
                PieMenuDismissInput::PointerButton(pb) => i.pointer.button_clicked(*pb),
            })
        });
        if dismissed {
            return PieMenuResponse::Dismissed;
        }

        let shape_factor = self.settings.shape_factor;
        let mirror = self.settings.mirror_horizontal;

        // Compute the bounding box of all buttons (relative to self.position) and
        // shift the whole menu so it stays within the screen rect with a margin.
        let screen_rect = ctx.content_rect();
        let margin = self.settings.screen_margin;
        let mut bb_min = Vec2::ZERO;
        let mut bb_max = Vec2::ZERO;
        for (idx, button) in buttons.iter().enumerate() {
            let dv = Self::direction_vec(&Self::eff(&button.direction, mirror), shape_factor);
            let dn = dv.normalized();
            let sz = self.button_sizes[idx];
            let hw = sz.x / 2.0;
            let hh = sz.y / 2.0;
            let sx = if dn.x > 0.5 {
                1.0
            } else if dn.x < -0.5 {
                -1.0
            } else {
                0.0
            };
            let sy = if dn.y > 0.5 {
                1.0
            } else if dn.y < -0.5 {
                -1.0
            } else {
                0.0
            };
            let offset = dv * self.settings.layout_radius + Vec2::new(hw * sx, hh * sy);
            bb_min = bb_min.min(offset - sz / 2.0);
            bb_max = bb_max.max(offset + sz / 2.0);
        }
        let center = Pos2::new(
            center.x.clamp(
                screen_rect.min.x + margin - bb_min.x,
                screen_rect.max.x - margin - bb_max.x,
            ),
            center.y.clamp(
                screen_rect.min.y + margin - bb_min.y,
                screen_rect.max.y - margin - bb_max.y,
            ),
        );

        // Block hover/interaction on background widgets while the menu is open.
        // Sits on Foreground (above panels, below Tooltip where buttons live) so
        // nothing underneath highlights as the mouse moves over it.
        egui::Area::new(self.id.with("_blocker"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::Pos2::ZERO)
            .show(ctx, |ui| {
                ui.allocate_rect(screen_rect, egui::Sense::click_and_drag());
            });

        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, self.id));

        // Center background circle
        if self
            .settings
            .center_indicator
            .background_radius
            .is_enabled()
        {
            painter.circle(
                center,
                self.settings.center_indicator.background_radius.get(),
                self.settings.center_indicator.background_fill_color,
                self.settings.center_indicator.background_stroke,
            );
        }

        // Numpad key selection
        if self.settings.input.use_numpad_keys {
            for (idx, button) in buttons.iter().enumerate() {
                if ctx.input(|i| i.key_pressed(Self::direction_numpad(&Self::eff(&button.direction, mirror)))) {
                    return PieMenuResponse::Selected(idx);
                }
            }
        }

        // Mnemonic key selection — consume matched events so other handlers never see them.
        if self.settings.input.use_mnemonic_keys {
            let mnemonics = self.mnemonics(buttons);
            #[cfg(debug_assertions)]
            {
                let mut seen = std::collections::HashMap::new();
                for (button, mnemonic) in buttons.iter().zip(&mnemonics) {
                    if let Some(c) = *mnemonic {
                        let key = fold_char(c);
                        let prev: &mut Vec<PieDirection> = seen.entry(key).or_default();
                        prev.push(button.direction.clone());
                        if prev.len() == 2 {
                            eprintln!("egui_pie_menu: duplicate mnemonic '{key}' on {:?}", prev);
                        }
                    }
                }
            }

            for (idx, mnemonic) in mnemonics.iter().enumerate() {
                let Some(m) = *mnemonic else { continue };
                let target = fold_char(m);
                let key = char_to_key(m);
                let pressed = ctx.input_mut(|i| {
                    let mut hit = false;
                    // Typed characters work for any script and keyboard layout.
                    i.events.retain(|e| match e {
                        egui::Event::Text(t)
                            if !hit
                                && t.chars().count() == 1
                                && fold_char(t.chars().next().unwrap()) == target =>
                        {
                            hit = true;
                            false
                        }
                        _ => true,
                    });
                    // Latin letters also match by key code.
                    if let Some(key) = key {
                        if i.key_pressed(key) {
                            i.consume_key(egui::Modifiers::NONE, key);
                            hit = true;
                        }
                    }
                    hit
                });
                if pressed {
                    return PieMenuResponse::Selected(idx);
                }
            }
        }

        // Update hover selection from mouse angle
        self.selected_index = current_mouse_pos.and_then(|mouse_pos| {
            let mouse_vec = mouse_pos - center;
            if mouse_vec.length() <= self.settings.mouse_trigger_threshold {
                return None;
            }
            let angle = mouse_vec.y.atan2(mouse_vec.x);
            buttons
                .iter()
                .enumerate()
                .map(|(idx, button)| {
                    let dir_vec = Self::direction_vec(&Self::eff(&button.direction, mirror), shape_factor);
                    let button_angle = dir_vec.y.atan2(dir_vec.x);
                    let diff = ((angle - button_angle + PI) % TAU - PI).abs();
                    (idx, diff)
                })
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .map(|(idx, _)| idx)
        });

        // Center highlight — progress driven by mouse distance from center.
        if self.settings.center_indicator.highlight_shape != PieMenuHighlightShape::None
            && self.settings.animations.center_highlight_show
        {
            let progress = current_mouse_pos
                .filter(|_| self.selected_index.is_some())
                .map(|p| {
                    let distance = (p - center).length();
                    let threshold = self.settings.mouse_trigger_threshold;
                    let max_dist = self.settings.layout_radius;
                    ((distance - threshold) / (max_dist - threshold)).clamp(0.0, 1.0)
                })
                .unwrap_or(0.0);

            let base_angle = if self.settings.animations.center_highlight_snapping {
                self.selected_index
                    .and_then(|idx| buttons.get(idx))
                    .map(|b| {
                        let v = Self::direction_vec(&Self::eff(&b.direction, mirror), shape_factor);
                        v.y.atan2(v.x)
                    })
                    .unwrap_or(0.0)
            } else {
                current_mouse_pos
                    .map(|p| {
                        let v = p - center;
                        v.y.atan2(v.x)
                    })
                    .unwrap_or(0.0)
            };

            let arc_angle = self.settings.center_indicator.highlight_angle * progress;
            let start_angle = base_angle - arc_angle / 2.0;
            let angle_range = start_angle..(start_angle + arc_angle);

            let stroke_color = self
                .settings
                .center_indicator
                .highlight_stroke
                .color
                .gamma_multiply(progress);
            let fill_color = self
                .settings
                .center_indicator
                .highlight_fill_color
                .gamma_multiply(progress);
            let colored_stroke = Stroke::new(
                self.settings.center_indicator.highlight_stroke.width,
                stroke_color,
            );
            let highlight_radius = self.settings.center_indicator.highlight_radius.get();
            let shape = self.settings.center_indicator.highlight_shape;

            let needs_arc = matches!(
                shape,
                PieMenuHighlightShape::Arc
                    | PieMenuHighlightShape::ArcSlice
                    | PieMenuHighlightShape::ArcCircle
                    | PieMenuHighlightShape::ArcSliceCircle
            );
            let needs_slice = matches!(
                shape,
                PieMenuHighlightShape::Slice
                    | PieMenuHighlightShape::SliceCircle
                    | PieMenuHighlightShape::ArcSlice
                    | PieMenuHighlightShape::ArcSliceCircle
            );
            let needs_circle = matches!(
                shape,
                PieMenuHighlightShape::Circle
                    | PieMenuHighlightShape::ArcCircle
                    | PieMenuHighlightShape::SliceCircle
                    | PieMenuHighlightShape::ArcSliceCircle
            );

            let arc_arg = needs_arc.then(|| ArcValues {
                angle_range: angle_range.clone(),
                center,
                radius: highlight_radius,
                resolution: 10.0,
                stroke: colored_stroke,
            });
            let slice_arg = needs_slice.then(|| {
                let inner_arc = matches!(
                    shape,
                    PieMenuHighlightShape::Slice | PieMenuHighlightShape::SliceCircle
                )
                .then(|| ArcValues {
                    angle_range: angle_range.clone(),
                    center,
                    radius: highlight_radius,
                    resolution: 10.0,
                    stroke: colored_stroke,
                });
                SliceValues {
                    arc_values: inner_arc,
                    stroke: None,
                    fill_color,
                }
            });
            let circle_arg = needs_circle.then(|| CircleValues {
                offset_angle: base_angle,
                offset_radius: highlight_radius,
                offset_center: center,
                circle_radius: self.settings.center_indicator.highlight_circle_radius,
                stroke: colored_stroke,
                fill_color,
            });

            painter.highlight_shape(shape, arc_arg, slice_arg, circle_arg);
        }

        // Draw menu title above the center indicator
        if self.settings.label.display {
            if let Some(label) = title {
                let pad = &self.settings.label.padding;
                let galley = painter.layout_no_wrap(
                    label.to_string(),
                    self.settings.label.text_font.clone(),
                    self.settings.label.text_color,
                );
                let text_size = galley.size();
                let bg_w = text_size.x + pad.left + pad.right;
                let bg_h = text_size.y + pad.top + pad.bottom;
                let above_offset =
                    self.settings.center_indicator.background_radius.get() + bg_h / 2.0 + 4.0;
                let label_center = center - Vec2::new(0.0, above_offset);
                let bg_rect = Rect::from_center_size(label_center, Vec2::new(bg_w, bg_h));

                if self.settings.label.background_color != Color32::TRANSPARENT {
                    painter.rect_filled(bg_rect, 3.0, self.settings.label.background_color);
                }
                if self.settings.label.background_stroke.width > 0.0
                    && self.settings.label.background_stroke.color != Color32::TRANSPARENT
                {
                    painter.rect_stroke(
                        bg_rect,
                        3.0,
                        self.settings.label.background_stroke,
                        egui::StrokeKind::Outside,
                    );
                }
                painter.galley(
                    bg_rect.min + Vec2::new(pad.left, pad.top),
                    galley,
                    self.settings.label.text_color,
                );
            }
        }

        // Draw buttons via caller-provided closure inside floating Areas
        // Resolved once per frame for the view handed to the render closure.
        let mnemonics = self.mnemonics(buttons);
        let number_hints: Vec<Option<NumberHint>> = buttons
            .iter()
            .map(|b| if self.settings.number_hints.show { None } else { self.number_hint(b) })
            .collect();
        let (text_color, body_font) = {
            let style = ctx.global_style();
            (
                style.visuals.text_color(),
                style.text_styles.get(&egui::TextStyle::Body).cloned().unwrap_or_default(),
            )
        };

        for (idx, button) in buttons.iter().enumerate() {
            let dir_vec = Self::direction_vec(&Self::eff(&button.direction, mirror), shape_factor);
            let dir_norm = dir_vec.normalized();

            // Push the button outward so its inner face/corner sits on the layout circle.
            // Each axis is offset independently: X by ±hw, Y by ±hh, based on the sign of
            // the direction. This keeps NE/NW at the same Y and SE/SW at the same Y even
            // when button widths differ, and the inner corner still lands exactly on the circle.
            let cached_size = self.button_sizes[idx];
            let hw = cached_size.x / 2.0;
            let hh = cached_size.y / 2.0;
            // Use 0.5 as threshold: diagonal components are ~0.707, cardinal near-zero
            // components are floating-point noise (~1e-17) and must not contribute.
            let sx = if dir_norm.x > 0.5 {
                1.0
            } else if dir_norm.x < -0.5 {
                -1.0
            } else {
                0.0
            };
            let sy = if dir_norm.y > 0.5 {
                1.0
            } else if dir_norm.y < -0.5 {
                -1.0
            } else {
                0.0
            };

            let button_center =
                center + dir_vec * self.settings.layout_radius + Vec2::new(hw * sx, hh * sy);
            let is_hovered = self.selected_index == Some(idx);

            // Reveal animation: movement and opacity progress, each 0..1.
            let anim = &self.settings.animations;
            let elapsed = self.shown_at.map(|at| at.elapsed().as_secs_f32());
            let progress = |d: Duration| {
                elapsed.map_or(1.0, |e| (e / d.as_secs_f32().max(f32::EPSILON)).clamp(0.0, 1.0))
            };
            let (move_t, opacity) = match anim.reveal {
                RevealAnimation::None => (1.0, 1.0),
                RevealAnimation::Fade => (1.0, progress(anim.reveal_duration)),
                RevealAnimation::SlideOut => (
                    progress(anim.reveal_duration),
                    anim.slide_fade.map_or(1.0, progress),
                ),
            };
            if move_t < 1.0 || opacity < 1.0 {
                ctx.request_repaint();
            }
            let button_center = if move_t < 1.0 {
                let eased = 1.0 - (1.0 - move_t).powi(3);
                // Interpolate the radius and (optionally) the shape factor, then lay the
                // button out as if that were the final layout, scaled about the centre.
                // With a start radius of 0 this collapses every button onto the centre.
                let end_radius = self.settings.layout_radius;
                let radius = anim.slide_start_radius + (end_radius - anim.slide_start_radius) * eased;
                let start_sf = anim.slide_start_shape_factor.unwrap_or(shape_factor);
                let sf = start_sf + (shape_factor - start_sf) * eased;
                let scale = if end_radius > 0.0 { radius / end_radius } else { 1.0 };
                let offset = Self::direction_vec(&Self::eff(&button.direction, mirror), sf) * end_radius
                    + Vec2::new(hw * sx, hh * sy);
                center + offset * scale
            } else {
                button_center
            };
            let area_pos = button_center - cached_size / 2.0;

            if self.settings.number_hints.show && self.settings.input.use_numpad_keys {
                let hints = &self.settings.number_hints;
                let (x, align) = if self.hint_on_right(&Self::eff(&button.direction, mirror)) {
                    (button_center.x + hw + hints.spacing, egui::Align2::LEFT_CENTER)
                } else {
                    (button_center.x - hw - hints.spacing, egui::Align2::RIGHT_CENTER)
                };
                let color = hints
                    .color
                    .unwrap_or_else(|| ctx.global_style().visuals.weak_text_color());
                let digit = self.hint_digit(&Self::eff(&button.direction, mirror));
                painter.text(
                    Pos2::new(x, button_center.y),
                    align,
                    digit,
                    hints.font.clone(),
                    color.gamma_multiply(opacity),
                );
            }

            let response = egui::Area::new(self.id.with(idx))
                .order(egui::Order::Tooltip)
                // egui's own area fade is always off: Fade replaces it, None means none.
                .fade_in(false)
                .fixed_pos(area_pos)
                .show(ctx, |ui| {
                    if opacity < 1.0 {
                        ui.set_opacity(opacity);
                    }
                    // Buttons are single-line. An Area keeps last frame's size as its max
                    // width, so content that grows (a number hint added, a label changed)
                    // would wrap instead of widening the button. Don't wrap; let it grow.
                    // Explicit newlines still work, and the closure can override this.
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    render_button(ui, &ButtonView {
                        idx,
                        hovered: is_hovered,
                        button,
                        mnemonic: mnemonics[idx],
                        number_hint: number_hints[idx].clone(),
                        text_color,
                        body_font: body_font.clone(),
                    })
                });

            self.button_sizes[idx] = response.response.rect.size();
        }

        // Primary click selection — runs after render so closures (e.g. checkboxes)
        // receive the click event before we consume it.
        if self.settings.input.select_on_primary_click && ctx.input(|i| i.pointer.primary_clicked())
        {
            return match current_mouse_pos {
                Some(p) if (p - center).length() > self.settings.mouse_trigger_threshold => self
                    .selected_index
                    .map_or(PieMenuResponse::None, PieMenuResponse::Selected),
                Some(_) => PieMenuResponse::None,
                None => PieMenuResponse::Dismissed,
            };
        }

        // Key-hold release (mouse_shown is true here, so menu was visible)
        if !key_down && !self.release_handled {
            self.release_handled = true;
            if let Some(mouse_pos) = current_mouse_pos {
                if (mouse_pos - center).length() <= self.settings.mouse_trigger_threshold {
                    return PieMenuResponse::Dismissed;
                } else if let Some(idx) = self.selected_index {
                    return PieMenuResponse::Selected(idx);
                }
            }
        }

        // Disable label text selection for the next frame. Placed after all rendering
        // so buttons in this frame see the restored style (set at the top of show()).
        // Restored at the top of show() on the closing frame.
        ctx.all_styles_mut(|s| s.interaction.selectable_labels = false);

        PieMenuResponse::None
    }
}

/// Build a [`LayoutJob`] that renders `text` with the first case-insensitive
/// occurrence of `mnemonic` underlined — suitable for use as a pie menu button
/// label when you want to surface keyboard shortcuts visually.
///
/// The rest of the text uses `format` unchanged. The underline color is
/// derived from `format.color`.  If `mnemonic` does not appear in `text`
/// the whole string is rendered with `format` and no underline.
///
/// # Example
/// ```ignore
/// ui.label(mnemonic_text("Copy", 'c', TextFormat {
///     color: Color32::WHITE,
///     font_id: FontId::default(),
///     ..Default::default()
/// }));
/// ```
pub fn mnemonic_text(text: &str, mnemonic: char, format: TextFormat) -> LayoutJob {
    let mut job = LayoutJob::default();
    append_mnemonic_text(&mut job, text, mnemonic, format);
    job
}

fn append_mnemonic_text(job: &mut LayoutJob, text: &str, mnemonic: char, format: TextFormat) {
    let mnemonic_folded = fold_char(mnemonic);
    let split = text
        .char_indices()
        .find(|(_, c)| fold_char(*c) == mnemonic_folded);

    match split {
        None => {
            job.append(text, 0.0, format);
        }
        Some((byte_idx, c)) => {
            let before = &text[..byte_idx];
            let after = &text[byte_idx + c.len_utf8()..];
            let ch_str = &text[byte_idx..byte_idx + c.len_utf8()];

            let underline_format = TextFormat {
                underline: Stroke::new(1.0, format.color),
                ..format.clone()
            };

            if !before.is_empty() {
                job.append(before, 0.0, format.clone());
            }
            job.append(ch_str, 0.0, underline_format);
            if !after.is_empty() {
                job.append(after, 0.0, format);
            }
        }
    }
}

/// Case-folds `c` and normalises letter variants that share a key:
/// Hebrew final forms (ך→כ …) and Arabic alef variants (أ إ آ ٱ → ا).
fn fold_char(c: char) -> char {
    let c = c.to_lowercase().next().unwrap_or(c);
    match c {
        'ך' => 'כ',
        'ם' => 'מ',
        'ן' => 'נ',
        'ף' => 'פ',
        'ץ' => 'צ',
        'أ' | 'إ' | 'آ' | 'ٱ' => 'ا',
        _ => c,
    }
}

/// Combining marks (Arabic/Hebrew points, accents) are never mnemonic candidates.
fn is_combining_mark(c: char) -> bool {
    matches!(c as u32,
        0x0300..=0x036F | 0x0591..=0x05BD | 0x05BF | 0x05C1..=0x05C2 | 0x05C4..=0x05C5 | 0x05C7
        | 0x0610..=0x061A | 0x064B..=0x065F | 0x0670 | 0x06D6..=0x06DC | 0x06DF..=0x06E4
        | 0x06E7..=0x06E8 | 0x06EA..=0x06ED)
}

fn is_mnemonic_letter(c: char) -> bool {
    c.is_alphabetic() && !is_combining_mark(c)
}

fn char_to_key(c: char) -> Option<Key> {
    match c.to_ascii_lowercase() {
        'a' => Some(Key::A),
        'b' => Some(Key::B),
        'c' => Some(Key::C),
        'd' => Some(Key::D),
        'e' => Some(Key::E),
        'f' => Some(Key::F),
        'g' => Some(Key::G),
        'h' => Some(Key::H),
        'i' => Some(Key::I),
        'j' => Some(Key::J),
        'k' => Some(Key::K),
        'l' => Some(Key::L),
        'm' => Some(Key::M),
        'n' => Some(Key::N),
        'o' => Some(Key::O),
        'p' => Some(Key::P),
        'q' => Some(Key::Q),
        'r' => Some(Key::R),
        's' => Some(Key::S),
        't' => Some(Key::T),
        'u' => Some(Key::U),
        'v' => Some(Key::V),
        'w' => Some(Key::W),
        'x' => Some(Key::X),
        'y' => Some(Key::Y),
        'z' => Some(Key::Z),
        _ => None,
    }
}

#[cfg(test)]
mod mnemonic_tests {
    use super::*;

    fn btn(label: &str) -> PieButton {
        PieButton::new(PieDirection::North).with_label(label)
    }

    #[test]
    fn auto_off_only_explicit_keys() {
        let b = vec![btn("Copy"), btn("Cut").with_mnemonic('C'), btn("Paste").without_mnemonic()];
        assert_eq!(resolve_mnemonics(&b, false), vec![None, Some('c'), None]);
    }

    #[test]
    fn explicit_kept_and_not_duplicated() {
        let b = vec![btn("Copy"), btn("Cut").with_mnemonic('c'), btn("Paste")];
        // 'c' is reserved by the explicit key, so Copy takes its next letter.
        assert_eq!(resolve_mnemonics(&b, true), vec![Some('o'), Some('c'), Some('p')]);
    }

    #[test]
    fn none_never_gets_one_and_reserves_nothing() {
        let b = vec![btn("Copy").without_mnemonic(), btn("Cut")];
        assert_eq!(resolve_mnemonics(&b, true), vec![None, Some('c')]);
    }

    #[test]
    fn reshuffles_so_everyone_gets_one() {
        // Greedy would give "Ab" 'a' and leave "A" with nothing.
        assert_eq!(resolve_mnemonics(&[btn("Ab"), btn("A")], true), vec![Some('b'), Some('a')]);
    }

    #[test]
    fn preference_order_initials_then_remaining_letters_left_to_right() {
        let pick = |reserved: &[char]| {
            let mut b: Vec<PieButton> = reserved.iter().map(|&c| btn("").with_mnemonic(c)).collect();
            b.push(btn("Select All Items"));
            *resolve_mnemonics(&b, true).last().unwrap()
        };
        assert_eq!(pick(&[]), Some('s')); // 1st word initial
        assert_eq!(pick(&['s']), Some('a')); // 2nd word initial
        assert_eq!(pick(&['s', 'a']), Some('i')); // 3rd word initial
        assert_eq!(pick(&['s', 'a', 'i']), Some('e')); // then letters in order: s,e,l,...
        assert_eq!(pick(&['s', 'a', 'i', 'e']), Some('l'));
    }

    #[test]
    fn exhausted_gets_none() {
        assert_eq!(resolve_mnemonics(&[btn("A"), btn("A")], true)[1], None);
    }

    #[test]
    fn all_auto_demo_labels_are_distinct() {
        let labels = ["Copy", "Paste Special", "Redo", "Save As…", "Delete Permanently", "Cut", "Undo Last Change", "Word wrap"];
        let b: Vec<PieButton> = labels.iter().map(|l| btn(l)).collect();
        let r = resolve_mnemonics(&b, true);
        let mut got: Vec<char> = r.iter().flatten().copied().collect();
        assert_eq!(got.len(), labels.len(), "everyone gets one: {r:?}");
        got.sort();
        got.dedup();
        assert_eq!(got.len(), labels.len(), "no duplicates: {r:?}");
    }

    #[test]
    fn empty_label_gets_none() {
        assert_eq!(resolve_mnemonics(&[btn("")], true), vec![None]);
    }
}

#[cfg(test)]
mod button_view_tests {
    use super::*;

    fn label(prefix: &str, mnemonic: Option<char>) -> LabelText {
        LabelText {
            prefix: prefix.to_string(),
            text: "Copy".to_string(),
            mnemonic,
            color: Color32::WHITE,
            font: FontId::default(),
        }
    }

    #[test]
    fn plain_label_is_one_section() {
        assert_eq!(label("", None).to_job().sections.len(), 1);
    }

    #[test]
    fn mnemonic_is_underlined_and_prefix_is_not() {
        let job = label("X ", Some('p')).to_job();
        assert_eq!(job.text, "X Copy");
        // egui merges equal adjacent formats: "X Co" | "p" (underlined) | "y"
        assert_eq!(job.sections.len(), 3);
        assert!(job.sections[1].format.underline != Stroke::NONE);
        let underlined: Vec<_> = job.sections.iter().filter(|s| s.format.underline != Stroke::NONE).collect();
        assert_eq!(underlined.len(), 1);
    }

    #[test]
    fn overrides_apply() {
        let l = label("", None).color(Color32::RED).font(FontId::monospace(20.0));
        let job = l.to_job();
        assert_eq!(job.sections[0].format.color, Color32::RED);
        assert_eq!(job.sections[0].format.font_id, FontId::monospace(20.0));
    }
}

#[cfg(test)]
mod unicode_tests {
    use super::*;

    fn one(label: &str, explicit: &[char]) -> Option<char> {
        let mut b: Vec<PieButton> = explicit
            .iter()
            .map(|&c| PieButton::new(PieDirection::East).with_mnemonic(c))
            .collect();
        b.push(PieButton::new(PieDirection::North).with_label(label));
        resolve_mnemonics(&b, true).last().copied().flatten()
    }

    #[test]
    fn other_scripts_get_mnemonics() {
        assert_eq!(one("Копировать", &[]), Some('к'));
        assert_eq!(one("نسخ ملف", &[]), Some('ن'));
        assert_eq!(one("نسخ ملف", &['ن']), Some('م')); // next word initial
        assert_eq!(one("העתק", &[]), Some('ה'));
        assert_eq!(one("Ärger", &[]), Some('ä'));
    }

    #[test]
    fn skips_combining_marks_and_folds_variants() {
        assert_eq!(one("\u{064E}بيت", &[]), Some('ب')); // fatha is skipped
        assert_eq!(fold_char('ך'), fold_char('כ'));
        assert_eq!(fold_char('أ'), fold_char('ا'));
        assert_eq!(fold_char('É'), 'é');
    }

    #[test]
    fn mnemonic_text_unicode_underline() {
        let job = mnemonic_text("Файл", 'Ф', TextFormat::default());
        // Mnemonic is the first char: underlined "Ф" + remainder (no "before" section).
        assert_eq!(job.sections.len(), 2);
    }

    #[test]
    fn mirror_swaps_horizontal_only() {
        assert_eq!(PieDirection::East.mirrored(), PieDirection::West);
        assert_eq!(PieDirection::NorthWest.mirrored(), PieDirection::NorthEast);
        assert_eq!(PieDirection::South.mirrored(), PieDirection::South);
    }

    #[test]
    fn hint_follows_mirrored_layout() {
        let mut m = PieMenu::new();
        let east = PieButton::new(PieDirection::East);
        assert_eq!(m.number_hint(&east).unwrap().text, "6");
        m.settings.mirror_horizontal = true;
        assert_eq!(m.number_hint(&east).unwrap().text, "4"); // East is shown on the west side
        assert!(!m.number_hint(&east).unwrap().on_right); // Trailing = left when mirrored
    }
}
