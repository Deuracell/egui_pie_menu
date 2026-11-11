//! A radial (pie) menu widget for [egui](https://github.com/emilk/egui), inspired by
//! Blender's pie menus.
//!
//! Keep a [`PieMenu`] in your app state, [`PieMenu::open`] it on a trigger, and call
//! [`PieMenu::show`] every frame. The menu owns its open state: `show` does nothing while
//! it is closed and closes it again on any response other than [`PieMenuResponse::None`].
//!
//! ```no_run
//! use egui_pie_menu::{PieButton, PieDirection, PieMenu, PieMenuResponse};
//!
//! # fn frame(ctx: &egui::Context, menu: &mut PieMenu) {
//! let buttons = [
//!     PieButton::new(PieDirection::North).with_label("Copy"),
//!     PieButton::new(PieDirection::South).with_label("Paste"),
//! ];
//! // Open on right-click, at the pointer.
//! if ctx.input(|i| i.pointer.secondary_pressed())
//!     && let Some(pos) = ctx.input(|i| i.pointer.latest_pos())
//! {
//!     menu.open(pos);
//! }
//! let key_down = ctx.input(|i| i.pointer.secondary_down());
//! match menu.show(ctx, &buttons, key_down, |ui, b| {
//!     ui.label(b.label_text());
//! }) {
//!     PieMenuResponse::Selected(idx) => println!("selected {}", buttons[idx].label),
//!     PieMenuResponse::QuickTap => println!("tapped without opening"),
//!     _ => {}
//! }
//! # }
//! ```

pub use egui::FontId;
pub use egui::text::{LayoutJob, TextFormat};
use egui::emath::TSTransform;
use egui::{Color32, Key, Pos2, Rect, Stroke, Ui, Vec2};
use std::time::Duration;

pub mod settings;
pub use settings::*;

mod button;
mod highlight_shapes;
mod layout;
mod mnemonic;
#[cfg(test)]
mod tests;

pub use button::{ButtonProblem, ButtonView, LabelText, NumberHint, PieButton, PieButtonKind};
pub use mnemonic::{Mnemonic, mnemonic_text};

use button::{button_problems, button_text_color, fan_out_problems};
use mnemonic::{char_to_key, fold_char, resolve_mnemonics};

/// Outcome of a [`PieMenu::show`] call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// What one frame of a level produced, before sub-pies, fans and back buttons are resolved.
enum LevelEvent {
    None,
    Dismissed,
    QuickTap,
    DoubleTap,
    /// A button of the level being shown.
    Pick(usize),
    /// A child of the open fan-out: (the fan's button, the child).
    FanPick(usize, usize),
}

/// What [`PieMenu::draw_level`] needs to draw one ring of buttons.
struct LevelDraw<'a> {
    buttons: &'a [PieButton],
    center: Pos2,
    /// Layout radius of this ring.
    radius: f32,
    /// When the ring appeared, for the reveal animation.
    shown_at: Option<f64>,
    hovered: Option<usize>,
    /// The parent ring while a fan is open: every button but this one is greyed out …
    dim_except: Option<usize>,
    /// … to this opacity (eases to and from `DIMMED_OPACITY` as the fan opens and closes).
    dim_opacity: f32,
    /// How the highlighted button is emphasised.
    emphasis: HoverEmphasis,
    /// Salt for the buttons' Area ids.
    id: egui::Id,
    /// Route from the root to this ring (each button's `path` is this plus its index).
    path_prefix: &'a [usize],
}

/// An open fan-out, laid out for this frame.
struct Fan<'a> {
    /// Index of the button that opened it, in the level being shown.
    trigger: usize,
    children: &'a [PieButton],
    center: Pos2,
    radius: f32,
    /// The opening button's rect: the pointer on it selects nothing in the fan.
    trigger_rect: Rect,
}

/// Opacity of the parent (its centre and its other buttons) while a fan-out is open.
const DIMMED_OPACITY: f32 = 0.35;

