use crate::editor::{Editor, Mode, action_label};
use eframe::egui::{self, Color32, RichText};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClickTarget {
    #[default]
    From,
    To,
}
#[derive(Default)]
pub struct KeyboardState {
    pub target: ClickTarget,
}
impl KeyboardState {
    pub fn set_mode(&mut self, editor: &mut Editor, mode: Mode) {
        if editor.selection.mode != mode {
            editor.selection.mode = mode;
            self.target = ClickTarget::From;
        }
    }
    pub fn set_target(
        &mut self,
        target: ClickTarget,
        blocked: bool,
        confirmation: bool,
        popup: bool,
    ) -> bool {
        if blocked || confirmation || popup || self.target == target {
            return false;
        }
        self.target = target;
        true
    }
    pub fn click(
        &mut self,
        editor: &mut Editor,
        slot: usize,
        blocked: bool,
        confirmation: bool,
        popup: bool,
    ) -> bool {
        if blocked || confirmation || popup || slot >= 66 {
            return false;
        }
        match (editor.selection.mode, self.target) {
            (Mode::Reassign, _) => editor.select_from(slot),
            (Mode::Swap, ClickTarget::From) => {
                editor.select_from(slot);
                self.target = ClickTarget::To;
                true
            }
            (Mode::Swap, ClickTarget::To) => {
                if editor.selection.to_position == slot {
                    false
                } else {
                    editor.selection.to_position = slot;
                    true
                }
            }
        }
    }
}

// Official main-view records use immutable IDs 1..66 in row-major order. The
// supplied EXE/DLL preserves these IDs through F2 get/set indexing. Geometry and
// printed legends are separate from the keyboard's mutable current assignments.
const ROW_UNITS: &[&[f32]] = &[
    &[1.; 15],
    &[1.5, 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1.5],
    &[1.75, 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 2.25],
    &[2.25, 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1.75, 1.],
    &[1.5, 1., 1., 1., 4.5, 1., 1., 1., 1., 1., 1.],
];
const GAP: f32 = 4.;
const KEY_HEIGHT: f32 = 46.;
const PHYSICAL_LEGENDS: [&str; 66] = [
    "Esc",
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
    "0",
    "-",
    "=",
    "`",
    "Backspace",
    "Tab",
    "Q",
    "W",
    "E",
    "R",
    "T",
    "Y",
    "U",
    "I",
    "O",
    "P",
    "[",
    "]",
    "\\",
    "Caps Lock",
    "A",
    "S",
    "D",
    "F",
    "G",
    "H",
    "J",
    "K",
    "L",
    ";",
    "'",
    "Enter",
    "Left Shift",
    "Z",
    "X",
    "C",
    "V",
    "B",
    "N",
    "M",
    ",",
    ".",
    "/",
    "Right Shift / Up",
    "Delete",
    "Left Control",
    "Left Fn",
    "Win",
    "Left Alt",
    "Space",
    "Right Fn",
    "Right Alt",
    "Right Control",
    "Left",
    "Down",
    "Right",
];
#[derive(Clone, Copy, Debug)]
pub struct KeyGeometry {
    pub slot: usize,
    pub x: f32,
    pub width: f32,
}
/// Internal spacing is part of each unit, so all five unequal rows end together.
pub fn row_geometry(width: f32) -> Vec<Vec<KeyGeometry>> {
    let unit = (width + GAP) / 15.;
    let mut slot = 0;
    ROW_UNITS
        .iter()
        .map(|row| {
            let mut x = 0.;
            row.iter()
                .map(|units| {
                    let key = KeyGeometry {
                        slot,
                        x,
                        width: units * unit - GAP,
                    };
                    slot += 1;
                    x += units * unit;
                    key
                })
                .collect()
        })
        .collect()
}
pub struct KeyView {
    pub label: String,
    pub legend: &'static str,
    pub tooltip: String,
    pub changed: bool,
    pub is_from: bool,
    pub is_to: bool,
}
pub fn key_view(editor: &Editor, slot: usize) -> KeyView {
    let index = editor.selection.layer * 66 + slot;
    let changed = editor.baseline.records.get(index) != editor.draft.records.get(index);
    let label = match editor.draft.definition(index) {
        Ok(d) if d.kind == 0 && d.keys.is_empty() => "—".into(),
        Ok(d) if d.kind == 0 && d.keys.len() == 1 => match d.keys[0] {
            27 => "BS".into(),
            28 => "Tab".into(),
            42 => "Caps".into(),
            54 => "Enter".into(),
            55 => "L⇧".into(),
            66 => "R⇧".into(),
            67 => "LCtrl".into(),
            68 => "LCmd".into(),
            69 => "LAlt".into(),
            70 => "Space".into(),
            71 => "RAlt".into(),
            72 => "RCmd".into(),
            73 => "Menu".into(),
            74 => "RCtrl".into(),
            84 => "Del".into(),
            87 => "↑".into(),
            88 => "←".into(),
            89 => "↓".into(),
            90 => "→".into(),
            156 => "RFn".into(),
            166 => "LFn".into(),
            code if (1..=74).contains(&code) => niz_core::key_name(code).into(),
            code if (1..=177).contains(&code) || code == 199 => "特殊".into(),
            _ => "不明".into(),
        },
        _ => "詳細".into(),
    };
    let layer = ["通常", "右 Fn", "左 Fn"]
        .get(editor.selection.layer)
        .copied()
        .unwrap_or("追加グループ");
    KeyView {
        label,
        legend: PHYSICAL_LEGENDS.get(slot).copied().unwrap_or("Unknown"),
        tooltip: format!(
            "{layer} · 位置 #{:02}\n刻印（公式配列）: {}\n現在: {}\n読み込み時: {}\n{}",
            slot + 1,
            PHYSICAL_LEGENDS.get(slot).copied().unwrap_or("Unknown"),
            action_label(&editor.draft, index),
            action_label(&editor.baseline, index),
            if changed {
                "変更あり · 生データに差分があります"
            } else {
                "変更なし"
            }
        ),
        changed,
        is_from: editor.selection.from == slot,
        is_to: editor.selection.mode == Mode::Swap && editor.selection.to_position == slot,
    }
}

