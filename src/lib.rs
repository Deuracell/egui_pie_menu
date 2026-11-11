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
//!     // `idx` is the index within the button's own level; `selected_path` is the full route.
//!     PieMenuResponse::Selected(idx) => println!("selected {idx}, route {:?}", menu.selected_path()),
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

pub use button::{ButtonProblem, ButtonView, ClusterItem, LabelText, MnemonicOwner, NumberHint, PieButton, PieButtonKind};
pub use mnemonic::{Mnemonic, mnemonic_text};

use button::{ItemHit, button_problems, button_text_color, fan_out_problems, item_problems};
use mnemonic::{char_to_key, fold_char, resolve_all, resolve_mnemonics};

/// Outcome of a [`PieMenu::show`] call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PieMenuResponse {
    /// A button or cluster item was activated. For a button the value is its index within
    /// its own level: the slice passed to `show` for a root button, but the index in the
    /// sub-pie's or fan-out's children for a button inside one. For an item of a cluster
    /// ([`PieButton::cluster`]) it is the item number you gave to [`ButtonView::item`].
    /// [`PieMenu::selected_path`] is the full route from the root.
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
    /// An item of a cluster button of the level being shown: (the cluster button, the
    /// item number).
    ClusterPick(usize, usize),
    /// A click in an open fan-out that hit no fan button: close the fan.
    CloseFan,
    /// The fan's own opening button was selected: break the fan out into a sub-pie.
    BreakOut(usize),
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
    /// Multiplied into every button's opacity (1.0: untouched; the fading main ring of a
    /// break-out uses `1 - fade`).
    opacity_scale: f32,
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

/// A fan-out that was broken out into a sub-pie: what the transition needs. The sub-pie
/// itself is an ordinary level (see [`PieMenu::break_out`]); this is the main ring fading
/// out, the opener included.
struct BreakOut {
    /// egui time of the break-out, for the fades.
    started: f64,
    /// Where the parent's centre (circle) and ring were drawn.
    parent_center: Pos2,
    parent_radius: f32,
    /// The parent ring's cached button sizes.
    parent_sizes: Vec<Vec2>,
    /// The opener's index in the parent ring.
    opener: usize,
}

/// The longest repaint delay ever requested. A huge ("never") delay setting is capped to
/// this instead of overflowing the conversion to a `Duration`.
const MAX_REPAINT_DELAY: Duration = Duration::from_secs(3600);

