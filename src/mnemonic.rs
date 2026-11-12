//! Mnemonic keys: resolving which key each button gets, matching typed characters,
//! and underlining the mnemonic in a label (including right-to-left scripts).

use crate::PieButton;
// Everything else at the crate root, for the doc links below.
#[allow(unused_imports)]
use crate::*;
use egui::text::{LayoutJob, TextFormat};
use egui::{Key, Stroke};

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

/// Resolves the mnemonic key in effect for each button.
///
/// - [`Mnemonic::Key`] is used as given, and its key is reserved.
/// - [`Mnemonic::None`] never gets one, nor does a cluster button itself; the items of a
///   cluster take part like buttons (see [`resolve_all`]).
/// - [`Mnemonic::Auto`] gets one generated from its label when `auto` is on. All
///   buttons are considered together: a button whose letters are all claimed makes
///   another button move to one of its alternative letters, so as many buttons as
///   possible get a distinct mnemonic. Word initials are preferred, then the other
///   letters left to right. Only a button with no possible letter is left without one.
///   Any alphabetic letter of any script can be generated (combining marks are skipped;
///   Hebrew final forms and Arabic alef variants are folded to one key).
pub(crate) fn resolve_mnemonics(buttons: &[PieButton], auto: bool) -> Vec<Option<char>> {
    resolve_all(buttons, auto).0
}

/// One thing that can own a mnemonic: a button, or an item of a cluster.
struct Slot<'a> {
    mnemonic: Mnemonic,
    label: &'a str,
    /// Whether it may be given a generated key.
    enabled: bool,
}

/// [`resolve_mnemonics`] for the buttons and the declared items of the clusters among
/// them, all competing for the same keys: one entry per button (`None` for a cluster),
/// and per cluster the entries of its items in declaration order (empty for other
/// buttons). A disabled item, or any item of a disabled cluster, gets no key and reserves
/// none.
pub(crate) fn resolve_all(buttons: &[PieButton], auto: bool) -> (Vec<Option<char>>, Vec<Vec<Option<char>>>) {
    let mut slots: Vec<Slot> = Vec::new();
    // Where each button's slot (or its items' slots) start.
    let mut starts = Vec::with_capacity(buttons.len());
    for b in buttons {
        starts.push(slots.len());
        if b.is_cluster() {
            for item in b.cluster_items() {
                let live = b.enabled && item.enabled;
                slots.push(Slot {
                    mnemonic: if live { item.mnemonic } else { Mnemonic::None },
                    label: &item.label,
                    enabled: live,
                });
            }
        } else {
            slots.push(Slot { mnemonic: b.mnemonic, label: &b.label, enabled: b.enabled });
        }
    }
    let resolved = resolve_slots(&slots, auto);
    let mut per_button = Vec::with_capacity(buttons.len());
    let mut per_item = Vec::with_capacity(buttons.len());
    for (b, &start) in buttons.iter().zip(&starts) {
        if b.is_cluster() {
            per_button.push(None);
            per_item.push(resolved[start..start + b.cluster_items().len()].to_vec());
        } else {
            per_button.push(resolved[start]);
            per_item.push(Vec::new());
        }
    }
    (per_button, per_item)
}

