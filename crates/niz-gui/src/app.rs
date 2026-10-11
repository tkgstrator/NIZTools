use crate::{
    controller::{Activation, Controller, ReadChoice, TrialApproval},
    editor::{Mode, action_label, exact_trial},
    input_test::InputTest,
    keyboard::{self, KeyboardState},
    protocol::{RawThresholdSource, Target},
    theme::{self, Palette},
    worker::{ChannelPort, spawn},
};
use eframe::egui::{self, RichText};
use niz_core::{Profile, key_name};
use niz_device::{CalibrationPhase, UnlockOutcome};
use std::path::PathBuf;
#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Assignment,
    Calibration,
    InputTest,
}
enum Confirm {
    Read(String),
    Load(Profile),
    Apply(Box<TrialApproval>),
}
pub struct App {
    controller: Controller<ChannelPort>,
    demo: bool,
    view: View,
    backup: PathBuf,
    confirm: Option<Confirm>,
    calibration_position: usize,
    sensitivity_coefficient: u8,
    keyboard: KeyboardState,
    appearance: egui::ThemePreference,
    input_test: InputTest,
}
impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, demo: bool) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "NotoJP".into(),
            egui::FontData::from_static(include_bytes!("../../../assets/fonts/NotoSansJP.ttf"))
                .into(),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push("NotoJP".into());
        cc.egui_ctx.set_fonts(fonts);
        theme::install(&cc.egui_ctx);
        let mut controller = Controller::new(spawn(demo, cc.egui_ctx.clone()));
        let _ = controller.list();
        let backup = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Library/Application Support/NIZTools/Backups");
        Self {
            controller,
            demo,
            view: View::Assignment,
            backup,
            confirm: None,
            calibration_position: 30,
            sensitivity_coefficient: 24,
            keyboard: KeyboardState::default(),
            appearance: egui::ThemePreference::System,
            input_test: InputTest::default(),
        }
    }
    fn result(&mut self, result: Result<(), String>) {
        if let Err(error) = result {
            self.controller.message = error;
        }
    }
    fn read_selected(&mut self) {
        let Some(path) = self.controller.selected_path.clone() else {
            return;
        };
        if self.controller.editor.as_ref().is_some_and(|e| e.dirty()) {
            self.confirm = Some(Confirm::Read(path));
        } else {
            let result = self.controller.read(path, ReadChoice::Discard);
            self.result(result);
        }
    }
    fn files(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("ファイルを開く…").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("NIZ profile", &["json", "pro"])
                    .pick_file()
            {
                match niz_device::load_profile(&path) {
                    Ok(profile) => {
                        if self.controller.editor.as_ref().is_some_and(|e| e.dirty()) {
                            self.confirm = Some(Confirm::Load(profile));
                        } else {
                            let result = self.controller.load(profile, true);
                            self.result(result);
                        }
                    }
                    Err(error) => self.controller.message = error.to_string(),
                }
            }
            let has = self.controller.editor.is_some();
            if ui
                .add_enabled(has, egui::Button::new("編集内容を保存…"))
                .clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .set_file_name("niz-draft.json")
                    .save_file()
            {
                let result = self
                    .controller
                    .editor
                    .as_ref()
                    .unwrap()
                    .draft
                    .to_json()
                    .map_err(|e| e.to_string())
                    .and_then(|text| std::fs::write(&path, text).map_err(|e| e.to_string()));
                self.controller.message = match result {
                    Ok(()) => format!(
                        "編集内容を保存しました: {}。デバイスの設定は変更していません。",
                        path.display()
                    ),
                    Err(error) => error,
                };
            }
            egui::CollapsingHeader::new("バックアップ").show(ui, |ui| {
                if ui
                    .add_enabled(has, egui::Button::new("読み込み時の設定を保存"))
                    .clicked()
                {
                    self.controller.message = match niz_device::save_backup(
                        &self.controller.editor.as_ref().unwrap().baseline,
                        &self.backup,
                    ) {
                        Ok(path) => format!(
                            "設定のバックアップ: {}。アナログ調整は復元できません。",
                            path.display()
                        ),
                        Err(error) => error.to_string(),
                    };
                }
                ui.small(self.backup.display().to_string());
                if ui.button("保存先を変更…").clicked()
                    && let Some(directory) = rfd::FileDialog::new().pick_folder()
                {
                    self.backup = directory;
                }
            });
        });
    }
    fn assignment(&mut self, ui: &mut egui::Ui, popup_at_frame_start: bool) {
        let operation_blocked = self.controller.blocked();
        let blocked = operation_blocked || self.confirm.is_some();
        // Capture popup ownership before a selector closes in this frame.
        let popup_was_open = popup_at_frame_start || egui::Popup::is_any_open(ui.ctx());
        let Some(e) = self.controller.editor.as_mut() else {
            Palette::of(ui).card().show(ui, |ui| {
                ui.strong("キーボードの設定を読み込む");
                ui.label("上でデバイスを選び、読み込んでください。");
                ui.small("保存したファイルを開いて、オフラインで編集することもできます。");
            });
            return;
        };
        let mut edit = false;
        let mut trial = false;
        ui.add_enabled_ui(!blocked, |ui| {
            ui.horizontal_wrapped(|ui| {
                let mut mode = e.selection.mode;
                crate::theme::choice_value(ui, &mut mode, Mode::Reassign, "出力を変更");
                crate::theme::choice_value(ui, &mut mode, Mode::Swap, "2つの位置を交換");
                self.keyboard.set_mode(e, mode);
                ui.separator();
                let mut layer = e.selection.layer;
                for (i, label) in ["通常", "右 Fn", "左 Fn"].iter().enumerate() {
                    crate::theme::choice_value(ui, &mut layer, i, *label);
                }
                e.select_layer(layer);
            });
            Palette::of(ui).card().show(ui, |ui| {
                ui.columns(2, |columns| {
                    columns[0].strong("From · 変更する位置");
                    if let Some(slot) = slot_selector(
                        &mut columns[0],
                        "from",
                        e.selection.from,
                        &e.draft,
                        e.selection.layer,
                    ) {
                        e.select_from(slot);
                    }
                    columns[0].small(format!(
                        "刻印: {}",
                        keyboard::key_view(e, e.selection.from).legend
                    ));
                    match e.selection.mode {
                        Mode::Reassign => {
                            columns[1].strong("To · 新しい出力");
                            let width = columns[1].available_width();
                            egui::ComboBox::from_id_salt("output")
                                .width(width)
                                .selected_text(if e.output_ready() {
                                    key_name(e.selection.to_code)
                                } else {
                                    "現在の設定を保持"
                                })
                                .show_ui(&mut columns[1], |ui| {
                                    for code in (1..=74).chain([156, 166]) {
                                        if crate::theme::choice(
                                            ui,
                                            e.output_ready() && e.selection.to_code == code,
                                            key_name(code),
                                        )
                                        .clicked()
                                        {
                                            e.choose_output(code);
                                            ui.close();
                                        }
                                    }
                                });
                            columns[1].small("選んだ出力は「変更を追加」で編集に反映");
                        }
                        Mode::Swap => {
                            columns[1].strong("To · 交換先の位置");
                            if let Some(slot) = slot_selector(
                                &mut columns[1],
                                "to",
                                e.selection.to_position,
                                &e.draft,
                                e.selection.layer,
                            ) {
                                e.selection.to_position = slot;
                            }
                            columns[1].small(format!(
                                "刻印: {}",
                                keyboard::key_view(e, e.selection.to_position).legend
                            ));
                        }
                    }
                });
                let next = e.proposed();
                ui.horizontal_wrapped(|ui| {
                    match &next {
                        Ok(next) => {
                            let changes = e.draft.differences(next).unwrap_or_default();
                            if changes.is_empty() {
                                ui.small("現在と同じ割り当てです");
                            } else {
                                let from = e.selection.layer * 66 + e.selection.from;
                                ui.label(format!(
                                    "#{}: {} → {}",
                                    e.selection.from + 1,
                                    action_label(&e.draft, from),
                                    action_label(next, from)
                                ));
                                if e.selection.mode == Mode::Swap {
                                    let to = e.selection.layer * 66 + e.selection.to_position;
                                    ui.label(format!(
                                        "/ #{}: {} → {}",
                                        e.selection.to_position + 1,
                                        action_label(&e.draft, to),
                                        action_label(next, to)
                                    ));
                                }
                            }
                        }
                        Err(_) if !e.output_ready() && e.selection.mode == Mode::Reassign => {
                            ui.small("新しい出力を選ぶまで、現在のデータを保持します");
                        }
                        Err(_) => {
                            ui.colored_label(
                                Palette::of(ui).warning,
                                "この割り当ては変更できません。現在のデータを保持します",
                            );
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_enabled(
                                next.as_ref().is_ok_and(|p| p.records != e.draft.records),
                                Palette::of(ui).primary_button("変更を追加"),
                            )
                            .clicked()
                        {
                            edit = true;
                        }
                    });
                });
                if next.as_ref().is_ok_and(|p| {
                    e.draft
                        .differences(p)
                        .unwrap_or_default()
                        .iter()
                        .any(|i| i / 66 != e.selection.layer)
                }) {
                    ui.colored_label(
                        Palette::of(ui).warning,
                        "Fn の移動は3レイヤーに反映されます。実機での適用は未対応です。",
                    );
                }
            });
            Palette::of(ui).card().show(ui, |ui| {
                keyboard::show(
                    ui,
                    e,
                    &mut self.keyboard,
                    operation_blocked,
                    self.confirm.is_some(),
                    popup_was_open || egui::Popup::is_any_open(ui.ctx()),
                );
            });
        });
        let changes = e.preview().unwrap_or_default();
        let gate = exact_trial(&e.baseline, &e.draft, e.selection);
        Palette::of(ui).card().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong(if e.dirty() { format!("未適用の変更 · {}件", changes.len()) } else { "未適用の変更なし".into() });
                if ui.add_enabled(!blocked && e.can_undo(), egui::Button::new("元に戻す")).clicked() { e.undo(); }
                if ui.add_enabled(!blocked && e.can_redo(), egui::Button::new("やり直す")).clicked() { e.redo(); }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add_enabled(!blocked && self.controller.live && e.dirty() && gate.is_ok(), Palette::of(ui).primary_button(if self.demo { "デモで適用を確認…" } else { "変更を確認して適用…" })).clicked() {
                        trial = true;
                    }
                });
            });
            ui.small(if !self.controller.live {
                "適用するには、選択したデバイスを読み直してください。"
            } else if gate.is_err() {
                "実機への試験適用は基本キー1つ、または2つの交換に限ります。この編集はファイルに保存できます。"
            } else {
                "デバイスへの適用は試験機能です。次の画面で対象と変更内容を確認します。"
            });
            if !changes.is_empty() {
                egui::CollapsingHeader::new("変更の一覧").show(ui, |ui| {
                    for i in &changes { ui.label(change_label(&e.baseline, &e.draft, *i)); }
                });
            }
        });
        if edit {
            let result = self.controller.edit();
            self.result(result);
        }
        if trial {
            match self.controller.prepare_trial(self.backup.clone()) {
                Ok(approval) => self.confirm = Some(Confirm::Apply(Box::new(approval))),
                Err(error) => self.controller.message = error,
            }
        }
    }
    fn calibration(&mut self, ui: &mut egui::Ui, popup_at_frame_start: bool) {
        let pending = self.controller.pending.is_some();
        let blocked = self.controller.blocked() || self.confirm.is_some();
        let current_source = self
            .controller
            .attached_target
            .as_ref()
            .is_some_and(|target| {
                self.controller.selected_path.as_ref() == Some(&target.path)
                    && self
                        .controller
                        .selected_device()
                        .is_some_and(|device| Target::from(device) == *target)
            });
        let held = self
            .controller
            .calibration
            .as_ref()
            .filter(|_| current_source)
            .and_then(|status| status.held_key.as_ref())
            .cloned();
        let sensitivity = self
            .controller
            .sensitivity
            .as_ref()
            .filter(|_| current_source)
            .cloned();
        let before = held
            .as_ref()
            .and_then(|h| h.before.as_ref())
            .or_else(|| sensitivity.as_ref().and_then(|s| s.before.as_ref()))
            .filter(|v| v.len() == 66)
            .and_then(|v| v.get(self.calibration_position))
            .copied()
            .or_else(|| {
                self.controller
                    .raw_thresholds()
                    .and_then(|r| r.snapshot.value_for_position(self.calibration_position))
            });
        let after = held
            .as_ref()
            .and_then(|h| h.after.as_ref())
            .or_else(|| sensitivity.as_ref().and_then(|s| s.after.as_ref()))
            .filter(|v| v.len() == 66)
            .and_then(|v| v.get(self.calibration_position))
            .copied();
        Palette::of(ui).card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.strong(format!(
                "選択中のキー: #{:02} · {}",
                self.calibration_position + 1,
                keyboard::physical_legend(self.calibration_position)
            ));
            ui.horizontal(|ui| {
                ui.label(format!(
                    "FROM: {}",
                    before.map(|v| v.to_string()).unwrap_or_else(|| {
                        if self.controller.raw_thresholds_pending() {
                            "…".into()
                        } else {
                            "—".into()
                        }
                    })
                ));
                ui.label("→");
                ui.label(format!(
                    "TO: {}",
                    after.map(|v| v.to_string()).unwrap_or_else(|| "—".into())
                ));
                if self.demo {
                    ui.small("DEMO値");
                }
            });
            let actions_enabled = !popup_at_frame_start
                && self.confirm.is_none()
                && !egui::Popup::is_any_open(ui.ctx())
                && !self.controller.close_requested;
            if let Some(status) = &self.controller.calibration
                && let Some(held) = &status.held_key
                && self.controller.calibration_active()
            {
                ui.label(format!(
                    "{}を押したまま実行",
                    keyboard::physical_legend(held.position)
                ));
                let ready = status.phase == CalibrationPhase::AwaitingHeldKeys
                    && !pending
                    && !self.controller.cancel_requested;
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            actions_enabled && ready,
                            egui::Button::new(
                                egui::RichText::new("実行").color(egui::Color32::WHITE),
                            )
                            .fill(egui::Color32::from_rgb(180, 40, 40)),
                        )
                        .clicked_by(egui::PointerButton::Primary)
                    {
                        let result = self.controller.held(Activation::Pointer);
                        self.result(result);
                    }
                    if ui
                        .add_enabled(
                            actions_enabled && !self.controller.cancel_requested,
                            egui::Button::new("キャンセル"),
                        )
                        .clicked()
                    {
                        self.controller.cancel_calibration();
                    }
                    if pending {
                        ui.spinner();
                    }
                });
            } else if self.controller.calibration_active() && self.controller.calibration.is_none()
            {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("準備中…");
                    if ui
                        .add_enabled(
                            actions_enabled && !self.controller.cancel_requested,
                            egui::Button::new("キャンセル"),
                        )
                        .clicked()
                    {
                        self.controller.cancel_calibration();
                    }
                });
            } else if !self.controller.calibration_active() {
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            actions_enabled && self.controller.can_read_raw_thresholds(),
                            egui::Button::new("しきい値を読み込む"),
                        )
                        .on_hover_text("キーを離して読み込む")
                        .clicked()
                    {
                        let result = self.controller.read_raw_thresholds();
                        self.result(result);
                    }
                    if ui
                        .add_enabled(
                            actions_enabled && self.controller.can_read_raw_thresholds(),
                            egui::Button::new("すべてのキーを離して準備"),
                        )
                        .clicked_by(egui::PointerButton::Primary)
                    {
                        let result = self.controller.begin_held(
                            self.backup.clone(),
                            self.calibration_position,
                            Activation::Pointer,
                        );
                        self.result(result);
                    }
                });
                if let Some(held) = held.as_ref()
                    && let (Some(before), Some(after)) = (&held.before, &held.after)
                    && before.len() == 66
                    && after.len() == 66
                {
                    let other = (0..66)
                        .filter(|&i| i != held.position && before[i] != after[i])
                        .count();
                    if other > 0 {
                        ui.colored_label(
                            Palette::of(ui).warning,
                            format!("他のキーも変化: {other}個"),
                        );
                    }
                }
            }
        });
        ui.add_enabled_ui(self.confirm.is_none(), |ui| {
            crate::sensitivity_ui::show(
                ui,
                &mut self.controller,
                &mut self.sensitivity_coefficient,
                &self.backup,
                self.demo,
                popup_at_frame_start,
            );
        });
        if self.controller.calibration_active()
            && self
                .controller
                .calibration
                .as_ref()
                .is_some_and(|s| s.held_key.is_none())
        {
            Palette::of(ui).card().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.strong("進行中の校正 · 全キーに作用");
                ui.label("選択したキーの個別調整ではありません。アナログの校正はバックアップで元に戻せません。");
                ui.horizontal_wrapped(|ui| {
                    ui.spinner();
                    ui.label(if pending {
                        "処理と安全な終了を待っています。"
                    } else {
                        "中止を要求すると安全な終了処理を行います。"
                    });
                    if ui.button("中止を要求").clicked() {
                        self.controller.cancel_calibration();
                    }
                });
            });
        }
        Palette::of(ui).card().show(ui, |ui| {
            if let Some(slot) = keyboard::show_physical(
                ui,
                self.calibration_position,
                blocked || popup_at_frame_start || egui::Popup::is_any_open(ui.ctx()),
            ) {
                self.calibration_position = slot;
            }
        });
        if let Some(read) = self.controller.raw_thresholds() {
            egui::CollapsingHeader::new("生しきい値の読み取り記録").show(ui, |ui| {
                target_details(ui, &read.target);
                ui.label(&read.snapshot.firmware);
                let time = read.read_at.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                ui.label(format!("読み取り時刻: Unix {}.{:03}秒 · 接続世代 {}", time.as_secs(), time.subsec_millis(), read.generation));
                ui.label(match read.source { RawThresholdSource::Device => "読み取り元: 接続中の実機", RawThresholdSource::Simulation => "読み取り元: DEMOの生成値" });
                ui.small("読み取り時点の通常スキャン用しきい値です。継続的なセンサー測定や、他の動作モードの確認は行いません。");
                ui.small(format!("受信記録: {}パケット", read.snapshot.raw_reports().len()));
            });
        }
        if let Some(status) = &self.controller.calibration {
            if matches!(
                status.phase,
                CalibrationPhase::Aborted | CalibrationPhase::Failed
            ) {
                ui.colored_label(
                    Palette::of(ui).warning,
                    if status.phase == CalibrationPhase::Aborted {
                        "中止しました"
                    } else {
                        "校正に失敗しました。読み込み直してください。"
                    },
                );
            }
            egui::CollapsingHeader::new("校正の結果・対象・バックアップ").show(ui, |ui| {
                if let Some(target) = &self.controller.attached_target {
                    target_details(ui, target);
                }
                if let Some(held) = &status.held_key {
                    if let (Some(before), Some(after)) = (&held.before, &held.after)
                        && before.len() == 66
                        && after.len() == 66
                    {
                        let changed = (0..66)
                            .filter(|&p| p != held.position && before[p] != after[p])
                            .map(|p| {
                                format!(
                                    "#{:02} · {}: {} → {}",
                                    p + 1,
                                    keyboard::physical_legend(p),
                                    before[p],
                                    after[p]
                                )
                            })
                            .collect::<Vec<_>>();
                        if !changed.is_empty() {
                            ui.colored_label(
                                Palette::of(ui).warning,
                                format!("変化した他のキー: {}", changed.join(" / ")),
                            );
                        }
                    }
                    if self.demo {
                        ui.label(
                            "DEMO: 以下は生成したシミュレーション値です。実機測定ではありません。",
                        );
                    }
                    ui.label("全66キーの比較 · 公式配列の刻印");
                    egui::Grid::new("held-key-comparison")
                        .striped(true)
                        .show(ui, |ui| {
                            ui.strong("物理位置・刻印");
                            ui.strong("前 → 後");
                            ui.strong("変化");
                            ui.end_row();
                            for position in 0..66 {
                                let before = held
                                    .before
                                    .as_ref()
                                    .filter(|v| v.len() == 66)
                                    .map(|v| v[position]);
                                let after = held
                                    .after
                                    .as_ref()
                                    .filter(|v| v.len() == 66)
                                    .map(|v| v[position]);
                                ui.label(format!(
                                    "#{:02} · {}",
                                    position + 1,
                                    keyboard::physical_legend(position)
                                ));
                                ui.label(format!(
                                    "{} → {}",
                                    before
                                        .map(|v| v.to_string())
                                        .unwrap_or_else(|| "未取得".into()),
                                    after
                                        .map(|v| v.to_string())
                                        .unwrap_or_else(|| "未取得".into())
                                ));
                                ui.label(match (before, after) {
                                    (Some(a), Some(b)) if a != b => "変化あり",
                                    (Some(_), Some(_)) => "変化なし",
                                    _ => "比較不可",
                                });
                                ui.end_row();
                            }
                        });
                }
                ui.label(format!(
                    "段階: {:?} / 失敗時の段階: {:?}",
                    status.phase, status.failure_phase
                ));
                ui.label(format!(
                    "離した状態の処理完了: {} / 押した状態の処理完了: {}",
                    status.release_completed, status.held_completed
                ));
                ui.label(match &status.unlock {
                    UnlockOutcome::NotAttempted => "ロック解除: 未送信".into(),
                    UnlockOutcome::Sent => {
                        "ロック解除: 送信済み。応答・タイピング回復は未確認です。".into()
                    }
                    UnlockOutcome::Uncertain(error) => format!(
                        "ロック解除: 不確実 ({error})。入力がロックされたままの可能性があります。"
                    ),
                });
                ui.label(format!(
                    "設定のバイト一致: {} / 照明のバイト一致: {}",
                    status.settings_verified, status.lighting_verified
                ));
                if let Some(error) = &status.original_error {
                    ui.label(format!("元のエラー: {error}"));
                }
                if let Some(backup) = &status.backup {
                    ui.small(format!(
                        "設定: {}\n照明: {}\nManifest: {}",
                        backup.config_path.display(),
                        backup.lighting_path.display(),
                        backup.manifest_path.display()
                    ));
                }
            });
        } else {
            egui::CollapsingHeader::new("対象デバイス").show(ui, |ui| {
                if let Some(target) = &self.controller.attached_target {
                    target_details(ui, target);
                }
            });
        }
    }
    fn input_test(&mut self, ui: &mut egui::Ui) {
        let enabled = !self.controller.blocked() && self.confirm.is_none();
        if !enabled {
            ui.small("入力テストを停止中");
        } else if !self.controller.live {
            ui.small("上の「読み込む」を押してください");
        }
        if let Some(editor) = &self.controller.editor {
            let matches = if self.controller.live && enabled {
                self.input_test.matches(&editor.baseline)
            } else {
                Default::default()
            };
            Palette::of(ui).card().show(ui, |ui| {
                ui.set_width(ui.available_width());
                keyboard::show_input(ui, &editor.baseline, 0, &matches);
            });
        }
    }
    fn confirmation(&mut self, ctx: &egui::Context) {
        let Some(confirm) = self.confirm.take() else {
            return;
        };
        let mut keep = true;
        let mut action = None;
        egui::Modal::new(egui::Id::new("confirmation")).show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 48.).min(520.));
            ui.heading("内容を確認");
            let mut execute_label = None;
            match &confirm {
                Confirm::Read(path) => {
                    ui.label("編集中の内容をどうしますか？");ui.small(format!("読み込み先: {path}"));
                    if ui.button("編集内容を保持して読み込む").clicked() {action=Some(0);keep=false;}
                    if ui.button("編集内容を破棄して読み込む").clicked() {action=Some(1);keep=false;}
                    ui.small("保持した編集は、同じデバイスの設定が一致した場合にだけ再び適用できます。");
                }
                Confirm::Load(_) => {
                    ui.label("ファイルの設定で編集中の内容を置き換えます。元に戻す操作で復元できます。");
                    if ui.button("ファイルを読み込む").clicked() {action=Some(1);keep=false;}
                }
                Confirm::Apply(approval) => {
                    ui.strong(if self.demo {"デモでの適用"} else {"試験機能として適用します"});
                    target_details(ui,&approval.target);ui.label(&approval.baseline.firmware);
                    for i in approval.baseline.differences(&approval.draft).unwrap_or_default() {ui.label(change_label(&approval.baseline,&approval.draft,i));}
                    ui.small(format!("バックアップ先: {}",approval.backup.display()));
                    ui.label(if self.demo {"デバイスには書き込まないシミュレーションです。"} else {"変更前に設定と照明を保存します。実際の出力と電源を切った後の保存は未検証です。"});
                    execute_label = Some(if self.demo {"デモで実行"} else {"確認した対象に適用"});
                }
            }
            if let Some(label) = execute_label {
                ui.horizontal(|ui| {
                    if ui.add(Palette::of(ui).danger_button(label)).clicked() { action=Some(1);keep=false; }
                    if ui.button("キャンセル").clicked() { keep=false; }
                });
            } else if ui.button("キャンセル").clicked() {keep=false;}
        });
        if let Some(action) = action {
            let result = match confirm {
                Confirm::Read(path) => self.controller.read(
                    path,
                    if action == 0 {
                        ReadChoice::Retain
                    } else {
                        ReadChoice::Discard
                    },
                ),
                Confirm::Load(profile) => self.controller.load(profile, true),
                Confirm::Apply(approval) => self.controller.approve_trial(*approval),
            };
            self.result(result);
        } else if keep {
            self.confirm = Some(confirm);
        }
    }
    fn device_controls(&mut self, ui: &mut egui::Ui) {
        Palette::of(ui).card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add_enabled_ui(!self.controller.blocked() && self.confirm.is_none(), |ui| {
                ui.horizontal_wrapped(|ui| {
                    let mut selected = self.controller.selected_path.clone();
                    let selected_label = self
                        .controller
                        .selected_device()
                        .map(|d| compact_product(&d.product))
                        .unwrap_or_else(|| "デバイスを選択".into());
                    let response = egui::ComboBox::from_id_salt("devices")
                        .width((ui.available_width() - 205.).clamp(160., 340.))
                        .selected_text(selected_label)
                        .show_ui(ui, |ui| {
                            for device in &self.controller.devices {
                                let label = format!(
                                    "{} · {}",
                                    compact_product(&device.product),
                                    short_path(&device.path)
                                );
                                if crate::theme::choice_value(
                                    ui,
                                    &mut selected,
                                    Some(device.path.clone()),
                                    label,
                                )
                                .on_hover_text(format!("{}\n{}", device.product, device.path))
                                .clicked()
                                {
                                    ui.close();
                                }
                            }
                        });
                    if let Some(device) = self.controller.selected_device() {
                        response
                            .response
                            .on_hover_text(format!("{}\n{}", device.product, device.path));
                    }
                    if selected != self.controller.selected_path
                        && let Some(path) = selected
                    {
                        let result = self.controller.select_device(path);
                        self.result(result);
                    }
                    if ui.button("再検索").clicked() {
                        let result = self.controller.list();
                        self.result(result);
                    }
                    if ui
                        .add_enabled(
                            self.controller.selected_device().is_some(),
                            Palette::of(ui).primary_button("読み込む"),
                        )
                        .clicked()
                    {
                        self.read_selected();
                    }
                });
            });
            ui.horizontal_wrapped(|ui| {
                let (label, color) = if self.controller.pending.is_some() {
                    ("処理中", Palette::of(ui).muted)
                } else if self.controller.failure.is_some() {
                    ("処理に失敗 · 詳細を確認", Palette::of(ui).warning)
                } else if self.controller.calibration_active() {
                    ("キャリブレーション中", Palette::of(ui).warning)
                } else if self.controller.live && self.controller.attached_target.is_some() {
                    (
                        if self.demo {
                            "DEMO · 読み込み済み"
                        } else {
                            "読み込み済み"
                        },
                        Palette::of(ui).accent_text,
                    )
                } else if self.controller.editor.is_some() {
                    (
                        "オフライン · 適用には読み直しが必要",
                        Palette::of(ui).warning,
                    )
                } else if self.controller.devices.is_empty() {
                    ("デバイス未検出 · USB接続を確認", Palette::of(ui).muted)
                } else {
                    ("未読み込み", Palette::of(ui).muted)
                };
                if self.controller.pending.is_some() {
                    ui.spinner();
                }
                ui.colored_label(color, RichText::new(label).size(12.));
                if self.demo {
                    ui.small("実際のキーボードには接続しません");
                }
                egui::CollapsingHeader::new("デバイス情報").show(ui, |ui| {
                    if let Some(device) = self.controller.selected_device() {
                        target_details(ui, &Target::from(device));
                    }
                    if let Some(target) = &self.controller.attached_target {
                        ui.label(if self.controller.live {
                            "表示中の設定の読み込み元"
                        } else {
                            "最後に読み込んだ設定の元（現在は未接続扱い）"
                        });
                        target_details(ui, target);
                        if let Some(editor) = &self.controller.editor {
                            ui.label(&editor.baseline.firmware);
                        }
                    }
                });
            });
        });
    }
}
impl eframe::App for App {
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        self.input_test
            .reference(self.controller.editor.as_ref().map(|e| &e.baseline));
        let test_view = self.view == View::InputTest;
        let focused = input.focused
            && !input
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::WindowFocused(false)));
        let capture = test_view && focused && !self.controller.blocked() && self.confirm.is_none();
        if !capture {
            self.input_test.clear_held();
        }
        if !test_view {
            return;
        }
        if capture {
            for event in &input.events {
                self.input_test.event(event);
            }
        }
        // Consume before egui's focus/shortcut pass. This is app-local only.
        input.events.retain(|event| {
            !matches!(
                event,
                egui::Event::Key { .. }
                    | egui::Event::Text(_)
                    | egui::Event::Copy
                    | egui::Event::Cut
                    | egui::Event::Paste(_)
                    | egui::Event::Ime(_)
                    | egui::Event::ModifiersChanged(_)
            )
        });
        ctx.input_mut(|state| {
            state.keys_down.clear();
            state.modifiers = Default::default();
        });
    }
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        while let Ok(event) = self.controller.port.receiver.try_recv() {
            self.controller.process(event);
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            self.controller.close();
            if !self.controller.ready_close {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }
        if self.controller.ready_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.controller.pending.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
    fn ui(&mut self, root: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        // Header popups may consume their closing event before the board is drawn.
        let popup_at_frame_start = egui::Popup::is_any_open(&ctx);
        if self.controller.calibration_active() {
            self.view = View::Calibration;
        }
        let palette = Palette::of(root);
        egui::Frame::central_panel(root.style()).fill(palette.root)
            .inner_margin(16.).show(root, |ui| {
            egui::ScrollArea::vertical().id_salt("main").auto_shrink([false, false]).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("NIZ Utility");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_enabled_ui(self.confirm.is_none(), |ui| {
                            let before = self.appearance;
                            egui::ComboBox::from_id_salt("appearance").width(100.)
                                .selected_text(appearance_name(self.appearance))
                                .show_ui(ui, |ui| {
                                    for preference in [egui::ThemePreference::System, egui::ThemePreference::Light, egui::ThemePreference::Dark] {
                                        if crate::theme::choice_value(ui, &mut self.appearance, preference, appearance_name(preference)).clicked() { ui.close(); }
                                    }
                                }).response.on_hover_text("このアプリの外観。システムを選ぶとOSの外観に追従します。");
                            ui.small("外観");
                            if before != self.appearance {
                                ctx.set_theme(self.appearance);
                                // Existing UIs retain their style until the next frame.
                                ctx.request_repaint();
                            }
                        });
                    });
                });
                self.device_controls(ui);
                ui.add_enabled_ui(self.confirm.is_none() && !self.controller.calibration_active(), |ui| {
                    ui.horizontal_wrapped(|ui| {
                        crate::theme::choice_value(ui, &mut self.view, View::Assignment, "キーの割り当て");
                        crate::theme::choice_value(ui, &mut self.view, View::Calibration, "入力しきい値");
                        crate::theme::choice_value(ui, &mut self.view, View::InputTest, "入力テスト");
                    });
                });
                if self.view != View::InputTest { self.input_test.clear_held(); }
                match self.view { View::Assignment => self.assignment(ui, popup_at_frame_start), View::Calibration => self.calibration(ui, popup_at_frame_start), View::InputTest => self.input_test(ui) }
                ui.add_enabled_ui(!self.controller.blocked() && self.confirm.is_none(), |ui| {
                    egui::CollapsingHeader::new("ファイル・バックアップ").show(ui, |ui| self.files(ui));
                });
                if self.controller.failure.is_some() {
                    ui.colored_label(palette.warning, "処理を完了できませんでした。編集は保持しています。詳細を確認してください。");
                }
                if !self.controller.message.is_empty() {
                    ui.add(egui::Label::new(RichText::new(format!("操作結果: {}", short_message(&self.controller.message))).small().color(palette.muted)).wrap());
                    egui::CollapsingHeader::new("操作の記録・詳細").show(ui, |ui| {
                        if self.demo { ui.small("シミュレーションの結果です。実機での動作確認ではありません。"); }
                        ui.label(&self.controller.message);
                        if let Some(failure) = &self.controller.failure {
                            ui.label(format!("変更する出力の試行: {} / 保持した受信パケット: {}", failure.began_write, failure.raw_reports.len()));
                            if let Some(path) = &failure.backup { ui.small(format!("バックアップ: {}", path.display())); }
                            if let Some(status) = &failure.calibration { ui.label(format!("元の段階: {:?} / ロック解除: {:?} / トレース: {}パケット", status.failure_phase, status.unlock, status.trace.len())); }
                        }
                    });
                }
            });
        });
        self.confirmation(&ctx);
    }
}
fn appearance_name(preference: egui::ThemePreference) -> &'static str {
    match preference {
        egui::ThemePreference::System => "システム",
        egui::ThemePreference::Light => "ライト",
        egui::ThemePreference::Dark => "ダーク",
    }
}
fn compact_product(product: &str) -> String {
    if product.is_empty() {
        "NIZ keyboard".into()
    } else {
        truncate(product, 32)
    }
}
fn compact_action(action: &str) -> String {
    truncate(action, 24)
}
fn short_message(message: &str) -> String {
    if message == "Fresh device baseline read. No settings were written." {
        return "設定を読み込みました".into();
    }
    truncate(message.lines().next().unwrap_or(message), 96)
}
fn truncate(value: &str, max: usize) -> String {
    let mut chars = value.chars();
    let mut text: String = chars.by_ref().take(max).collect();
    if chars.next().is_some() {
        text.push('…');
    }
    text
}
fn layer_name(layer: usize) -> &'static str {
    ["通常", "右 Fn", "左 Fn"]
        .get(layer)
        .copied()
        .unwrap_or("追加グループ")
}
fn change_label(baseline: &Profile, draft: &Profile, index: usize) -> String {
    format!(
        "{} · 位置 #{}: {} → {}",
        layer_name(index / 66),
        index % 66 + 1,
        action_label(baseline, index),
        action_label(draft, index)
    )
}
fn slot_selector(
    ui: &mut egui::Ui,
    id: &str,
    position: usize,
    profile: &Profile,
    layer: usize,
) -> Option<usize> {
    let mut selected = None;
    egui::ComboBox::from_id_salt(id)
        .width(ui.available_width())
        .selected_text(format!(
            "#{:02} · {}",
            position + 1,
            compact_action(&action_label(profile, layer * 66 + position))
        ))
        .show_ui(ui, |ui| {
            for i in 0..66 {
                if crate::theme::choice(
                    ui,
                    position == i,
                    format!(
                        "#{:02} · {}",
                        i + 1,
                        compact_action(&action_label(profile, layer * 66 + i))
                    ),
                )
                .clicked()
                {
                    selected = Some(i);
                    ui.close();
                }
            }
        });
    selected
}
fn target_details(ui: &mut egui::Ui, target: &Target) {
    ui.label(format!(
        "対象: {} · {}",
        target.product,
        short_path(&target.path)
    ));
    ui.label(format!(
        "識別情報: {:04x}:{:04x} / usage {:04x}:{}",
        target.identity.vendor_id,
        target.identity.product_id,
        target.identity.usage_page,
        target.identity.usage
    ));
    ui.label(format!("接続先: {}", target.path));
}
fn short_path(path: &str) -> String {
    let tail = path.chars().rev().take(24).collect::<String>();
    tail.chars().rev().collect()
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