/// Radial (pie) menu widget.
///
/// Owns interaction state but not button content. Construct once, then call
/// [`PieMenu::show`] every frame with the buttons for the current context.
pub struct PieMenu {
    pub id: egui::Id,
    /// Text shown above the center indicator (see [`PieMenuSettings::title`] for its
    /// appearance). `None`: no title.
    pub title: Option<String>,
    /// The pointer only counts inside this rect; outside it the menu behaves as if there
    /// were no pointer (nothing hovered, clicks select nothing). Mainly for a pinned menu
    /// that shares the window with other widgets. `None` (default): everywhere.
    pub pointer_area: Option<Rect>,
    /// Between [`PieMenu::open`] and a closing response.
    is_open: bool,
    /// Opened with [`PieMenu::pin`]: stays open after a selection.
    pinned: bool,
    hovered_index: Option<usize>,
    pub position: Pos2,
    pub settings: PieMenuSettings,
    release_handled: bool,
    /// Whether the show condition has been met since `open()` was called.
    visible: bool,
    /// egui time (`InputState::time`) of the last QuickTap, used for double-tap detection.
    /// egui's clock is used rather than `std::time::Instant`, which panics on wasm.
    last_quick_tap: Option<f64>,
    /// Set by `open()`: the first `show` after it decides `double_tap_pending`.
    fresh_open: bool,
    /// This press follows a QuickTap within the double-tap window.
    /// While set, the menu never becomes visible; releasing yields `DoubleTap`.
    double_tap_pending: bool,
    /// Debug-only: the duplicate direction/mnemonic check already ran for this open.
    checked_buttons: bool,
    /// egui time when the buttons became visible; drives the reveal animation.
    /// `None` while visible means "stamp it on the next `show`".
    shown_at: Option<f64>,
    /// Indices of the buttons descended through to reach the current sub-pie level.
    path: Vec<usize>,
    /// Full index path of the last leaf selection (see [`PieMenu::selected_path`]).
    selected_path: Vec<usize>,
    /// Cached button sizes from the previous frame, used to centre each Area.
    button_sizes: Vec<Vec2>,
    /// Where each button of the level was drawn last frame (its final, un-animated
    /// centre), for [`SubPieCenter::Button`].
    button_centers: Vec<Pos2>,
    /// The centre of each level descended through (parallel to `path`), restored on
    /// going back.
    level_centers: Vec<Pos2>,
    /// Sub-pie hover timer: the hovered sub-pie button and the egui time it became so.
    hover_since: Option<(usize, f64)>,
    /// The open fan-out: index of its button in the level being shown.
    fan: Option<usize>,
    /// The button of the fan that was open last, while the parent's dimming eases out.
    last_fan: Option<usize>,
    /// egui time the fan appeared (`None`: stamp on the next draw), for its animation.
    fan_shown_at: Option<f64>,
    /// Cached sizes of the fan's buttons from the previous frame.
    fan_sizes: Vec<Vec2>,
    /// Test-only: the fan buttons' final centres from the previous frame.
    #[cfg(test)]
    fan_centers: Vec<Pos2>,
    /// The fan child the pointer highlights.
    fan_hover: Option<usize>,
    /// The pointer has been out at the fan since it opened; coming back in then closes it.
    fan_armed: bool,
    /// Fan-out hover timer: the hovered fan button and the egui time it became so.
    fan_pending: Option<(usize, f64)>,
    /// The current level has not been shown yet since it opened.
    level_fresh: bool,
    /// The button the pointer already pointed at when the current level opened. The
    /// hover timer ignores it until the pointer has left it once, so arriving in a level
    /// never triggers the next navigation by itself.
    inert_hover: Option<usize>,
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
            title: None,
            pointer_area: None,
            is_open: false,
            pinned: false,
            hovered_index: None,
            position: Pos2::ZERO,
            settings: PieMenuSettings::default(),
            release_handled: false,
            visible: false,
            last_quick_tap: None,
            fresh_open: false,
            double_tap_pending: false,
            checked_buttons: false,
            shown_at: None,
            path: Vec::new(),
            selected_path: Vec::new(),
            button_sizes: Vec::new(),
            button_centers: Vec::new(),
            level_centers: Vec::new(),
            hover_since: None,
            fan: None,
            last_fan: None,
            fan_shown_at: None,
            fan_sizes: Vec::new(),
            #[cfg(test)]
            fan_centers: Vec::new(),
            fan_hover: None,
            fan_armed: false,
            fan_pending: None,
            level_fresh: true,
            inert_hover: None,
        }
    }

    /// Override the egui [`egui::Id`] used for this menu's layer and Areas.
    /// Required when multiple pie menus coexist in the same app.
    pub fn with_id(mut self, id: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id = egui::Id::new(id);
        self
    }

    /// Sets the title shown above the center indicator.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Override the default [`PieMenuSettings`] at construction time.
    pub fn with_settings(mut self, settings: PieMenuSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Problems in one level's buttons: several in one direction, or the same mnemonic
    /// on several enabled buttons; and, for each button that opens a fan-out, the same
    /// among its children plus children in a blocked direction or with sub-pies of their
    /// own ([`ButtonProblem::InFanOut`]). Debug builds assert on these when the level
    /// opens; call this to check (or show the problems) yourself, e.g. in an editor, in
    /// any build. Pass the buttons of a single level.
    pub fn check_buttons(&self, buttons: &[PieButton]) -> Vec<ButtonProblem> {
        let mut problems = button_problems(buttons, &self.mnemonics(buttons));
        for (parent, button) in buttons.iter().enumerate().filter(|(_, b)| self.opens_fan(b)) {
            let children = button.children();
            for problem in fan_out_problems(button.direction, children, &self.mnemonics(children)) {
                problems.push(ButtonProblem::InFanOut { parent, problem: Box::new(problem) });
            }
        }
        problems
    }

    /// Whether `button` opens its sub-pie as a fan-out: it has children, and its own
    /// style (or else [`PieMenuSettings::sub_pie_style`]) is [`SubPieStyle::FanOut`].
    pub fn opens_fan(&self, button: &PieButton) -> bool {
        match &button.kind {
            PieButtonKind::SubPie { children, style } => {
                !children.is_empty() && style.unwrap_or(self.settings.sub_pie_style) == SubPieStyle::FanOut
            }
            _ => false,
        }
    }

    /// The mnemonic key in effect for each button (lower-case), from each button's
    /// [`Mnemonic`] and [`PieMenuInput::auto_mnemonics`], for use outside the render closure. Inside it, prefer [`ButtonView::mnemonic`] and
    /// [`ButtonView::label_text`]. Cheap: a few microseconds for eight buttons.
    pub fn mnemonics(&self, buttons: &[PieButton]) -> Vec<Option<char>> {
        resolve_mnemonics(buttons, self.settings.input.auto_mnemonics)
    }

    /// The number-key hint for `button`, for drawing inside your own button so the button
    /// grows to fit it, e.g. `egui::Button::new((label, hint.text)).gap(spacing)`.
    ///
    /// Returns `None` when number keys are off. Inside the render closure, prefer
    /// [`ButtonView::number_hint`], which also knows whether the button is hovered
    /// (this one gives the colour for a non-hovered button in a dark theme).
    pub fn number_hint(&self, button: &PieButton) -> Option<NumberHint> {
        self.number_hint_styled(button, false, &egui::Visuals::dark())
    }

    /// [`Self::number_hint`] with the colour resolved for `hovered` against `visuals`.
    fn number_hint_styled(&self, button: &PieButton, hovered: bool, visuals: &egui::Visuals) -> Option<NumberHint> {
        let dir = Self::eff(&button.direction, self.settings.mirror_horizontal);
        let hints = &self.settings.number_hints;
        self.settings.input.use_number_keys.then(|| NumberHint {
            text: self.hint_digit(&dir).to_string(),
            color: button_text_color(hints.color, visuals, hovered, button.enabled),
            on_right: self.hint_on_right(&dir),
        })
    }

    /// The hint digit for a direction, in the [`PieMenuNumberHints::digits`] style.
    fn hint_digit(&self, dir: &PieDirection) -> char {
        let digit = Self::direction_digit(dir);
        let zero = match self.settings.number_hints.digits {
            HintDigits::Western => return digit,
            HintDigits::ArabicIndic => '٠',
            HintDigits::Persian => '۰',
        };
        char::from_u32(zero as u32 + digit.to_digit(10).unwrap_or(0)).unwrap_or(digit)
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

    /// Opens the menu centred at `pos` and resets its interaction state. It stays open
    /// until [`PieMenu::show`] returns a closing response (anything but
    /// [`PieMenuResponse::None`]) or [`PieMenu::close`] is called.
    pub fn open(&mut self, pos: Pos2) {
        self.position = pos;
        self.is_open = true;
        self.pinned = false;
        self.release_handled = false;
        self.hovered_index = None;
        self.visible = false;
        self.shown_at = None;
        self.path.clear();
        self.level_centers.clear();
        self.hover_since = None;
        self.close_fan();
        self.level_fresh = true;
        self.inert_hover = None;
        self.selected_path.clear();
        self.checked_buttons = false;
        self.double_tap_pending = false;
        self.fresh_open = true;
    }

    /// Opens the menu as a pinned (persistent) one centred at `center`:
    ///
    /// - visible immediately, regardless of [`ShowBehavior`];
    /// - stays open after a selection ([`PieMenuResponse::Selected`] is still returned);
    /// - never blocks the widgets behind it, and leaves typing to a focused text field;
    /// - is dismissed only by [`PieMenuInput::pinned_dismiss_inputs`] (default: Escape)
    ///   or [`PieMenu::close`];
    /// - ignores `key_down`: there is no key to release.
    ///
    /// Set [`PieMenu::pointer_area`] so clicks on the rest of your UI don't select the
    /// button the pointer happens to point towards. Update [`PieMenu::position`] to move it.
    pub fn pin(&mut self, center: Pos2) {
        self.open(center);
        self.pinned = true;
        self.reveal();
    }

    /// Closes the menu without a response. Does nothing if it is not open.
    pub fn close(&mut self) {
        self.is_open = false;
        self.pinned = false;
    }

    /// Whether the menu is open: [`PieMenu::show`] does nothing unless it is.
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Whether the open menu is a pinned one (see [`PieMenu::pin`]).
    pub fn is_pinned(&self) -> bool {
        self.is_open && self.pinned
    }

    /// Makes the open menu visible immediately, regardless of [`ShowBehavior`].
    pub fn reveal(&mut self) {
        self.visible = true;
        self.shown_at = None;
    }

    /// Index of the button the pointer currently highlights in the level being shown, if
    /// any. While a fan-out is open this is the button that opened it.
    pub fn hovered(&self) -> Option<usize> {
        self.hovered_index
    }

    /// The open fan-out, if any: the index of the button that opened it (in the level
    /// being shown) and the index of the fan child the pointer highlights.
    pub fn open_fan_out(&self) -> Option<(usize, Option<usize>)> {
        self.fan.map(|trigger| (trigger, self.fan_hover))
    }

    /// Test helper: where each button of the open fan was drawn last frame.
    #[cfg(test)]
    pub(crate) fn fan_centers_for_test(&self) -> &[Pos2] {
        &self.fan_centers
    }

    fn open_fan(&mut self, trigger: usize) {
        self.fan = Some(trigger);
        self.last_fan = Some(trigger);
        self.fan_shown_at = None;
        self.fan_sizes.clear();
        self.fan_hover = None;
        self.fan_armed = false;
        self.fan_pending = None;
        self.hover_since = None;
    }

    fn close_fan(&mut self) {
        self.fan = None;
        self.fan_hover = None;
        self.fan_pending = None;
    }

    /// Shows the pie menu for this frame. Call it every frame; it does nothing (and
    /// returns [`PieMenuResponse::None`]) unless the menu is open (see [`PieMenu::open`]).
    /// Any other response closes the menu, except a selection in a pinned menu.
    ///
    /// `render_button(ui, &button_view)` is called once per button inside a floating
    /// [`egui::Area`] centred at that button's position. Use it to draw any egui
    /// content — labels, images, custom widgets, etc. The [`ButtonView`] carries the
    /// index, hover state, resolved mnemonic and number hint; `ui.label(b.label_text())`
    /// draws the label with its mnemonic underlined.
    ///
    /// `key_down` should be `true` while the key or button that opened the menu is held.
    ///
    /// `buttons` is always the **root** level. When a button has children
    /// ([`PieButton::with_children`]), selecting it opens them as a sub-pie, replacing
    /// the level shown or fanning out next to it ([`SubPieStyle`]). The `usize` in
    /// [`PieMenuResponse::Selected`] is the chosen button's index within its own level;
    /// the full route to it is [`PieMenu::selected_path`], and each button's route is
    /// [`ButtonView::path`] in the render closure.
    ///
    /// The pointer position is egui's latest one; use [`PieMenu::show_with_pointer`] to
    /// supply your own.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        buttons: &[PieButton],
        key_down: bool,
        render_button: impl FnMut(&mut Ui, &ButtonView),
    ) -> PieMenuResponse {
        let pointer = ctx.input(|i| i.pointer.latest_pos());
        self.show_with_pointer(ctx, buttons, pointer, key_down, render_button)
    }

    /// [`PieMenu::show`] with the pointer position supplied by the caller (`None`: no
    /// pointer), e.g. one in another coordinate space or from a different input source.
    pub fn show_with_pointer(
        &mut self,
        ctx: &egui::Context,
        buttons: &[PieButton],
        pointer: Option<Pos2>,
        key_down: bool,
        mut render_button: impl FnMut(&mut Ui, &ButtonView),
    ) -> PieMenuResponse {
        if !self.is_open {
            return PieMenuResponse::None;
        }
        let current_mouse_pos = pointer.filter(|p| self.pointer_area.is_none_or(|area| area.contains(*p)));
        // A pinned menu has no key to release.
        let key_down = key_down || self.pinned;

        // A stale path (the buttons changed under an open sub-pie, or its level is now
        // empty) falls back to the root level.
        if Self::resolve_level(buttons, &self.path).is_none_or(|level| level.is_empty()) {
            self.back_to_root();
        }
        let level = Self::resolve_level(buttons, &self.path).unwrap_or(buttons);

        // Backspace closes an open fan-out, otherwise goes back one level; so does a click
        // on the centre (when clicks are enabled).
        if self.visible && (self.fan.is_some() || !self.path.is_empty()) {
            let thr = self.settings.dead_zone_radius;
            let backspace = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Backspace));
            let centre_click = self.settings.input.select_on_primary_click
                && ctx.input(|i| i.pointer.primary_clicked())
                && current_mouse_pos.is_some_and(|p| (p - self.position).length() <= thr);
            if backspace && self.fan.is_some() {
                self.close_fan();
                return PieMenuResponse::None;
            }
            if (backspace || centre_click) && !self.path.is_empty() {
                self.go_back(current_mouse_pos, key_down);
                return PieMenuResponse::None;
            }
        }

        let response = match self.show_level(ctx, level, current_mouse_pos, key_down, &mut render_button) {
            LevelEvent::None => PieMenuResponse::None,
            LevelEvent::Dismissed => PieMenuResponse::Dismissed,
            LevelEvent::QuickTap => PieMenuResponse::QuickTap,
            LevelEvent::DoubleTap => PieMenuResponse::DoubleTap,
            LevelEvent::Pick(idx) => {
                let button = level.get(idx);
                if button.is_some_and(|b| b.is_back()) && !self.path.is_empty() {
                    self.go_back(current_mouse_pos, key_down);
                    PieMenuResponse::None
                } else if button.is_some_and(|b| self.opens_fan(b)) {
                    self.open_fan(idx);
                    // As for a sub-pie: if the key is already up, the fan is click/keyboard
                    // driven; otherwise a later release selects in it.
                    self.release_handled = !key_down;
                    PieMenuResponse::None
                } else if button.is_some_and(|b| !b.children().is_empty()) {
                    self.descend(idx, current_mouse_pos, key_down);
                    PieMenuResponse::None
                } else {
                    self.selected_path = self.path.clone();
                    self.selected_path.push(idx);
                    PieMenuResponse::Selected(idx)
                }
            }
            LevelEvent::FanPick(trigger, child) => {
                let is_back = level.get(trigger).and_then(|b| b.children().get(child)).is_some_and(|b| b.is_back());
                if is_back {
                    self.close_fan();
                    PieMenuResponse::None
                } else {
                    self.selected_path = self.path.clone();
                    self.selected_path.extend([trigger, child]);
                    PieMenuResponse::Selected(child)
                }
            }
        };
        match response {
            PieMenuResponse::None => {}
            // A pinned menu stays open after a selection, back at its root level.
            PieMenuResponse::Selected(_) if self.pinned => {
                if !self.path.is_empty() {
                    self.back_to_root();
                    self.restart_level(None, key_down);
                }
            }
            _ => self.close(),
        }
        response
    }

    /// The level currently displayed: the root, or the open (replace-style) sub-pie.
    /// An open fan-out's children are not a level of their own: they are the
    /// [`PieButton::children`] of one of these buttons (see [`PieMenu::open_fan_out`]).
    pub fn current_buttons<'a>(&self, root: &'a [PieButton]) -> &'a [PieButton] {
        Self::resolve_level(root, &self.path).unwrap_or(root)
    }

    /// Indices of the parent buttons descended through to reach the current sub-pie.
    /// Empty at the root level.
    pub fn path(&self) -> &[usize] {
        &self.path
    }

    /// After [`PieMenuResponse::Selected`]: the full index path from the root to the
    /// chosen button (the last element is the index within its level).
    pub fn selected_path(&self) -> &[usize] {
        &self.selected_path
    }

    fn resolve_level<'a>(root: &'a [PieButton], path: &[usize]) -> Option<&'a [PieButton]> {
        let mut level = root;
        for &i in path {
            level = level.get(i)?.children();
        }
        Some(level)
    }

    /// Opens the sub-pie of button `idx` of the current level.
    fn descend(&mut self, idx: usize, mouse: Option<Pos2>, key_down: bool) {
        let center = match self.settings.sub_pie_center {
            // A pinned menu stays put rather than jumping to the pointer.
            SubPieCenter::Pointer => mouse.filter(|_| !self.pinned),
            SubPieCenter::Parent => None,
            SubPieCenter::Button => self.button_centers.get(idx).copied(),
        };
        self.level_centers.push(self.position);
        self.path.push(idx);
        self.restart_level(center, key_down);
    }

    /// Returns to the parent level of the open sub-pie.
    fn go_back(&mut self, mouse: Option<Pos2>, key_down: bool) {
        self.path.pop();
        let parent_center = self.level_centers.pop();
        let center = match self.settings.sub_pie_center {
            SubPieCenter::Pointer if !self.pinned => mouse,
            _ => parent_center,
        };
        self.restart_level(center, key_down);
    }

    /// Forgets the open sub-pie(s); the root level reappears where it was when it was left
    /// ([`SubPieCenter::Pointer`]: where the menu is now, unless pinned).
    fn back_to_root(&mut self) {
        let root_center = self.level_centers.first().copied();
        if self.pinned || self.settings.sub_pie_center != SubPieCenter::Pointer {
            self.position = root_center.unwrap_or(self.position);
        }
        self.path.clear();
        self.level_centers.clear();
        self.hover_since = None;
    }

    /// Shows the (new) level immediately, centred at `center` (`None`: where the menu is).
    fn restart_level(&mut self, center: Option<Pos2>, key_down: bool) {
        if let Some(p) = center {
            self.position = p;
        }
        self.hover_since = None;
        self.close_fan();
        self.last_fan = None;
        self.level_fresh = true;
        self.inert_hover = None;
        self.hovered_index = None;
        self.checked_buttons = false;
        self.visible = true;
        self.shown_at = None; // restart the reveal animation on the next `show`
        // If the key is already up (selection came from the release), the sub-pie is
        // click/keyboard driven; otherwise a later release selects as usual.
        self.release_handled = !key_down;
    }

    /// Shifts `center` so the ring of `buttons` (sizes from `sizes`) at `radius` stays
    /// within the screen with the configured margin.
    fn clamp_center(&self, center: Pos2, buttons: &[PieButton], sizes: &[Vec2], radius: f32, screen: Rect) -> Pos2 {
        let mirror = self.settings.mirror_horizontal;
        let margin = self.settings.screen_margin;
        let mut bb_min = Vec2::ZERO;
        let mut bb_max = Vec2::ZERO;
        for (button, &size) in buttons.iter().zip(sizes) {
            let dir = Self::eff(&button.direction, mirror);
            let offset = self.button_offset(&dir, size, radius, self.settings.shape_factor);
            bb_min = bb_min.min(offset - size / 2.0);
            bb_max = bb_max.max(offset + size / 2.0);
        }
        Pos2::new(
            center.x.clamp(screen.min.x + margin - bb_min.x, screen.max.x - margin - bb_max.x),
            center.y.clamp(screen.min.y + margin - bb_min.y, screen.max.y - margin - bb_max.y),
        )
    }

    /// Each button's displayed layout vector (which follows `intercardinal_height`), for
    /// hover picking and the centre highlight.
    fn layout_dirs(&self, buttons: &[PieButton], sizes: &[Vec2], radius: f32) -> Vec<Vec2> {
        let mirror = self.settings.mirror_horizontal;
        buttons
            .iter()
            .zip(sizes)
            .map(|(b, &size)| self.layout_vec(&Self::eff(&b.direction, mirror), size, radius, self.settings.shape_factor))
            .collect()
    }

    /// Number-key or mnemonic selection among `buttons` (a level, or an open fan-out's
    /// children). Matched keys and their text events are consumed, so they reach neither
    /// other handlers nor text fields.
    fn key_pick(&self, ctx: &egui::Context, buttons: &[PieButton]) -> Option<usize> {
        let mirror = self.settings.mirror_horizontal;
        if self.settings.input.use_number_keys {
            for (idx, button) in buttons.iter().enumerate().filter(|(_, b)| b.enabled) {
                let dir = Self::eff(&button.direction, mirror);
                let key = Self::direction_numpad(&dir);
                let digit = Self::direction_digit(&dir).to_string();
                let hit = ctx.input_mut(|i| {
                    let hit = i.consume_key(egui::Modifiers::NONE, key);
                    if hit
                        && let Some(p) = i
                            .events
                            .iter()
                            .position(|e| matches!(e, egui::Event::Text(t) if *t == digit))
                    {
                        i.events.remove(p);
                    }
                    hit
                });
                if hit {
                    return Some(idx);
                }
            }
        }

        if self.settings.input.use_mnemonic_keys {
            let mnemonics = self.mnemonics(buttons);
            for (idx, mnemonic) in mnemonics.iter().enumerate() {
                if !buttons[idx].enabled {
                    continue;
                }
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
                    // Latin letters also match by key code, but not as part of a Ctrl/Cmd
                    // chord: Ctrl+C is a shortcut, not the mnemonic `c`.
                    if let Some(key) = key {
                        hit |= i.consume_key(egui::Modifiers::NONE, key);
                    }
                    hit
                });
                if pressed {
                    return Some(idx);
                }
            }
        }
        None
    }

    /// Draws one ring of buttons through the caller's closure, each in its own floating
    /// Area. `sizes` holds last frame's button sizes (used to centre each Area) and is
    /// updated; `centers`, if given, receives each button's final (un-animated) centre.
    fn draw_level(
        &self,
        ctx: &egui::Context,
        now: f64,
        level: &LevelDraw,
        sizes: &mut [Vec2],
        mut centers: Option<&mut Vec<Pos2>>,
        render_button: &mut impl FnMut(&mut Ui, &ButtonView),
    ) {
        let mirror = self.settings.mirror_horizontal;
        let shape_factor = self.settings.shape_factor;
        // Resolved once per frame for the view handed to the render closure.
        let mnemonics = self.mnemonics(level.buttons);
        let visuals = ctx.global_style().visuals.clone();
        let body_font = ctx.global_style().text_styles.get(&egui::TextStyle::Body).cloned().unwrap_or_default();

        // Reveal animation: movement and opacity progress, each 0..1.
        let anim = &self.settings.animations;
        let elapsed = level.shown_at.map(|at| (now - at) as f32);
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

        let mut path = level.path_prefix.to_vec();
        path.push(0);
        for (idx, button) in level.buttons.iter().enumerate() {
            // Each axis is offset independently (see `button_offset`), which keeps NE/NW at
            // the same Y and SE/SW at the same Y even when button widths differ.
            let eff_dir = Self::eff(&button.direction, mirror);
            let cached_size = sizes[idx];
            let final_center = level.center + self.button_offset(&eff_dir, cached_size, level.radius, shape_factor);
            if let Some(centers) = centers.as_deref_mut() {
                if centers.len() <= idx {
                    centers.resize(idx + 1, final_center);
                }
                centers[idx] = final_center;
            }
            let dimmed = level.dim_except.is_some_and(|keep| keep != idx);
            let is_hovered = level.hovered == Some(idx) && !dimmed;

            let button_center = if move_t < 1.0 {
                let eased = 1.0 - (1.0 - move_t).powi(3);
                // Interpolate the radius and (optionally) the shape factor, then lay the
                // button out as if that were the final layout, scaled about the centre.
                // With a start radius of 0 this collapses every button onto the centre.
                let end_radius = level.radius;
                let radius = anim.slide_start_radius + (end_radius - anim.slide_start_radius) * eased;
                let start_sf = anim.slide_start_shape_factor.unwrap_or(shape_factor);
                let sf = start_sf + (shape_factor - start_sf) * eased;
                let scale = if end_radius > 0.0 { radius / end_radius } else { 1.0 };
                level.center + self.button_offset(&eff_dir, cached_size, end_radius, sf) * scale
            } else {
                final_center
            };
            let area_pos = button_center - cached_size / 2.0;
            if let Some(last) = path.last_mut() {
                *last = idx;
            }

            let response = egui::Area::new(level.id.with(idx))
                .order(egui::Order::Tooltip)
                // egui's own area fade is always off: Fade replaces it, None means none.
                .fade_in(false)
                .fixed_pos(area_pos)
                .show(ctx, |ui| {
                    if opacity < 1.0 {
                        ui.set_opacity(opacity);
                    }
                    if dimmed {
                        ui.multiply_opacity(level.dim_opacity);
                    }
                    // Buttons are single-line. An Area keeps last frame's size as its max
                    // width, so content that grows (a number hint added, a label changed)
                    // would wrap instead of widening the button. Don't wrap; let it grow.
                    // Explicit newlines still work, and the closure can override this.
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    // A press on a button's label selects the button, not its text.
                    // Set on this Ui only, so the app's own labels and style are untouched.
                    ui.style_mut().interaction.selectable_labels = false;
                    // Disabled: egui's disabled look, and no hover outline on the widget.
                    if !button.enabled {
                        ui.disable();
                    }
                    render_button(ui, &ButtonView {
                        idx,
                        path: &path,
                        hovered: is_hovered,
                        enabled: button.enabled,
                        button,
                        mnemonic: mnemonics[idx],
                        number_hint: self.number_hint_styled(button, is_hovered, &visuals),
                        rtl: mirror,
                        text_color: button_text_color(self.settings.text_color, &visuals, is_hovered, button.enabled),
                        body_font: body_font.clone(),
                    })
                });

            sizes[idx] = response.response.rect.size();

            // Hover emphasis: ease the highlighted button outward and/or larger by
            // transforming its whole layer, so it works whatever the closure drew. Layer
            // transforms are sticky, so the identity is set again once it has eased back.
            let emphasis = ctx.animate_bool_with_time(
                level.id.with((idx, "hover")),
                is_hovered && !level.emphasis.is_none(),
                anim.hover_duration.as_secs_f32(),
            );
            let transform = if emphasis > 0.0 {
                let outward = (final_center - level.center).normalized();
                let outward = if outward.is_finite() { outward } else { Vec2::ZERO };
                let scale = 1.0 + (level.emphasis.scale.max(0.0) - 1.0) * emphasis;
                // Scale about the button's centre, then move it outward.
                let pivot = final_center.to_vec2();
                TSTransform::from_translation(pivot + outward * level.emphasis.shift * emphasis)
                    * TSTransform::from_scaling(scale)
                    * TSTransform::from_translation(-pivot)
            } else {
                TSTransform::IDENTITY
            };
            ctx.set_transform_layer(response.response.layer_id, transform);
        }
    }

    fn show_level(
        &mut self,
        ctx: &egui::Context,
        buttons: &[PieButton],
        current_mouse_pos: Option<Pos2>,
        key_down: bool,
        render_button: &mut impl FnMut(&mut Ui, &ButtonView),
    ) -> LevelEvent {
        // Ensure cached size vec is long enough
        if self.button_sizes.len() < buttons.len() {
            self.button_sizes
                .resize(buttons.len(), Vec2::new(50.0, 24.0));
        }

        // egui's clock (seconds), also available on wasm.
        let now = ctx.input(|i| i.time);
        if self.fresh_open {
            self.fresh_open = false;
            let window = self.settings.input.double_tap_window.as_secs_f64();
            self.double_tap_pending = self.last_quick_tap.is_some_and(|t| now - t <= window);
        }

        let center = self.position;

        // Update visible: latch on once the show condition is met.
        // (shown_at is stamped below, once visible has latched)
        // A double-tap-and-hold must not reveal the menu (see `double_tap_pending`).
        if !self.visible && !self.double_tap_pending {
            match self.settings.show_behavior {
                ShowBehavior::Instant => self.visible = true,
                ShowBehavior::OnMovement { threshold } => {
                    if current_mouse_pos.is_some_and(|p| (p - center).length() > threshold) {
                        self.visible = true;
                    }
                }
            }
        }

        if self.visible && self.shown_at.is_none() {
            self.shown_at = Some(now);
        }

        // Before the menu is visible, handle key release as QuickTap/DoubleTap only.
        if !self.visible {
            if !key_down && !self.release_handled {
                self.release_handled = true;
                if self.double_tap_pending {
                    self.double_tap_pending = false;
                    self.last_quick_tap = None;
                    return LevelEvent::DoubleTap;
                } else {
                    self.last_quick_tap = Some(now);
                    return LevelEvent::QuickTap;
                }
            }
            return LevelEvent::None;
        }

        // Keyboard ownership. A blocking menu takes keyboard focus away, so a text field
        // that was focused when the menu opened stops receiving keys from the next frame
        // (this frame it already ran, so consuming keys below would be too late).
        // A non-blocking (or pinned) menu leaves typing alone: while egui wants keyboard
        // input, dismiss keys, number keys and mnemonics go to that widget, not the menu.
        let blocking = self.settings.block_background && !self.pinned;
        let keys_for_menu = if blocking {
            ctx.memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
            true
        } else {
            !ctx.egui_wants_keyboard_input()
        };

        // Early exit checks. Dismiss keys are consumed, and only match without Ctrl/Cmd.
        let input = &self.settings.input;
        let dismiss_inputs = if self.pinned { &input.pinned_dismiss_inputs } else { &input.dismiss_inputs };
        let dismissed = ctx.input_mut(|i| {
            dismiss_inputs.iter().any(|d| match d {
                PieMenuDismissInput::Key(k) => keys_for_menu && i.consume_key(egui::Modifiers::NONE, *k),
                PieMenuDismissInput::PointerButton(pb) => i.pointer.button_clicked(*pb),
            })
        });
        if dismissed {
            return LevelEvent::Dismissed;
        }

        let shape_factor = self.settings.shape_factor;
        let mirror = self.settings.mirror_horizontal;
        let radius = self.settings.layout_radius;
        let dead_zone = self.settings.dead_zone_radius;

        // Shift the whole menu so every button stays within the screen rect with a margin.
        let screen_rect = ctx.content_rect();
        let center = self.clamp_center(center, buttons, &self.button_sizes, radius, screen_rect);

        // Block hover/interaction on background widgets while the menu is open.
        // Sits on Foreground (above panels, below Tooltip where buttons live) so
        // nothing underneath highlights as the mouse moves over it.
        if blocking {
            egui::Area::new(self.id.with("_blocker"))
                .order(egui::Order::Foreground)
                .fixed_pos(egui::Pos2::ZERO)
                .show(ctx, |ui| {
                    ui.allocate_rect(screen_rect, egui::Sense::click_and_drag());
                });
        }

        // Debug builds: assert once per open (and per sub-pie level) that this level's
        // buttons are well-formed. Only buttons shown together are compared.
        #[cfg(debug_assertions)]
        if !self.checked_buttons {
            self.checked_buttons = true;
            let problems = self.check_buttons(buttons);
            debug_assert!(
                problems.is_empty(),
                "egui_pie_menu: {}",
                problems.iter().map(|p| p.to_string()).collect::<Vec<_>>().join("; ")
            );
        }

        // The button the pointer points at in this level, by angle against each button's
        // displayed layout vector. Nothing inside the dead zone.
        let dirs = self.layout_dirs(buttons, &self.button_sizes, radius);
        let level_pick = current_mouse_pos.and_then(|p| {
            let mouse_vec = p - center;
            (mouse_vec.length() > dead_zone).then(|| Self::pick_button(buttons, mouse_vec, &dirs)).flatten()
        });

        // An open fan-out closes when its button is gone, when the pointer comes back in to
        // the parent after having been out at the fan, or (before it has been out there,
        // e.g. a fan opened by key) when it points at another button of the parent.
        if let Some(trigger) = self.fan {
            let gone = buttons.get(trigger).is_none_or(|b| !self.opens_fan(b));
            let left = current_mouse_pos.is_some_and(|p| {
                let distance = (p - center).length();
                if distance >= radius {
                    self.fan_armed = true;
                }
                if self.fan_armed { distance < radius - dead_zone } else { level_pick.is_some_and(|i| i != trigger) }
            });
            if gone || left {
                self.close_fan();
            }
        }

        // Lay the open fan out around its button.
        let fan: Option<Fan> = self.fan.map(|trigger| {
            let button = &buttons[trigger];
            let children = button.children();
            if self.fan_sizes.len() < children.len() {
                self.fan_sizes.resize(children.len(), Vec2::new(50.0, 24.0));
            }
            let size = self.button_sizes[trigger];
            let button_center = center + self.button_offset(&Self::eff(&button.direction, mirror), size, radius, shape_factor);
            let fan_radius = self.settings.fan_out_radius.unwrap_or(radius);
            Fan {
                trigger,
                children,
                center: self.clamp_center(button_center, children, &self.fan_sizes, fan_radius, screen_rect),
                radius: fan_radius,
                trigger_rect: Rect::from_center_size(button_center, size),
            }
        });

        // The parent's centre (circle, highlight, title) and its other buttons are greyed out
        // while a fan is open, easing in and out with it.
        let dim = ctx.animate_bool_with_time(self.id.with("fan_dim"), fan.is_some(), 0.12);
        let dim_opacity = 1.0 + (DIMMED_OPACITY - 1.0) * dim;
        let mut painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, self.id));
        painter.multiply_opacity(dim_opacity);

        // Center background circle
        if let Some(radius) = self.settings.center_indicator.background_radius {
            painter.circle(
                center,
                radius,
                self.settings.center_indicator.background_fill_color,
                self.settings.center_indicator.background_stroke,
            );
        }

        // Keys: an open fan has the keyboard to itself.
        if keys_for_menu {
            match &fan {
                Some(fan) => {
                    if let Some(child) = self.key_pick(ctx, fan.children) {
                        return LevelEvent::FanPick(fan.trigger, child);
                    }
                }
                None => {
                    if let Some(idx) = self.key_pick(ctx, buttons) {
                        return LevelEvent::Pick(idx);
                    }
                }
            }
        }

        // Hover.
        match &fan {
            Some(fan) => {
                // The parent's highlight stays on the fan's button; the pointer picks among
                // the fan's children, by angle from the fan's centre. Nothing while it is
                // on the opening button, in the fan's dead zone, or pointing back at the
                // parent (the blocked sector: three of the eight 45° slices).
                self.hovered_index = Some(fan.trigger);
                self.hover_since = None;
                let fan_dirs = self.layout_dirs(fan.children, &self.fan_sizes, fan.radius);
                self.fan_hover = current_mouse_pos.and_then(|p| {
                    let v = p - fan.center;
                    let back = center - fan.center;
                    let blocked = Self::angle_diff(v.y.atan2(v.x), back.y.atan2(back.x)) < 67.5f32.to_radians();
                    if fan.trigger_rect.contains(p) || v.length() <= dead_zone || blocked {
                        return None;
                    }
                    Self::pick_button(fan.children, v, &fan_dirs)
                });
            }
            None => {
                self.hovered_index = level_pick;
                self.fan_hover = None;

                // A fan-out opens once the pointer has reached its button (as far out as
                // the ring) and rested on it for `fan_out_delay`.
                let reached = current_mouse_pos.is_some_and(|p| (p - center).length() >= radius);
                match level_pick.filter(|&i| reached && self.opens_fan(&buttons[i])) {
                    Some(i) => {
                        let since = match self.fan_pending {
                            Some((j, since)) if j == i => since,
                            _ => now,
                        };
                        self.fan_pending = Some((i, since));
                        let remaining = self.settings.fan_out_delay.as_secs_f64() - (now - since);
                        if remaining <= 0.0 {
                            self.open_fan(i);
                            ctx.request_repaint(); // the fan is laid out and drawn next frame
                        } else {
                            ctx.request_repaint_after(Duration::from_secs_f64(remaining));
                        }
                    }
                    None => self.fan_pending = None,
                }
            }
        }

        // Hover timer (replace-style sub-pies): resting on a button that leads to another
        // pie (a sub-pie button, or a back button inside a sub-pie) long enough goes there.
        // The button the pointer already pointed at when this level opened is ignored until
        // the pointer leaves it.
        if std::mem::take(&mut self.level_fresh) {
            self.inert_hover = self.hovered_index;
        } else if self.inert_hover != self.hovered_index {
            self.inert_hover = None;
        }
        if let Some(delay) = self.settings.sub_pie_hover_delay
            && self.fan.is_none()
        {
            let in_sub_pie = !self.path.is_empty();
            let navigates = |b: &PieButton| {
                (!b.children().is_empty() && !self.opens_fan(b)) || (b.is_back() && in_sub_pie)
            };
            let target = self.hovered_index.filter(|&i| self.inert_hover != Some(i) && navigates(&buttons[i]));
            match (target, self.hover_since) {
                (Some(i), Some((j, since))) if i == j => {
                    let remaining = delay.as_secs_f64() - (now - since);
                    if remaining <= 0.0 {
                        self.hover_since = None;
                        return LevelEvent::Pick(i);
                    }
                    ctx.request_repaint_after(Duration::from_secs_f64(remaining));
                }
                (Some(i), _) => {
                    self.hover_since = Some((i, now));
                    ctx.request_repaint_after(delay);
                }
                (None, _) => self.hover_since = None,
            }
        }

        // Center highlight — progress driven by mouse distance from center.
        let indicator = &self.settings.center_indicator;
        if let Some(highlight_radius) = indicator.highlight_radius
            && !indicator.highlight_shape.is_none()
        {
            let progress = current_mouse_pos
                .filter(|_| self.hovered_index.is_some())
                .map(|p| {
                    let distance = (p - center).length();
                    ((distance - dead_zone) / (radius - dead_zone)).clamp(0.0, 1.0)
                })
                .unwrap_or(0.0);

            // With a fan open the highlight stays on the fan's button.
            let base_angle = if indicator.highlight_snapping || fan.is_some() {
                self.hovered_index
                    .and_then(|idx| dirs.get(idx))
                    .map(|v| v.y.atan2(v.x))
                    .unwrap_or(0.0)
            } else {
                current_mouse_pos
                    .map(|p| {
                        let v = p - center;
                        v.y.atan2(v.x)
                    })
                    .unwrap_or(0.0)
            };

            let arc_angle = indicator.highlight_angle * progress;
            let start_angle = base_angle - arc_angle / 2.0;
            highlight_shapes::paint(&painter, highlight_shapes::Highlight {
                shape: indicator.highlight_shape,
                center,
                radius: highlight_radius,
                angle_range: start_angle..(start_angle + arc_angle),
                dot_angle: base_angle,
                dot_radius: indicator.highlight_circle_radius,
                stroke: Stroke::new(
                    indicator.highlight_stroke.width,
                    indicator.highlight_stroke.color.gamma_multiply(progress),
                ),
                fill: indicator.highlight_fill_color.gamma_multiply(progress),
            });
        }

        // Draw the title above the center indicator
        let title_style = &self.settings.title;
        if title_style.show
            && let Some(title) = &self.title
        {
            let pad = title_style.padding;
            let galley = painter.layout_no_wrap(title.clone(), title_style.text_font.clone(), title_style.text_color);
            let box_size = galley.size() + pad.sum();
            // Sits just above the background circle, or the centre when there is none.
            let above_offset =
                self.settings.center_indicator.background_radius.unwrap_or(0.0) + box_size.y / 2.0 + 4.0;
            let bg_rect = Rect::from_center_size(center - Vec2::new(0.0, above_offset), box_size);

            if title_style.background_color != Color32::TRANSPARENT {
                painter.rect_filled(bg_rect, 3.0, title_style.background_color);
            }
            if title_style.background_stroke.width > 0.0
                && title_style.background_stroke.color != Color32::TRANSPARENT
            {
                painter.rect_stroke(bg_rect, 3.0, title_style.background_stroke, egui::StrokeKind::Outside);
            }
            painter.galley(bg_rect.min + pad.left_top(), galley, title_style.text_color);
        }

        // Draw the level's buttons, then the open fan's around its button. The caches are
        // taken out of `self` while its settings are borrowed for drawing.
        let path = self.path.clone();
        let mut sizes = std::mem::take(&mut self.button_sizes);
        let mut centers = std::mem::take(&mut self.button_centers);
        self.draw_level(
            ctx,
            now,
            &LevelDraw {
                buttons,
                center,
                radius,
                shown_at: self.shown_at,
                hovered: self.hovered_index,
                // While the dimming eases out after a fan closed, its button stays the lit one.
                dim_except: fan.as_ref().map(|f| f.trigger).or(self.last_fan.filter(|_| dim > 0.0)),
                dim_opacity,
                // The root pie and sub-pies have their own amounts.
                emphasis: if path.is_empty() {
                    self.settings.animations.hover_emphasis
                } else {
                    self.settings.animations.sub_pie_hover_emphasis
                },
                id: self.id,
                path_prefix: &path,
            },
            &mut sizes,
            Some(&mut centers),
            render_button,
        );
        self.button_sizes = sizes;
        self.button_centers = centers;

        if let Some(fan) = &fan {
            let shown_at = *self.fan_shown_at.get_or_insert(now);
            let mut fan_path = path.clone();
            fan_path.push(fan.trigger);
            let mut sizes = std::mem::take(&mut self.fan_sizes);
            // Only tests look at where the fan's buttons ended up.
            let mut fan_centers: Option<Vec<Pos2>> = cfg!(test).then(Vec::new);
            self.draw_level(
                ctx,
                now,
                &LevelDraw {
                    buttons: fan.children,
                    center: fan.center,
                    radius: fan.radius,
                    shown_at: Some(shown_at),
                    hovered: self.fan_hover,
                    dim_except: None,
                    dim_opacity: 1.0,
                    emphasis: self.settings.animations.sub_pie_hover_emphasis,
                    id: self.id.with(("fan", fan.trigger)),
                    path_prefix: &fan_path,
                },
                &mut sizes,
                fan_centers.as_mut(),
                render_button,
            );
            self.fan_sizes = sizes;
            #[cfg(test)]
            {
                self.fan_centers = fan_centers.unwrap_or_default();
            }
        }

        // Primary click selection — runs after render so closures (e.g. checkboxes)
        // receive the click event before we consume it.
        if self.settings.input.select_on_primary_click && ctx.input(|i| i.pointer.primary_clicked())
        {
            return match (current_mouse_pos, &fan) {
                // In an open fan a click selects the highlighted child, if any.
                (Some(_), Some(fan)) => self.fan_hover.map_or(LevelEvent::None, |child| LevelEvent::FanPick(fan.trigger, child)),
                (Some(p), None) if (p - center).length() > dead_zone => {
                    self.hovered_index.map_or(LevelEvent::None, LevelEvent::Pick)
                }
                (Some(_), None) => LevelEvent::None,
                // A click elsewhere: nothing for a pinned menu, otherwise it dismisses.
                (None, _) if self.pinned => LevelEvent::None,
                (None, _) => LevelEvent::Dismissed,
            };
        }

        // Key-hold release (visible is true here, so menu was visible)
        if !key_down && !self.release_handled {
            self.release_handled = true;
            if let Some(fan) = &fan {
                // Released on a fan child: select it. Released on the fan's button: the fan
                // stays open, click/keyboard driven.
                if let Some(child) = self.fan_hover {
                    return LevelEvent::FanPick(fan.trigger, child);
                }
            } else if let Some(mouse_pos) = current_mouse_pos {
                if (mouse_pos - center).length() <= dead_zone {
                    return LevelEvent::Dismissed;
                } else if let Some(idx) = self.hovered_index {
                    return LevelEvent::Pick(idx);
                }
            }
        }

        LevelEvent::None
    }
}