fn resolve_slots(buttons: &[Slot], auto: bool) -> Vec<Option<char>> {
    let mut resolved: Vec<Option<char>> = buttons
        .iter()
        .map(|b| match b.mnemonic {
            Mnemonic::Key(c) => Some(fold_char(c)),
            _ => None,
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
            if button.mnemonic == Mnemonic::Auto && button.enabled {
                let label = button.label;
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
            if let std::collections::hash_map::Entry::Vacant(free) = owner.entry(c) {
                free.insert(i);
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

/// Build a [`LayoutJob`] that renders `text` with the first case-insensitive
/// occurrence of `mnemonic` underlined — suitable for use as a pie menu button
/// label when you want to surface keyboard shortcuts visually.
///
/// The rest of the text uses `format` unchanged. The underline color is
/// derived from `format.color`.  If `mnemonic` does not appear in `text`
/// the whole string is rendered with `format` and no underline.
///
/// # Example
/// ```
/// use egui_pie_menu::{FontId, TextFormat, mnemonic_text};
///
/// let format = TextFormat { color: egui::Color32::WHITE, font_id: FontId::default(), ..Default::default() };
/// let job = mnemonic_text("Copy", 'c', format); // pass it to `ui.label(job)`
/// assert_eq!(job.text, "Copy");
/// assert_eq!(job.sections.len(), 2); // underlined "C" + "opy"
/// ```
pub fn mnemonic_text(text: &str, mnemonic: char, format: TextFormat) -> LayoutJob {
    let mut job = LayoutJob::default();
    append_mnemonic_text(&mut job, text, mnemonic, format);
    job
}

pub(crate) fn append_mnemonic_text(job: &mut LayoutJob, text: &str, mnemonic: char, format: TextFormat) {
    let mnemonic_folded = fold_char(mnemonic);
    let Some((idx, c)) = text.char_indices().find(|(_, c)| fold_char(*c) == mnemonic_folded) else {
        job.append(text, 0.0, format);
        return;
    };
    // The underlined piece keeps the combining marks (accents, vowel points) of its letter.
    let mut m_end = idx + c.len_utf8();
    for mark in text[m_end..].chars().take_while(|&m| is_combining_mark(m)) {
        m_end += mark.len_utf8();
    }
    let underline_format = TextFormat { underline: Stroke::new(1.0, format.color), ..format.clone() };

    let mut append = |piece: &str, format: &TextFormat| {
        if !piece.is_empty() {
            job.append(piece, 0.0, format.clone());
        }
    };
    if !is_rtl_letter(c) {
        append(&text[..idx], &format);
        append(&text[idx..m_end], &underline_format);
        append(&text[m_end..], &format);
        return;
    }

    // Right-to-left letter. egui has no bidi: it reverses each run of right-to-left
    // letters in place (punctuation and spaces stay put) but lays separately formatted
    // pieces out left to right in the order given. A run split for the underline would come
    // out with its pieces swapped (the mnemonic jumping to the far side), so the mnemonic's
    // run goes in reversed: after, letter, before.
    // Multi-word RTL labels still get egui's left-to-right word order; `unicode-bidi`
    // could supply the visual word order (see the README's RTL notes). Remove this whole
    // workaround once egui does bidi itself (the RTL render test should start failing).
    let in_run = |ch: char| is_rtl_letter(ch) || is_combining_mark(ch);
    let ws = text[..idx].char_indices().rev().take_while(|(_, ch)| in_run(*ch)).last().map_or(idx, |(i, _)| i);
    let we = text[m_end..].char_indices().find(|(_, ch)| !in_run(*ch)).map_or(text.len(), |(i, _)| m_end + i);
    let (head, word_before, letter, word_after, tail) =
        (&text[..ws], &text[ws..idx], &text[idx..m_end], &text[m_end..we], &text[we..]);
    // In joining scripts (Arabic, Persian) a zero-width joiner on both sides of each cut
    // keeps the letters' connected forms.
    const ZWJ: char = '\u{200D}';
    let joins = |a: Option<char>, b: Option<char>| matches!((a, b), (Some(a), Some(b)) if is_joining_letter(a) && is_joining_letter(b));
    let join_before = joins(word_before.chars().last(), Some(c));
    let join_after = joins(Some(c), word_after.chars().next());
    let zwj = |on: bool| if on { ZWJ.to_string() } else { String::new() };

    append(head, &format);
    append(&format!("{}{word_after}", zwj(join_after && !word_after.is_empty())), &format);
    append(&format!("{}{letter}{}", zwj(join_before), zwj(join_after)), &underline_format);
    append(&format!("{word_before}{}", zwj(join_before && !word_before.is_empty())), &format);
    append(tail, &format);
}

/// Letters of right-to-left scripts (Hebrew, Arabic, Syriac, Thaana, N'Ko and their
/// presentation forms).
pub(crate) fn is_rtl_letter(c: char) -> bool {
    c.is_alphabetic()
        && matches!(c as u32, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF)
}

/// Letters of the Arabic script (Arabic, Persian, Urdu, …), which join to their neighbours.
pub(crate) fn is_joining_letter(c: char) -> bool {
    c.is_alphabetic()
        && matches!(c as u32, 0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF)
}

/// Case-folds `c` and normalises letter variants that share a key:
/// Hebrew final forms (ך→כ …) and Arabic alef variants (أ إ آ ٱ → ا).
pub(crate) fn fold_char(c: char) -> char {
    let c = c.to_lowercase().next().unwrap_or(c);
    match c {
        'ך' => 'כ',
        'ם' => 'מ',
        'ן' => 'נ',
        'ף' => 'פ',
        'ץ' => 'צ',
        'أ' | 'إ' | 'آ' | 'ٱ' => 'ا',
        // Persian keyboards type Farsi yeh/keheh where Arabic ones type yeh/kaf.
        'ی' | 'ى' => 'ي',
        'ک' => 'ك',
        _ => c,
    }
}

/// Combining marks (Arabic/Hebrew points, accents) are never mnemonic candidates.
pub(crate) fn is_combining_mark(c: char) -> bool {
    matches!(c as u32,
        0x0300..=0x036F | 0x0591..=0x05BD | 0x05BF | 0x05C1..=0x05C2 | 0x05C4..=0x05C5 | 0x05C7
        | 0x0610..=0x061A | 0x064B..=0x065F | 0x0670 | 0x06D6..=0x06DC | 0x06DF..=0x06E4
        | 0x06E7..=0x06E8 | 0x06EA..=0x06ED)
}

pub(crate) fn is_mnemonic_letter(c: char) -> bool {
    c.is_alphabetic() && !is_combining_mark(c)
}

/// The egui key of an ASCII letter, for matching mnemonics by key code.
pub(crate) fn char_to_key(c: char) -> Option<Key> {
    c.is_ascii_alphabetic().then(|| Key::from_name(c.to_ascii_uppercase().encode_utf8(&mut [0; 4]))).flatten()
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use super::*;
    use crate::{button::*, highlight_shapes::*, mnemonic::*, *};
    use egui::{Color32, FontId, Key, Pos2, Rect, Stroke, Vec2};

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

    mod rtl_mnemonic_tests {
        use super::*;

        fn fmt() -> TextFormat {
            TextFormat { font_id: FontId::proportional(20.0), color: Color32::WHITE, ..Default::default() }
        }

        fn underlined(job: &LayoutJob) -> String {
            job.sections.iter().filter(|s| s.format.underline != Stroke::NONE)
                .map(|s| job.text[usize::from(s.byte_range.start)..usize::from(s.byte_range.end)].to_string()).collect()
        }

        #[test]
        fn rtl_word_pieces_are_reversed_other_words_untouched() {
            // "גלישת שורות", mnemonic ל in the first word: that word goes in as ישת | ל | ג.
            let job = mnemonic_text("גלישת שורות", 'ל', fmt());
            assert_eq!(job.text, "ישתלג שורות");
            assert_eq!(underlined(&job), "ל");
            // Latin text is unchanged.
            let job = mnemonic_text("Paste Special", 's', fmt());
            assert_eq!(job.text, "Paste Special");
            assert_eq!(underlined(&job), "s");
        }

        #[test]
        fn joining_scripts_get_zero_width_joiners_at_the_cuts() {
            let job = mnemonic_text("نسخ", 'س', fmt());
            assert_eq!(job.text, "\u{200D}خ\u{200D}س\u{200D}ن\u{200D}");
            assert_eq!(underlined(&job), "\u{200D}س\u{200D}");
            // Hebrew doesn't join: no joiners.
            assert!(!mnemonic_text("העתק", 'ע', fmt()).text.contains('\u{200D}'));
        }

        #[test]
        fn combining_marks_stay_with_the_underlined_letter() {
            let job = mnemonic_text("café", 'e', fmt()); // precomposed é is its own letter: no match
            assert_eq!(underlined(&job), "");
            let job = mnemonic_text("cafe\u{0301}s", 'e', fmt());
            assert_eq!(underlined(&job), "e\u{0301}");
        }

        /// Renders every demo RTL label with every mnemonic letter and checks egui draws each
        /// letter exactly where it draws it in the unsplit label (same width: joins intact).
        /// Needs system fonts with Hebrew/Arabic; skipped if none are installed.
        #[test]
        fn rtl_mnemonics_render_like_the_unsplit_label() {
            let mut fonts = egui::FontDefinitions::default();
            let mut found = 0;
            for path in [
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "/usr/share/fonts/truetype/noto/NotoSansArabic-Regular.ttf",
                "/usr/share/fonts/truetype/noto/NotoSansHebrew-Regular.ttf",
            ] {
                if let Ok(bytes) = std::fs::read(path) {
                    found += 1;
                    fonts.font_data.insert(path.into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
                    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().push(path.into());
                }
            }
            if found == 0 {
                eprintln!("skipped: no Hebrew/Arabic system fonts");
                return;
            }
            let ctx = egui::Context::default();
            ctx.set_fonts(fonts);
            let mut out = ctx.run_ui(Default::default(), |_| {});
            out.textures_delta.clear();
            let layout = |job: LayoutJob| {
                let g = ctx.fonts_mut(|f| f.layout_job(job));
                for gl in g.rows.iter().flat_map(|r| r.glyphs.iter()).filter(|gl| gl.chr == '\u{200D}') {
                    assert_eq!(gl.advance_width, 0.0, "zero-width joiner is visible");
                }
                let mut v: Vec<(f32, char)> = g.rows.iter().flat_map(|r| r.glyphs.iter())
                    .filter(|gl| gl.advance_width > 0.0).map(|gl| (gl.pos.x, gl.chr)).collect();
                v.sort_by(|a, b| a.0.total_cmp(&b.0));
                (v.into_iter().map(|(_, c)| c).collect::<String>(), g.rect.width())
            };
            let labels = [
                "העתק", "הדבק מיוחד", "בצע שוב", "שמור בשם…", "מחק לצמיתות", "גזור", "בטל שינוי", "גלישת שורות",
                "نسخ", "لصق خاص", "إعادة", "حفظ باسم…", "حذف نهائي", "قص", "تراجع", "التفاف النص",
                "کپی", "چسباندن ویژه", "انجام مجدد", "ذخیره با نام…", "حذف دائمی", "برش", "واگرد", "شکستن خط",
            ];
            let mut worst = 0.0f32;
            for label in labels {
                let mut plain = LayoutJob::default();
                plain.append(label, 0.0, fmt());
                let (target, width) = layout(plain);
                for m in label.chars().filter(|c| is_mnemonic_letter(*c)) {
                    let job = mnemonic_text(label, m, fmt());
                    // The same pieces without joiners: what broken joins would look like.
                    let mut broken = LayoutJob::default();
                    for sec in &job.sections {
                        let piece = &job.text[usize::from(sec.byte_range.start)..usize::from(sec.byte_range.end)];
                        broken.append(&piece.replace('\u{200D}', ""), 0.0, sec.format.clone());
                    }
                    let (got, w) = layout(job);
                    assert_eq!(got, target, "{label} with mnemonic {m}");
                    let off = (w - width).abs();
                    // A cut can't be kerned across (up to ~2 px); broken joins usually cost far
                    // more (e.g. لصق خاص: −1.5 px with joiners, +14 px without). Some letters'
                    // joined and unjoined forms are nearly equally wide, hence the 2 px floor.
                    let (_, broken_w) = layout(broken);
                    let broken_off = (broken_w - width).abs();
                    assert!(off < 2.0 || off < broken_off / 3.0,
                        "{label} with mnemonic {m}: width off by {off:.2} px (broken joins: {broken_off:.2} px)");
                    worst = worst.max(off);
                }
            }
            eprintln!("worst width difference: {worst:.2} px");
            assert!(worst < 3.0, "worst width difference {worst:.2} px");
        }
    }
}
