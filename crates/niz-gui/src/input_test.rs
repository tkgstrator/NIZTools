use eframe::egui::{self, Event, Key};
use niz_core::Profile;
use std::collections::BTreeSet;

#[derive(Default)]
pub(crate) struct InputTest {
    pub text: String,
    pub layer: usize,
    held: BTreeSet<u8>,
    modifiers: egui::Modifiers,
    precise_modifiers: [bool; 4],
    reference: Option<Profile>,
}
impl InputTest {
    pub fn clear_held(&mut self) {
        self.held.clear();
        self.modifiers = Default::default();
        self.precise_modifiers = [false; 4];
    }
    pub fn reference(&mut self, profile: Option<&Profile>) {
        let unchanged = match (&self.reference, profile) {
            (Some(a), Some(b)) => {
                a.identity == b.identity && a.firmware == b.firmware && a.records == b.records
            }
            (None, None) => true,
            _ => false,
        };
        if !unchanged {
            self.clear_held();
            self.reference = profile.cloned();
        }
    }
    pub fn event(&mut self, event: &Event) {
        match event {
            Event::Text(text) => {
                self.text.push_str(text);
                if self.text.chars().count() > 256 {
                    self.text = self
                        .text
                        .chars()
                        .rev()
                        .take(256)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect();
                }
            }
            Event::Key {
                key,
                physical_key,
                pressed,
                modifiers,
                ..
            } => {
                if let Some(code) = code(physical_key.unwrap_or(*key)) {
                    if *pressed && let Some(family) = modifier_family(code) {
                        self.precise_modifiers[family] = true;
                    }
                    if *pressed {
                        self.held.insert(code);
                    } else {
                        self.held.remove(&code);
                    }
                    if modifier_family(code).is_none() {
                        self.aggregate(*modifiers);
                    }
                }
            }
            Event::ModifiersChanged(modifiers) => self.aggregate(*modifiers),
            Event::WindowFocused(false) => self.clear_held(),
            _ => {}
        }
    }
    fn aggregate(&mut self, modifiers: egui::Modifiers) {
        self.modifiers = modifiers;
        for (family, active) in [
            modifiers.shift,
            modifiers.ctrl,
            modifiers.alt,
            modifiers.mac_cmd || (modifiers.command && !modifiers.ctrl),
        ]
        .into_iter()
        .enumerate()
        {
            if !active {
                self.held
                    .retain(|code| modifier_family(*code) != Some(family));
                self.precise_modifiers[family] = false;
            }
        }
    }
    pub fn codes(&self) -> (BTreeSet<u8>, bool) {
        let mut codes = self.held.clone();
        let mut ambiguous = false;
        for (family, (active, pair)) in [
            (self.modifiers.shift, [55, 66]),
            (self.modifiers.ctrl, [67, 74]),
            (self.modifiers.alt, [69, 71]),
            (
                self.modifiers.mac_cmd || (self.modifiers.command && !self.modifiers.ctrl),
                [68, 72],
            ),
        ]
        .into_iter()
        .enumerate()
        {
            // A precise release remains authoritative until the aggregate
            // family goes inactive; its prior flag cannot revive both sides.
            if active && !self.precise_modifiers[family] {
                codes.extend(pair);
                ambiguous = true;
            }
        }
        (codes, ambiguous)
    }
    pub fn matches(&self, profile: &Profile) -> BTreeSet<usize> {
        let codes = self.codes().0;
        (0..66)
            .filter(|slot| {
                profile
                    .definition(self.layer * 66 + slot)
                    .is_ok_and(|d| d.kind == 0 && d.keys.len() == 1 && codes.contains(&d.keys[0]))
            })
            .collect()
    }
}
fn modifier_family(code: u8) -> Option<usize> {
    match code {
        55 | 66 => Some(0),
        67 | 74 => Some(1),
        69 | 71 => Some(2),
        68 | 72 => Some(3),
        _ => None,
    }
}
fn code(key: Key) -> Option<u8> {
    Some(match key {
        Key::Escape => 1,
        Key::F1 => 2,
        Key::F2 => 3,
        Key::F3 => 4,
        Key::F4 => 5,
        Key::F5 => 6,
        Key::F6 => 7,
        Key::F7 => 8,
        Key::F8 => 9,
        Key::F9 => 10,
        Key::F10 => 11,
        Key::F11 => 12,
        Key::F12 => 13,
        Key::Backtick => 14,
        Key::Num1 => 15,
        Key::Num2 => 16,
        Key::Num3 => 17,
        Key::Num4 => 18,
        Key::Num5 => 19,
        Key::Num6 => 20,
        Key::Num7 => 21,
        Key::Num8 => 22,
        Key::Num9 => 23,
        Key::Num0 => 24,
        Key::Minus => 25,
        Key::Equals | Key::Plus => 26,
        Key::Backspace => 27,
        Key::Tab => 28,
        Key::Q => 29,
        Key::W => 30,
        Key::E => 31,
        Key::R => 32,
        Key::T => 33,
        Key::Y => 34,
        Key::U => 35,
        Key::I => 36,
        Key::O => 37,
        Key::P => 38,
        Key::OpenBracket | Key::OpenCurlyBracket => 39,
        Key::CloseBracket | Key::CloseCurlyBracket => 40,
        Key::Backslash | Key::Pipe => 41,
        Key::A => 43,
        Key::S => 44,
        Key::D => 45,
        Key::F => 46,
        Key::G => 47,
        Key::H => 48,
        Key::J => 49,
        Key::K => 50,
        Key::L => 51,
        Key::Semicolon | Key::Colon => 52,
        Key::Quote => 53,
        Key::Enter => 54,
        Key::ShiftLeft => 55,
        Key::Z => 56,
        Key::X => 57,
        Key::C => 58,
        Key::V => 59,
        Key::B => 60,
        Key::N => 61,
        Key::M => 62,
        Key::Comma => 63,
        Key::Period => 64,
        Key::Slash | Key::Questionmark => 65,
        Key::ShiftRight => 66,
        Key::ControlLeft => 67,
        Key::SuperLeft => 68,
        Key::AltLeft => 69,
        Key::Space => 70,
        Key::AltRight => 71,
        Key::SuperRight => 72,
        Key::ControlRight => 74,
        Key::Insert => 81,
        Key::Home => 82,
        Key::PageUp => 83,
        Key::Delete => 84,
        Key::End => 85,
        Key::PageDown => 86,
        Key::ArrowUp => 87,
        Key::ArrowLeft => 88,
        Key::ArrowDown => 89,
        Key::ArrowRight => 90,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(key: Key, physical_key: Option<Key>, pressed: bool) -> Event {
        Event::Key {
            key,
            physical_key,
            pressed,
            repeat: false,
            modifiers: Default::default(),
        }
    }
    #[test]
    fn repeated_press_needs_one_release_and_physical_punctuation_survives_shift() {
        let mut t = InputTest::default();
        for _ in 0..3 {
            t.event(&key(Key::Questionmark, Some(Key::Slash), true));
        }
        assert_eq!(t.codes().0, BTreeSet::from([65]));
        t.event(&key(Key::Slash, Some(Key::Slash), false));
        assert!(t.codes().0.is_empty());
        t.event(&key(Key::Pipe, None, true));
        assert_eq!(t.codes().0, BTreeSet::from([41]));
        t.event(&key(Key::Backslash, None, false));
        assert!(t.codes().0.is_empty());
    }
    #[test]
    fn exact_modifier_sides_override_ambiguous_aggregate_and_release_independently() {
        let mut t = InputTest::default();
        t.event(&Event::ModifiersChanged(egui::Modifiers {
            shift: true,
            ..Default::default()
        }));
        assert_eq!(t.codes(), (BTreeSet::from([55, 66]), true));
        t.event(&key(Key::ShiftLeft, Some(Key::ShiftLeft), true));
        assert_eq!(t.codes(), (BTreeSet::from([55]), false));
        t.event(&key(Key::ShiftRight, Some(Key::ShiftRight), true));
        t.event(&key(Key::ShiftLeft, Some(Key::ShiftLeft), false));
        assert_eq!(t.codes(), (BTreeSet::from([66]), false));
        t.event(&Event::ModifiersChanged(Default::default()));
        assert!(t.codes().0.is_empty());
        for (k, code) in [
            (Key::ControlLeft, 67),
            (Key::ControlRight, 74),
            (Key::AltLeft, 69),
            (Key::AltRight, 71),
            (Key::SuperLeft, 68),
            (Key::SuperRight, 72),
        ] {
            t.clear_held();
            t.event(&key(k, Some(k), true));
            assert_eq!(t.codes().0, BTreeSet::from([code]));
        }
    }
    #[test]
    fn matching_uses_single_baseline_outputs_and_preserves_duplicate_candidates() {
        let mut p = Profile::demo();
        let a = p.records[30].clone();
        p.records[31] = a;
        p.records[31][0][3] = 32; // Keep the slot identity while sharing A's output.
        let mut t = InputTest::default();
        t.event(&key(Key::A, Some(Key::A), true));
        assert_eq!(t.matches(&p), BTreeSet::from([30, 31]));
        p.records[30][0][6] = 178;
        assert_eq!(t.matches(&p), BTreeSet::from([31]));
        p.records[31][0][6] = 156;
        assert!(t.matches(&p).is_empty());
        p.records[31][0][6] = 0;
        assert!(t.matches(&p).is_empty());
        p.records[31] = niz_core::encode_definition(
            31,
            &niz_core::Definition {
                kind: 2,
                keys: vec![43],
                interval: 0,
                cycles: 1,
                custom_delay: false,
                delays: vec![],
            },
        )
        .unwrap();
        assert_eq!(p.definition(31).unwrap().kind, 2);
        assert!(t.matches(&p).is_empty());
    }
    #[test]
    fn focus_loss_and_reference_change_clear_held_without_losing_local_unicode_text() {
        let mut t = InputTest::default();
        let p = Profile::demo();
        t.reference(Some(&p));
        t.event(&key(Key::A, Some(Key::A), true));
        t.event(&Event::Text("日本語🙂".repeat(100)));
        assert_eq!(t.text.chars().count(), 256);
        t.event(&Event::WindowFocused(false));
        assert!(t.codes().0.is_empty());
        t.event(&key(Key::S, Some(Key::S), true));
        let mut changed = p.clone();
        changed.records[30][0][6] = 44;
        t.reference(Some(&changed));
        assert!(t.codes().0.is_empty());
        assert_eq!(t.text.chars().count(), 256);
    }
    #[test]
    fn clipboard_ime_and_unreported_vendor_keys_never_create_a_physical_match() {
        let mut t = InputTest::default();
        for event in [
            Event::Copy,
            Event::Cut,
            Event::Paste("clipboard".into()),
            Event::Ime(egui::ImeEvent::Commit("ime".into())),
            key(Key::Copy, None, true),
            key(Key::F35, None, true),
        ] {
            t.event(&event);
        }
        assert!(t.codes().0.is_empty());
        assert!(t.text.is_empty());
    }
    #[test]
    fn precise_last_modifier_release_does_not_revive_stale_aggregate_candidates() {
        for (side, modifiers) in [
            (
                Key::ShiftLeft,
                egui::Modifiers {
                    shift: true,
                    ..Default::default()
                },
            ),
            (
                Key::ControlRight,
                egui::Modifiers {
                    ctrl: true,
                    command: true,
                    ..Default::default()
                },
            ),
            (
                Key::AltLeft,
                egui::Modifiers {
                    alt: true,
                    ..Default::default()
                },
            ),
            (
                Key::SuperRight,
                egui::Modifiers {
                    mac_cmd: true,
                    command: true,
                    ..Default::default()
                },
            ),
        ] {
            let mut t = InputTest::default();
            t.event(&Event::ModifiersChanged(modifiers));
            assert!(t.codes().1);
            t.event(&key(side, Some(side), true));
            assert_eq!(t.codes().0.len(), 1);
            t.event(&key(side, Some(side), false));
            assert!(
                t.codes().0.is_empty(),
                "last {side:?} release must clear its glow"
            );
            t.event(&Event::ModifiersChanged(Default::default()));
            t.event(&Event::ModifiersChanged(modifiers));
            assert!(t.codes().1, "a new aggregate-only observation is ambiguous");
        }
    }
    #[test]
    fn modifier_release_orders_and_unmatched_release_allow_new_aggregate_only_input() {
        for (side, modifiers, pair) in [
            (
                Key::ShiftLeft,
                egui::Modifiers {
                    shift: true,
                    ..Default::default()
                },
                [55, 66],
            ),
            (
                Key::ControlRight,
                egui::Modifiers {
                    ctrl: true,
                    command: true,
                    ..Default::default()
                },
                [67, 74],
            ),
            (
                Key::AltLeft,
                egui::Modifiers {
                    alt: true,
                    ..Default::default()
                },
                [69, 71],
            ),
            (
                Key::SuperRight,
                egui::Modifiers {
                    mac_cmd: true,
                    command: true,
                    ..Default::default()
                },
                [68, 72],
            ),
        ] {
            for aggregate_first in [false, true] {
                let mut t = InputTest::default();
                t.event(&Event::ModifiersChanged(modifiers));
                t.event(&key(side, Some(side), true));
                if aggregate_first {
                    t.event(&Event::ModifiersChanged(Default::default()));
                }
                t.event(&key(side, Some(side), false));
                if !aggregate_first {
                    t.event(&Event::ModifiersChanged(Default::default()));
                }
                assert!(t.codes().0.is_empty());
                t.event(&Event::ModifiersChanged(modifiers));
                assert_eq!(
                    t.codes(),
                    (BTreeSet::from(pair), true),
                    "side={side:?}, aggregate_first={aggregate_first}"
                );
            }
            let mut t = InputTest::default();
            t.event(&key(side, Some(side), false));
            t.event(&Event::ModifiersChanged(modifiers));
            assert_eq!(
                t.codes(),
                (BTreeSet::from(pair), true),
                "unmatched release {side:?}"
            );
        }
    }
}
