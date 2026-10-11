use crate::editor::{Editor, Mode, action_label};
use crate::theme::Palette;
use eframe::egui::{self, RichText};

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
    let palette = Palette::of(ui);
    ui.horizontal_wrapped(|ui| {
        ui.strong("編集中の割り当て").on_hover_text("公式ソフトの配列を基準に、現在の編集内容を表示しています。刻印と異なることがあります。適用するまでデバイスは変わりません。");
        ui.colored_label(palette.from_border, egui::RichText::new("■ From").size(12.));
        if editor.selection.mode == Mode::Swap {
            ui.colored_label(palette.to_border, egui::RichText::new("■ To").size(12.));
        }
        ui.colored_label(palette.dirty, egui::RichText::new("● 未適用").size(12.));
        if editor.selection.mode == Mode::Swap {
            ui.add_enabled_ui(!blocked && !confirmation && !popup, |ui| {
                ui.small("次のクリック:");
                for (target, label) in [(ClickTarget::From, "From"), (ClickTarget::To, "To")] {
                    if crate::theme::choice(ui, state.target == target, label).clicked() {
                        state.set_target(target, blocked, confirmation, popup);
                    }
                }
            });
        } else { ui.small("キーをクリックして選択"); }
    });
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
                    (palette.from_fill, palette.from_border, palette.from_text)
                } else if view.is_to {
                    (palette.to_fill, palette.to_border, palette.to_text)
                } else {
                    (palette.control, palette.control_border, palette.text)
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
                        palette.dirty,
                    );
                }
                if view.is_from && view.is_to {
                    ui.painter().line_segment(
                        [
                            rect.left_bottom() + egui::vec2(4., -3.),
                            rect.right_bottom() + egui::vec2(-4., -3.),
                        ],
                        egui::Stroke::new(2.5, palette.to_border),
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

/// Read-only reference output candidates; no selection or editor history is used.
pub(crate) fn show_input(
    ui: &mut egui::Ui,
    profile: &niz_core::Profile,
    layer: usize,
    pressed: &std::collections::BTreeSet<usize>,
) {
    let palette = Palette::of(ui);
    let mut reference = Editor::new(profile.clone());
    reference.select_layer(layer);
    let width = ui.available_width();
    let (_, area) = ui.allocate_space(egui::vec2(width, KEY_HEIGHT * 5. + GAP * 4.));
    for (row_index, row) in row_geometry(width).iter().enumerate() {
        for key in row {
            let view = key_view(&reference, key.slot);
            let rect = egui::Rect::from_min_size(
                area.min + egui::vec2(key.x, row_index as f32 * (KEY_HEIGHT + GAP)),
                egui::vec2(key.width, KEY_HEIGHT),
            );
            let held = pressed.contains(&key.slot);
            let (fill, border, text) = if held {
                (palette.input_fill, palette.input_border, palette.input_text)
            } else {
                (palette.control, palette.control_border, palette.text)
            };
            ui.painter().rect(
                rect,
                5.,
                fill,
                egui::Stroke::new(if held { 2. } else { 1. }, border),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("{}\n#{:02}", view.label, key.slot + 1),
                egui::FontId::proportional(12.),
                text,
            );
            ui.interact(
                rect,
                ui.id().with(("input-key", key.slot)),
                egui::Sense::hover(),
            )
            .on_hover_text(format!(
                "参照レイヤー: {} · 位置 #{}\n刻印: {}\n読み込んだ出力: {}\n{}",
                ["通常", "右 Fn", "左 Fn"][layer],
                key.slot + 1,
                view.legend,
                action_label(profile, layer * 66 + key.slot),
                if held {
                    "受け取った出力の一致候補"
                } else {
                    "一致候補の入力なし"
                }
            ));
        }
    }
}

pub(crate) fn physical_legend(slot: usize) -> &'static str {
    PHYSICAL_LEGENDS.get(slot).copied().unwrap_or("Unknown")
}
fn compact_legend(slot: usize) -> &'static str {
    match physical_legend(slot) {
        "Backspace" => "BS",
        "Caps Lock" => "Caps",
        "Left Shift" => "L⇧",
        "Right Shift / Up" => "R⇧/↑",
        "Left Control" => "LCtrl",
        "Right Control" => "RCtrl",
        "Left Fn" => "LFn",
        "Right Fn" => "RFn",
        "Left Alt" => "LAlt",
        "Right Alt" => "RAlt",
        "Delete" => "Del",
        "Left" => "←",
        "Down" => "↓",
        "Right" => "→",
        legend => legend,
    }
}
pub(crate) fn show_physical(ui: &mut egui::Ui, selected: usize, blocked: bool) -> Option<usize> {
    let palette = Palette::of(ui);
    let width = ui.available_width();
    let (_, area) = ui.allocate_space(egui::vec2(width, KEY_HEIGHT * 5. + GAP * 4.));
    let mut choice = None;
    ui.add_enabled_ui(!blocked, |ui| {
        ui.spacing_mut().button_padding = egui::vec2(2., 3.);
        for (row_index, row) in row_geometry(width).iter().enumerate() {
            for key in row {
                let rect = egui::Rect::from_min_size(
                    area.min + egui::vec2(key.x, row_index as f32 * (KEY_HEIGHT + GAP)),
                    egui::vec2(key.width, KEY_HEIGHT),
                );
                let (fill, border, text) = if selected == key.slot {
                    (palette.from_fill, palette.from_border, palette.from_text)
                } else {
                    (palette.control, palette.control_border, palette.text)
                };
                let response = ui
                    .put(
                        rect,
                        egui::Button::new(
                            RichText::new(format!(
                                "{}\n#{:02}",
                                compact_legend(key.slot),
                                key.slot + 1
                            ))
                            .size(12.)
                            .color(text),
                        )
                        .fill(fill)
                        .stroke(egui::Stroke::new(
                            if selected == key.slot { 1.8 } else { 1. },
                            border,
                        ))
                        .corner_radius(5.)
                        .min_size(rect.size()),
                    )
                    .on_hover_text(format!(
                        "公式配列の刻印: {} · 位置 #{}\nキーの選択だけでは設定を変更しません。",
                        physical_legend(key.slot),
                        key.slot + 1
                    ));
                if response.clicked() {
                    choice = Some(key.slot);
                }
            }
        }
    });
    choice
}