/// Asks egui for a repaint in `secs` seconds (capped, never panicking).
fn repaint_after(ctx: &egui::Context, secs: f64) {
    let delay = Duration::try_from_secs_f64(secs.max(0.0)).unwrap_or(MAX_REPAINT_DELAY);
    ctx.request_repaint_after(delay.min(MAX_REPAINT_DELAY));
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
    /// Bumped by every `open`, so animation state kept by egui (button emphasis, fan
    /// dimming) starts fresh for each opening and does not leak from the previous one.
    anim_gen: u64,
    /// Where the menu's centre was actually drawn last frame (the position clamped to the
    /// screen). The centre click that goes back is tested against this, not `position`.
    display_center: Pos2,
    /// The open fan-out: index of its button in the level being shown.
    fan: Option<usize>,
    /// The button of the fan that was open last, while the parent's dimming eases out.
    last_fan: Option<usize>,
    /// A fan the user just closed (click, Backspace, back button) while the pointer still
    /// rests on its button: it does not reopen until the pointer has left that button.
    fan_suppressed: Option<usize>,
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
    /// The current level was entered by descending into a sub-pie (not by going back or
    /// opening the menu), so a sub-pie button under the resting pointer starts its timer at
    /// once and the navigation flows on.
    entered_forward: bool,
    /// Set while the level shown is a broken-out fan (see [`BreakOut`]).
    breakout: Option<BreakOut>,
    /// The radius of the broken-out level (the fan's); `None`: `layout_radius`.
    level_radius: Option<f32>,
    /// Where the open fan's centre was drawn last frame, and its radius.
    fan_display_center: Pos2,
    fan_display_radius: f32,
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
            anim_gen: 0,
            display_center: Pos2::ZERO,
            fan: None,
            last_fan: None,
            fan_suppressed: None,
            fan_shown_at: None,
            fan_sizes: Vec::new(),
            #[cfg(test)]
            fan_centers: Vec::new(),
            fan_hover: None,
            fan_armed: false,
            fan_pending: None,
            level_fresh: true,
            inert_hover: None,
            entered_forward: false,
            breakout: None,
            level_radius: None,
            fan_display_center: Pos2::ZERO,
            fan_display_radius: 0.0,
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
    /// on several enabled buttons (or cluster items, which share the level's keys); two
    /// items of a cluster with the same number; and, for each button that opens a fan-out, the same
    /// among its children plus children in a blocked direction or with sub-pies of their
    /// own ([`ButtonProblem::InFanOut`]). Debug builds assert on these when the level
    /// opens; call this to check (or show the problems) yourself, e.g. in an editor, in
    /// any build. Pass the buttons of a single level.
    pub fn check_buttons(&self, buttons: &[PieButton]) -> Vec<ButtonProblem> {
        let (mnemonics, item_mnemonics) = resolve_all(buttons, self.settings.input.auto_mnemonics);
        let mut problems = button_problems(buttons, &mnemonics);
        problems.extend(item_problems(buttons, &mnemonics, &item_mnemonics));
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

    /// The mnemonic key in effect for each declared item of each cluster among `buttons`:
    /// per button, per item in declaration order (empty for a button that is not a
    /// cluster). Items compete with the level's buttons for keys, so no key is handed
    /// out twice; see [`ClusterItem`]. Inside the render closure, prefer
    /// [`ButtonView::item_mnemonic`] and [`ButtonView::item_label`].
    pub fn cluster_mnemonics(&self, buttons: &[PieButton]) -> Vec<Vec<Option<char>>> {
        resolve_all(buttons, self.settings.input.auto_mnemonics).1
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
        (self.settings.input.use_number_keys && !button.is_cluster()).then(|| NumberHint {
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
        self.last_fan = None;
        self.fan_suppressed = None;
        self.anim_gen = self.anim_gen.wrapping_add(1);
        self.display_center = pos;
        self.level_fresh = true;
        self.inert_hover = None;
        self.entered_forward = false;
        self.selected_path.clear();
        self.clear_breakout();
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
        self.fan_suppressed = None;
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

    /// The user closes the open fan: like [`Self::close_fan`], and it stays closed while
    /// the pointer rests where it would reopen it.
    fn dismiss_fan(&mut self) {
        self.fan_suppressed = self.fan;
        self.close_fan();
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
    /// [`PieMenuResponse::Selected`] is the chosen button's index within its own level (for a
    /// cluster's item: the number given to [`ButtonView::item`]);
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
        render_button: impl FnMut(&mut Ui, &ButtonView),
    ) -> PieMenuResponse {
        let before = (self.path.clone(), self.fan);
        let response = self.show_inner(ctx, buttons, pointer, key_down, render_button);
        // A new level or fan is drawn on the next frame; make sure there is one even if
        // nothing else (input, animation) would ask for it, e.g. after a hover timer.
        if self.path != before.0 || self.fan != before.1 {
            ctx.request_repaint();
        }
        response
    }

    fn show_inner(
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
            let backspace = self.keys_for_menu(ctx) && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Backspace));
            let centre_click = self.settings.input.select_on_primary_click
                && ctx.input(|i| i.pointer.primary_clicked())
                && current_mouse_pos.is_some_and(|p| (p - self.display_center).length() <= thr);
            if backspace && self.fan.is_some() {
                self.dismiss_fan();
                return PieMenuResponse::None;
            }
            if (backspace || centre_click) && !self.path.is_empty() {
                self.go_back(current_mouse_pos, key_down);
                return PieMenuResponse::None;
            }
        }

        // The ring one level up, for the main ring fading out after a break-out.
        let parent: &[PieButton] = match self.path.split_last() {
            Some((_, up)) => Self::resolve_level(buttons, up).unwrap_or(&[]),
            None => &[],
        };
        let response = match self.show_level(ctx, level, parent, current_mouse_pos, key_down, &mut render_button) {
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
            LevelEvent::ClusterPick(button, item) => {
                self.selected_path = self.path.clone();
                self.selected_path.extend([button, item]);
                PieMenuResponse::Selected(item)
            }
            LevelEvent::CloseFan => {
                self.dismiss_fan();
                PieMenuResponse::None
            }
            LevelEvent::BreakOut(trigger) => {
                self.break_out(ctx, trigger, current_mouse_pos, key_down);
                PieMenuResponse::None
            }
            LevelEvent::FanPick(trigger, child) => {
                let is_back = level.get(trigger).and_then(|b| b.children().get(child)).is_some_and(|b| b.is_back());
                if is_back {
                    self.dismiss_fan();
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
                } else {
                    self.close_fan();
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
    /// chosen button (the last element is the index within its level). For an item of a
    /// cluster the last element is the item number and the one before it is the cluster
    /// button's index within its level: `[.., cluster_button_index, n]`.
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

    /// Whether keys (Backspace, dismiss keys, number keys, mnemonics) belong to the menu:
    /// always for a blocking menu, otherwise only while egui does not want the keyboard
    /// for a focused text field.
    fn keys_for_menu(&self, ctx: &egui::Context) -> bool {
        (self.settings.block_background && !self.pinned) || !ctx.egui_wants_keyboard_input()
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
        self.entered_forward = true;
    }

    /// Breaks the open fan of button `trigger` out into a sub-pie level, in place: the
    /// children stay exactly where the fan drew them (same centre, radius, Area ids and
    /// cached sizes) and the main ring, the opener included, fades out (see [`BreakOut`]).
    /// The parent's state is kept while the level is open so that going back restores it.
    fn break_out(&mut self, ctx: &egui::Context, trigger: usize, mouse: Option<Pos2>, key_down: bool) {
        if self.breakout.is_some() {
            // A fan inside a broken-out level: no transition, an ordinary sub-pie.
            self.close_fan();
            self.descend(trigger, mouse, key_down);
            return;
        }
        let now = ctx.input(|i| i.time);
        let parent = BreakOut {
            started: now,
            parent_center: self.display_center,
            parent_radius: self.level_radius(),
            parent_sizes: std::mem::replace(&mut self.button_sizes, std::mem::take(&mut self.fan_sizes)),
            opener: trigger,
        };
        let (center, radius) = (self.fan_display_center, self.fan_display_radius);
        self.close_fan();
        self.level_centers.push(self.position);
        self.path.push(trigger);
        self.restart_level(Some(center), key_down);
        self.entered_forward = true;
        // Fully revealed at once: no sliding, no fading in of the children.
        self.shown_at = Some(now - 1.0e6);
        self.level_radius = Some(radius);
        self.breakout = Some(parent);
        // The parent's dimming must not ease back up under the new centre.
        ctx.animate_bool_with_time(self.id.with(("fan_dim", self.anim_gen)), false, 0.0);
    }

    /// Forgets a break-out in progress (the level shown is an ordinary one again).
    fn clear_breakout(&mut self) {
        self.breakout = None;
        self.level_radius = None;
    }

    /// The radius of the level being shown.
    fn level_radius(&self) -> f32 {
        self.level_radius.unwrap_or(self.settings.layout_radius)
    }

    /// Returns to the parent level of the open sub-pie.
    fn go_back(&mut self, mouse: Option<Pos2>, key_down: bool) {
        let broken_out = self.breakout.take();
        let left = self.path.pop();
        let parent_center = self.level_centers.pop();
        let center = match self.settings.sub_pie_center {
            _ if broken_out.is_some() => parent_center,
            SubPieCenter::Pointer if !self.pinned => mouse,
            _ => parent_center,
        };
        if let Some(b) = broken_out {
            self.button_sizes = b.parent_sizes;
        }
        self.restart_level(center, key_down);
        // The pointer may still rest on the button just left: it must not fan out again.
        self.fan_suppressed = left;
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
        self.close_fan();
        self.clear_breakout();
        self.level_fresh = true;
        self.inert_hover = None;
        self.entered_forward = false;
        self.checked_buttons = false;
    }

    /// Shows the (new) level immediately, centred at `center` (`None`: where the menu is).
    fn restart_level(&mut self, center: Option<Pos2>, key_down: bool) {
        if let Some(p) = center {
            self.position = p;
        }
        self.display_center = self.position;
        self.hover_since = None;
        self.close_fan();
        self.clear_breakout();
        self.last_fan = None;
        self.fan_suppressed = None;
        self.level_fresh = true;
        self.inert_hover = None;
        self.entered_forward = false;
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
        // `f32::clamp` panics when the range is empty: a ring that does not fit the screen
        // (a small window, a big margin) is centred on the screen along that axis instead.
        let axis = |v: f32, lo: f32, hi: f32| {
            if lo <= hi {
                v.clamp(lo, hi)
            } else if ((lo + hi) / 2.0).is_finite() {
                (lo + hi) / 2.0
            } else {
                v
            }
        };
        Pos2::new(
            axis(center.x, screen.min.x + margin - bb_min.x, screen.max.x - margin - bb_max.x),
            axis(center.y, screen.min.y + margin - bb_min.y, screen.max.y - margin - bb_max.y),
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
    ///
    /// Returns the button's index and, for a mnemonic of a cluster's item, its number.
    /// `items`: whether cluster items take part (not in a fan-out).
    fn key_pick(&self, ctx: &egui::Context, buttons: &[PieButton], items: bool) -> Option<(usize, Option<usize>)> {
        let mirror = self.settings.mirror_horizontal;
        if self.settings.input.use_number_keys {
            for (idx, button) in buttons.iter().enumerate().filter(|(_, b)| b.enabled && !b.is_cluster()) {
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
                    return Some((idx, None));
                }
            }
        }

        if self.settings.input.use_mnemonic_keys {
            let (mnemonics, item_mnemonics) = resolve_all(buttons, self.settings.input.auto_mnemonics);
            // Every key that can be pressed, in button order, items after their cluster's turn.
            let mut owners: Vec<(char, usize, Option<usize>)> = Vec::new();
            for (idx, button) in buttons.iter().enumerate().filter(|(_, b)| b.enabled) {
                if button.is_cluster() {
                    if !items {
                        continue;
                    }
                    for (item, m) in button.cluster_items().iter().zip(&item_mnemonics[idx]) {
                        if let (true, Some(m)) = (item.enabled, *m) {
                            owners.push((m, idx, Some(item.number)));
                        }
                    }
                } else if let Some(m) = mnemonics[idx] {
                    owners.push((m, idx, None));
                }
            }
            for (m, idx, item) in owners {
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
                    return Some((idx, item));
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
    ) -> Vec<ItemHit> {
        let items = std::cell::RefCell::new(Vec::new());
        let mirror = self.settings.mirror_horizontal;
        let shape_factor = self.settings.shape_factor;
        // Resolved once per frame for the view handed to the render closure.
        let (mnemonics, item_mnemonics) = resolve_all(level.buttons, self.settings.input.auto_mnemonics);
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

            // Hover emphasis, part 1: the eased amount, which scales the button's content
            // through the font size (see `ButtonView::scale`).
            let emphasis = ctx.animate_bool_with_time(
                level.id.with((self.anim_gen, level.path_prefix, idx, "hover")),
                is_hovered && !level.emphasis.is_none(),
                anim.hover_duration.as_secs_f32(),
            );
            let scale = 1.0 + (level.emphasis.scale.max(0.0) - 1.0) * emphasis;

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
                    if level.opacity_scale != 1.0 {
                        ui.multiply_opacity(level.opacity_scale);
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
                        opens_fan: self.opens_fan(button),
                        scale,
                        items: &items,
                        item_mnemonics: &item_mnemonics[idx],
                    })
                });

            sizes[idx] = response.response.rect.size();

            // Hover emphasis, part 2: shift the layer outward. Only translated, never
            // scaled (a layer scale would resample the glyph images and blur the text),
            // and snapped to whole physical pixels (fractional offsets blur them too).
            // Layer transforms are sticky, so the identity is set again once eased back.
            let transform = if emphasis > 0.0 {
                let outward = (final_center - level.center).normalized();
                let outward = if outward.is_finite() { outward } else { Vec2::ZERO };
                let ppp = ctx.pixels_per_point();
                let snap = |v: f32| (v * ppp).round() / ppp;
                let shift = outward * level.emphasis.shift * emphasis;
                TSTransform::from_translation(Vec2::new(snap(shift.x), snap(shift.y)))
            } else {
                TSTransform::IDENTITY
            };
            ctx.set_transform_layer(response.response.layer_id, transform);
        }
        items.into_inner()
    }

    fn show_level(
        &mut self,
        ctx: &egui::Context,
        buttons: &[PieButton],
        parent: &[PieButton],
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
        if blocking {
            ctx.memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
        }
        let keys_for_menu = self.keys_for_menu(ctx);

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
        let radius = self.level_radius();
        let dead_zone = self.settings.dead_zone_radius;

        // Shift the whole menu so every button stays within the screen rect with a margin.
        let screen_rect = ctx.content_rect();
        let center = self.clamp_center(center, buttons, &self.button_sizes, radius, screen_rect);
        self.display_center = center;

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
            self.fan_display_radius = fan_radius;
            let fan_center = self.clamp_center(button_center, children, &self.fan_sizes, fan_radius, screen_rect);
            self.fan_display_center = fan_center;
            Fan {
                trigger,
                children,
                center: fan_center,
                radius: fan_radius,
                trigger_rect: Rect::from_center_size(button_center, size),
            }
        });

        // The parent's centre (circle, highlight, title) and its other buttons are greyed out
        // while a fan is open, easing in and out with it.
        // After a break-out it stays at rest: the main ring and old centre fade out instead.
        let dim_time = if self.breakout.is_some() { 0.0 } else { 0.12 };
        let dim = ctx.animate_bool_with_time(self.id.with(("fan_dim", self.anim_gen)), fan.is_some(), dim_time);
        let dim_opacity = 1.0 + (DIMMED_OPACITY - 1.0) * dim;

        // A break-out's fades: the old centre and main ring out, the new centre in. One
        // linear progress for all, whatever the reveal style.
        let reveal = &self.settings.animations;
        let fade = self.breakout.as_ref().map_or(1.0, |b| match reveal.reveal {
            RevealAnimation::None => 1.0,
            _ => ((now - b.started) as f32 / reveal.reveal_duration.as_secs_f32().max(f32::EPSILON)).clamp(0.0, 1.0),
        });
        if fade < 1.0 {
            ctx.request_repaint();
        }
        let mut painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, self.id));
        painter.multiply_opacity(dim_opacity * fade);
        if let Some(b) = self.breakout.as_ref().filter(|_| fade < 1.0)
            && let Some(circle_radius) = self.settings.center_indicator.background_radius
        {
            let mut old = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, self.id));
            old.multiply_opacity(DIMMED_OPACITY * (1.0 - fade));
            old.circle(
                b.parent_center,
                circle_radius,
                self.settings.center_indicator.background_fill_color,
                self.settings.center_indicator.background_stroke,
            );
        }

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
                    if let Some((child, _)) = self.key_pick(ctx, fan.children, false) {
                        return LevelEvent::FanPick(fan.trigger, child);
                    }
                }
                None => {
                    match self.key_pick(ctx, buttons, true) {
                        Some((idx, Some(item))) => return LevelEvent::ClusterPick(idx, item),
                        Some((idx, None)) => return LevelEvent::Pick(idx),
                        None => {}
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
                    let angle = v.y.atan2(v.x);
                    let back = center - fan.center;
                    // Pointing back at the parent picks nothing, unless the pointer is right
                    // on a child (one pushed into that sector by `intercardinal_height`).
                    let nearest_child = fan_dirs
                        .iter()
                        .zip(fan.children)
                        .filter(|(_, child)| child.enabled)
                        .map(|(d, _)| Self::angle_diff(angle, d.y.atan2(d.x)))
                        .fold(f32::MAX, f32::min);
                    let blocked = Self::angle_diff(angle, back.y.atan2(back.x)) < 67.5f32.to_radians()
                        && nearest_child > 22.5f32.to_radians();
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
                if self.fan_suppressed.is_some() && !(reached && level_pick == self.fan_suppressed) {
                    self.fan_suppressed = None; // the pointer has left that button
                }
                match level_pick.filter(|&i| reached && self.fan_suppressed != Some(i) && self.opens_fan(&buttons[i])) {
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
                            repaint_after(ctx, remaining);
                        }
                    }
                    None => self.fan_pending = None,
                }
            }
        }

        // Hover timer (replace-style sub-pies): resting on a button that leads to another
        // pie (a sub-pie button, or a back button inside a sub-pie) long enough goes there.
        // The button the pointer already points at when a level opens is ignored until the
        // pointer leaves it, so a resting pointer cannot bounce between levels: after going
        // back, whatever it rests on; after descending, a back button (the way it came).
        // A sub-pie button under the pointer after descending is not ignored: the timer
        // starts at once and navigation flows through the levels.
        if std::mem::take(&mut self.level_fresh) {
            let forward = self.entered_forward;
            self.inert_hover = self.hovered_index.filter(|&i| !forward || buttons[i].is_back());
        } else if self.inert_hover != self.hovered_index {
            self.inert_hover = None;
        }
        let mut timer_pick = None;
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
                        // Decided now, acted on once this frame's buttons are drawn: the
                        // level then changes without a frame that shows no buttons.
                        self.hover_since = None;
                        timer_pick = Some(i);
                    } else {
                        repaint_after(ctx, remaining);
                    }
                }
                (Some(i), _) => {
                    self.hover_since = Some((i, now));
                    repaint_after(ctx, delay.as_secs_f64());
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
        let cluster_items = self.draw_level(
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
                // A broken-out level keeps the Areas the fan drew its buttons in.
                id: self.breakout.as_ref().map_or(self.id, |b| self.id.with(("fan", b.opener))),
                path_prefix: &path,
                opacity_scale: 1.0,
            },
            &mut sizes,
            Some(&mut centers),
            render_button,
        );
        self.button_sizes = sizes;
        self.button_centers = centers;

        // After a break-out: the whole main ring, the opener included (lit), fades out where
        // it was; once the fade is over nothing of it is drawn any more.
        if let Some(mut b) = self.breakout.take() {
            if fade < 1.0 && b.opener < parent.len() {
                b.parent_sizes.resize(parent.len(), Vec2::new(50.0, 24.0));
                let up = &path[..path.len().saturating_sub(1)];
                let draw = LevelDraw {
                    buttons: parent,
                    center: b.parent_center,
                    radius: b.parent_radius,
                    shown_at: None,
                    hovered: Some(b.opener),
                    dim_except: Some(b.opener),
                    dim_opacity: DIMMED_OPACITY,
                    emphasis: if up.is_empty() {
                        self.settings.animations.hover_emphasis
                    } else {
                        self.settings.animations.sub_pie_hover_emphasis
                    },
                    id: self.id,
                    path_prefix: up,
                    opacity_scale: 1.0 - fade,
                };
                self.draw_level(ctx, now, &draw, &mut b.parent_sizes, None, render_button);
            }
            self.breakout = Some(b);
        }

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
                    opacity_scale: 1.0,
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

        if let Some(i) = timer_pick {
            return LevelEvent::Pick(i);
        }

        // A cluster item under the pointer: it wins over the angle-based pick (an item can
        // reach into another button's sector). Not while a fan is open, which has the
        // pointer to itself.
        let item_at = |p: Pos2, clicked_only: bool| {
            cluster_items
                .iter()
                .find(|it| {
                    it.enabled
                        && (it.clicked || !clicked_only)
                        && it.rect.contains(p)
                        && buttons.get(it.button).is_some_and(|b| {
                            // A declared item can also be disabled by its declaration.
                            b.enabled
                                && b.is_cluster()
                                && b.cluster_items().iter().all(|d| d.number != it.n || d.enabled)
                        })
                })
                .map(|it| LevelEvent::ClusterPick(it.button, it.n))
        };

        // Primary click selection — runs after render so closures (e.g. checkboxes)
        // receive the click event before we consume it.
        let opener = self.settings.fan_out_opener_action;
        if self.settings.input.select_on_primary_click && ctx.input(|i| i.pointer.primary_clicked())
        {
            if fan.is_none()
                && let Some(event) = current_mouse_pos.and_then(|p| item_at(p, true))
            {
                return event;
            }
            return match (current_mouse_pos, &fan) {
                // In an open fan a click selects the highlighted child, if any.
                // A click on the opening button breaks the fan out (or closes it, as set);
                // a click on nothing else (the blocked sector) closes the fan.
                (Some(p), Some(fan)) => match self.fan_hover {
                    Some(child) => LevelEvent::FanPick(fan.trigger, child),
                    None if fan.trigger_rect.contains(p) && opener == FanOpenerAction::BreakOut => LevelEvent::BreakOut(fan.trigger),
                    None => LevelEvent::CloseFan,
                },
                (Some(p), None) if (p - center).length() > dead_zone => {
                    self.hovered_index.map_or(LevelEvent::None, LevelEvent::Pick)
                }
                // A click in the centre of the root dismisses (in a sub-pie it went back a
                // level before this point); a pinned menu is never dismissed by the pointer.
                (Some(_), None) if self.path.is_empty() && !self.pinned => LevelEvent::Dismissed,
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
                // Released on a fan child: select it. Released on the fan's button: break
                // out into a sub-pie, or cancel, as set. Released over nothing: cancel, as
                // releasing in the centre does.
                if let Some(child) = self.fan_hover {
                    return LevelEvent::FanPick(fan.trigger, child);
                }
                if current_mouse_pos.is_some_and(|p| fan.trigger_rect.contains(p)) && opener == FanOpenerAction::BreakOut {
                    return LevelEvent::BreakOut(fan.trigger);
                }
                return LevelEvent::Dismissed;
            } else if let Some(mouse_pos) = current_mouse_pos {
                if let Some(event) = item_at(mouse_pos, false) {
                    return event;
                }
                if (mouse_pos - center).length() <= dead_zone {
                    return LevelEvent::Dismissed;
                } else if let Some(idx) = self.hovered_index {
                    return LevelEvent::Pick(idx);
                }
                // Nothing to select (every button disabled): cancel rather than strand the menu.
                return LevelEvent::Dismissed;
            } else {
                // No pointer at release (it left the window): cancel.
                return LevelEvent::Dismissed;
            }
        }

        LevelEvent::None
    }
}