pub fn show(
    ui: &mut egui::Ui,
    editor: &mut Editor,
    state: &mut KeyboardState,
    blocked: bool,
    confirmation: bool,
    popup: bool,
) {
    ui.add_space(6.);
    ui.strong("キーボード · 現在の割り当て");
    ui.small(
        "公式ソフトの配列を基準に、現在の割り当てを表示しています。刻印と異なることがあります。",
    );
    ui.horizontal_wrapped(|ui| {
        ui.colored_label(Color32::from_rgb(33, 113, 91), "■ From");
        if editor.selection.mode == Mode::Swap {
            ui.colored_label(Color32::from_rgb(154, 105, 23), "■ To");
        }
        ui.colored_label(Color32::from_rgb(176, 83, 22), "● 未適用の変更");
        if editor.selection.mode == Mode::Reassign {
            ui.small("キーをクリックして設定を選択");
        }
    });
    if editor.selection.mode == Mode::Swap {
        ui.add_enabled_ui(!blocked && !confirmation && !popup, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("次のクリックで選ぶ位置:");
                for (target, label) in [(ClickTarget::From, "From"), (ClickTarget::To, "To")] {
                    if ui.selectable_label(state.target == target, label).clicked() {
                        state.set_target(target, blocked, confirmation, popup);
                    }
                }
            });
        });
    }
    let width = ui.available_width();
    let rows = row_geometry(width);
    let (_, area) = ui.allocate_space(egui::vec2(width, KEY_HEIGHT * 5. + GAP * 4.));
    let mut selected = None;
    ui.add_enabled_ui(!blocked && !confirmation && !popup, |ui| {
        ui.spacing_mut().button_padding = egui::vec2(2., 3.);
        for (row_index, row) in rows.iter().enumerate() {
            for key in row {
                let view = key_view(editor, key.slot);
                let rect = egui::Rect::from_min_size(
                    area.min + egui::vec2(key.x, row_index as f32 * (KEY_HEIGHT + GAP)),
                    egui::vec2(key.width, KEY_HEIGHT),
                );
                let (fill, stroke, text) = if view.is_from {
                    (
                        Color32::from_rgb(223, 243, 235),
                        Color32::from_rgb(33, 113, 91),
                        Color32::from_rgb(24, 84, 68),
                    )
                } else if view.is_to {
                    (
                        Color32::from_rgb(255, 244, 215),
                        Color32::from_rgb(174, 118, 22),
                        Color32::from_rgb(115, 77, 12),
                    )
                } else {
                    (
                        Color32::from_rgb(247, 249, 250),
                        Color32::from_rgb(189, 199, 203),
                        Color32::from_rgb(48, 62, 67),
                    )
                };
                let title = format!("{}\n#{:02}", view.label, key.slot + 1);
                let button = egui::Button::new(RichText::new(title).size(12.).color(text))
                    .fill(fill)
                    .stroke(egui::Stroke::new(
                        if view.is_from || view.is_to { 1.8 } else { 1. },
                        stroke,
                    ))
                    .corner_radius(5.)
                    .min_size(rect.size());
                let response = ui.put(rect, button).on_hover_text(&view.tooltip);
                if view.changed {
                    ui.painter().circle_filled(
                        rect.right_top() + egui::vec2(-5., 5.),
                        2.5,
                        Color32::from_rgb(176, 83, 22),
                    );
                }
                if view.is_from && view.is_to {
                    ui.painter().line_segment(
                        [
                            rect.left_bottom() + egui::vec2(4., -3.),
                            rect.right_bottom() + egui::vec2(-4., -3.),
                        ],
                        egui::Stroke::new(2.5, Color32::from_rgb(174, 118, 22)),
                    );
                }
                if response.clicked() {
                    selected = Some(key.slot);
                }
            }
        }
    });
    if let Some(slot) = selected {
        state.click(editor, slot, blocked, confirmation, popup);
    }
}
