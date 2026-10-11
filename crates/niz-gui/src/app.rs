use crate::{
    controller::{Activation, Controller, ReadChoice, TrialApproval},
    editor::{Mode, TriggerGuide, action_label, exact_trial, trigger_guide},
    keyboard::{self, KeyboardState},
    protocol::Target,
    worker::{ChannelPort, spawn},
};
use eframe::egui::{self, Color32, RichText};
use niz_core::{Profile, key_name};
use niz_device::{CalibrationPhase, UnlockOutcome};
use std::path::PathBuf;
#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Assignment,
    Calibration,
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
    calibration_ack: bool,
    a_test: String,
    keyboard: KeyboardState,
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
        cc.egui_ctx.set_theme(egui::Theme::Light);
        let mut style = (*cc.egui_ctx.global_style()).clone();
        style.spacing.item_spacing = egui::vec2(10., 8.);
        style.spacing.button_padding = egui::vec2(14., 9.);
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(16.));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(16.));
        style.visuals.selection.bg_fill = Color32::from_rgb(42, 105, 87);
        style.visuals.selection.stroke.color = Color32::WHITE;
        cc.egui_ctx.set_global_style(style);
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
            calibration_ack: false,
            a_test: String::new(),
            keyboard: KeyboardState::default(),
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
    fn assignment(&mut self, ui: &mut egui::Ui) {
        ui.heading("キーの出力を変更");
        let operation_blocked = self.controller.blocked();
        let blocked = operation_blocked || self.confirm.is_some();
        // Remember popup ownership before selectors can close it this frame.
        let popup_was_open = egui::Popup::is_any_open(ui.ctx());
        let Some(e) = self.controller.editor.as_mut() else {
            ui.add_space(16.);
            ui.label("上でデバイスを選び、読み込んでください。");
            ui.small("保存した設定ファイルからオフラインで編集することもできます。");
            return;
        };
        let mut edit = false;
        let mut trial = false;
        ui.add_enabled_ui(!blocked,|ui| {
            let mut mode = e.selection.mode;
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut mode,Mode::Reassign,"出力を変更");
                ui.selectable_value(&mut mode,Mode::Swap,"2つの位置を交換");
            });
            self.keyboard.set_mode(e, mode);
            let mut layer = e.selection.layer;
            ui.horizontal_wrapped(|ui| {
                ui.label("レイヤー");
                for (i,label) in ["通常","右 Fn","左 Fn"].iter().enumerate() { ui.selectable_value(&mut layer,i,*label); }
            });
            e.select_layer(layer);
            ui.group(|ui| {
                ui.columns(2,|columns| {
                    columns[0].strong("From · 変更するキーの位置");
                    if let Some(slot) = slot_selector(&mut columns[0],"from",e.selection.from,&e.draft,e.selection.layer) { e.select_from(slot); }
                    columns[0].small(format!("刻印（公式配列）: {}", keyboard::key_view(e, e.selection.from).legend));
                    match e.selection.mode {
                        Mode::Reassign => {
                            columns[1].strong("To · 新しい出力");
                            egui::ComboBox::from_id_salt("output").width(230.).selected_text(if e.output_ready() { key_name(e.selection.to_code) } else { "現在の設定を保持" }).show_ui(&mut columns[1],|ui| {
                                for code in (1..=74).chain([156,166]) {
                                    if ui.selectable_label(e.output_ready() && e.selection.to_code == code,key_name(code)).clicked() {
                                        e.choose_output(code);
                                        ui.close();
                                    }
                                }
                            });
                        }
                        Mode::Swap => {
                            columns[1].strong("To · 交換先の位置");
                            if let Some(slot) = slot_selector(&mut columns[1],"to",e.selection.to_position,&e.draft,e.selection.layer) { e.selection.to_position = slot; }
                            columns[1].small(format!("刻印（公式配列）: {}", keyboard::key_view(e, e.selection.to_position).legend));
                        }
                    }
                });
                let next=e.proposed();
                match &next {
                    Ok(next) => {
                        let changes=e.draft.differences(next).unwrap_or_default();
                        if changes.is_empty() { ui.small("変更なし · 現在と同じ出力です。"); }
                        else {
                            let from=e.selection.layer*66+e.selection.from;
                            ui.label(format!("位置 #{}: {} → {}",e.selection.from+1,action_label(&e.draft,from),action_label(next,from)));
                            if e.selection.mode==Mode::Swap {
                                let to=e.selection.layer*66+e.selection.to_position;
                                ui.label(format!("位置 #{}: {} → {}",e.selection.to_position+1,action_label(&e.draft,to),action_label(next,to)));
                            }
                            if changes.iter().any(|i|i/66!=e.selection.layer) {ui.colored_label(warning(),"Fn の移動は3レイヤーすべてに反映されます。実機での適用は未対応です。");}
                        }
                    }
                    Err(_) if !e.output_ready() && e.selection.mode == Mode::Reassign => {ui.small("現在の設定はそのまま保持されます。変更するときだけ、新しい出力を選んでください。");}
                    Err(_) => {ui.colored_label(warning(),"この割り当ては変更できません。既存のデータを保持します。");}
                }
                if ui.add_enabled(next.as_ref().is_ok_and(|p|p.records!=e.draft.records),egui::Button::new("変更を追加")).clicked() {edit=true;}
            });
            keyboard::show(ui, e, &mut self.keyboard,
                operation_blocked, self.confirm.is_some(), popup_was_open || egui::Popup::is_any_open(ui.ctx()));
            ui.horizontal(|ui| {
                if ui.add_enabled(e.can_undo(),egui::Button::new("元に戻す")).clicked() {e.undo();}
                if ui.add_enabled(e.can_redo(),egui::Button::new("やり直す")).clicked() {e.redo();}
            });
        });
        let changes = e.preview().unwrap_or_default();
        ui.separator();
        ui.strong(if e.dirty() {
            format!("まだデバイスに適用していません · {}件", changes.len())
        } else {
            "未適用の変更はありません".into()
        });
        egui::ScrollArea::vertical()
            .id_salt("changes")
            .max_height(100.)
            .show(ui, |ui| {
                for i in changes {
                    ui.label(change_label(&e.baseline, &e.draft, i));
                }
            });
        let gate = exact_trial(&e.baseline, &e.draft, e.selection);
        if ui
            .add_enabled(
                !blocked && self.controller.live && e.dirty() && gate.is_ok(),
                egui::Button::new(if self.demo {
                    "デモで適用を確認…"
                } else {
                    "変更を確認して適用…"
                }),
            )
            .clicked()
        {
            trial = true;
        }
        ui.small(if !self.controller.live {"適用するには、選択したデバイスを読み込んでください。"} else if gate.is_err() {"この編集内容はオフラインで保存できます。実機での試験適用は基本キー1つの変更、または2つの交換に限られます。"} else {"実機での適用は試験機能です。確認画面で変更内容と対象を確認してください。"});
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
    fn calibration(&mut self, ui: &mut egui::Ui) {
        ui.heading("Aキーの反応を確認・調整");
        ui.group(|ui| {
            ui.strong("まず、キーボードのトリガー設定を切り替える");
            ui.label("右 Fn を押しながら、刻印 7/& のキーを一度押します。キーを離して、Aを軽く押したときの反応を比べてください。");
            ui.small("現在の段階はPCから取得できません。説明書では、電源を切ると既定の設定に戻ります。");
            let guide=trigger_guide(self.controller.editor.as_ref().map(|e|&e.baseline),self.controller.live);
            ui.small(match guide {
                TriggerGuide::CapturedShortcut=>"読み込んだ設定では、右Fn＋7の切替アクションを確認できました。",
                TriggerGuide::CustomMapping=>"読み込んだ設定では、この位置に切替アクションがありません。上の説明は説明書の標準操作です。",
                TriggerGuide::UnverifiedTarget=>"上の説明はXRGB BLEの説明書の標準操作です。現在の割り当ては未確認です。",
            });
            ui.add_enabled_ui(!self.controller.blocked(),|ui| {
                ui.label("キーボードの入力確認（センサー測定ではありません）");
                ui.add(egui::TextEdit::singleline(&mut self.a_test).hint_text("ここをクリックして、Aを軽く何度か押してください").desired_width(f32::INFINITY));
                if ui.small_button("入力を消す").clicked() {self.a_test.clear();}
            });
            ui.small("入力はこの画面内だけに保持され、ファイルに保存されません。この案内はデバイスにコマンドを送りません。");
        });
        ui.add_space(4.);
        ui.strong("Aキーのキャリブレーション · 実験的");
        ui.label(
            "トリガー設定の切替とは別の操作です。アナログの調整はバックアップで元に戻せません。",
        );
        if let Some(target) = &self.controller.attached_target {
            ui.small(format!(
                "対象: {} · {}",
                target.product,
                short_path(&target.path)
            ));
        }
        let phase = self.controller.calibration.as_ref().map(|s| s.phase);
        let pending = self.controller.pending.is_some();
        match phase {
            None
            | Some(
                CalibrationPhase::Complete | CalibrationPhase::Aborted | CalibrationPhase::Failed,
            ) => {
                ui.group(|ui| {
                    ui.strong("1. すべてのキーを離す");
                    ui.label("キーボードに触れず、マウスで開始します。");
                    ui.add_enabled_ui(!self.controller.blocked(), |ui| {
                        ui.checkbox(
                            &mut self.calibration_ack,
                            "実験的な操作で、アナログの調整は復元できないことを理解しました",
                        );
                    });
                    let enabled = self.controller.live
                        && !pending
                        && !self.controller.blocked()
                        && self.calibration_ack;
                    if ui
                        .add_enabled(enabled, egui::Button::new("全キーを離して開始"))
                        .clicked_by(egui::PointerButton::Primary)
                    {
                        let result = self
                            .controller
                            .begin(self.backup.clone(), Activation::Pointer);
                        self.result(result);
                    }
                });
                if !self.controller.live {
                    ui.small("開始するには、選択したデバイスを読み込んでください。");
                }
            }
            Some(CalibrationPhase::AwaitingHeldKeys) => {
                ui.group(|ui| {
                    ui.strong("2. 刻印 A のキーを押し続ける");
                    ui.label("Aだけを物理的に押し続け、マウスで次に進みます。画面のキー選択は送信しません。");
                    if ui.add_enabled(!pending&&!self.controller.close_requested,egui::Button::new("Aを押したまま、マウスで続ける")).clicked_by(egui::PointerButton::Primary) {let result=self.controller.held(Activation::Pointer);self.result(result);}
                });
            }
            Some(CalibrationPhase::AwaitingFinish) => {
                ui.group(|ui| {
                    ui.strong("3. Aを離し、すべてのキーを離して終了");
                    if ui
                        .add_enabled(
                            !pending && !self.controller.close_requested,
                            egui::Button::new("すべてのキーを離して終了"),
                        )
                        .clicked_by(egui::PointerButton::Primary)
                    {
                        let result = self.controller.finish(Activation::Pointer);
                        self.result(result);
                    }
                });
            }
            Some(_) => {
                ui.label("デバイスの処理を待っています。");
            }
        }
        if self.controller.calibration_active() {
            ui.horizontal_wrapped(|ui| {
                ui.spinner();
                ui.label(if pending {
                    "処理の完了を待っています。"
                } else {
                    "調整中は他のデバイス操作を行えません。"
                });
                if ui.button("中止を要求").clicked() {
                    self.controller.cancel_calibration();
                }
            });
            ui.small("Enter・Spaceでは開始しません。中止や終了は、実行中の処理と安全な後処理が返るまで待ちます。");
        }
        if let Some(status) = &self.controller.calibration {
            if status.phase == CalibrationPhase::Complete {
                ui.label("調整の手順が完了しました。タイピングとAの反応を実際に確認してください。");
            }
            if matches!(
                status.phase,
                CalibrationPhase::Aborted | CalibrationPhase::Failed
            ) {
                ui.colored_label(warning(),"調整が中断しました。ロック解除の結果を詳細で確認し、次の操作の前に接続し直してください。");
            }
            egui::CollapsingHeader::new("調整の結果・対象・バックアップ").show(ui, |ui| {
                if let Some(target) = &self.controller.attached_target {
                    target_details(ui, target);
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
            egui::CollapsingHeader::new("対象と注意点").show(ui,|ui| {
                if let Some(target)=&self.controller.attached_target {target_details(ui,target);}
                ui.label("XRGB V1.0.48 / 0483:502aのみ。実機のアナログ動作と保存は未検証です。");
                ui.label("ソフトウェアはキーが離れているかを測定しません。キーのIDや選択マスクも送信しません。");
            });
        }
    }
    fn confirmation(&mut self, ctx: &egui::Context) {
        let Some(confirm) = self.confirm.take() else {
            return;
        };
        let mut keep = true;
        let mut action = None;
        egui::Window::new("内容を確認").collapsible(false).resizable(false).default_width(520.).show(ctx,|ui| {
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
                    target_details(ui,&approval.target);ui.small(&approval.baseline.firmware);
                    for i in approval.baseline.differences(&approval.draft).unwrap_or_default() {ui.label(change_label(&approval.baseline,&approval.draft,i));}
                    ui.small(format!("バックアップ先: {}",approval.backup.display()));
                    ui.label(if self.demo {"デバイスには書き込まないシミュレーションです。"} else {"変更前に設定と照明を保存します。実際の出力と電源を切った後の保存は未検証です。"});
                    if ui.button(if self.demo {"デモで実行"} else {"確認した対象に適用"}).clicked() {action=Some(1);keep=false;}
                }
            }
            if ui.button("キャンセル").clicked() {keep=false;}
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
        ui.add_enabled_ui(!self.controller.blocked(), |ui| {
            ui.horizontal_wrapped(|ui| {
                let mut selected = self.controller.selected_path.clone();
                egui::ComboBox::from_id_salt("devices")
                    .width(300.)
                    .selected_text(
                        self.controller
                            .selected_device()
                            .map(|d| {
                                format!(
                                    "{} · {}",
                                    if d.product.is_empty() {
                                        "NIZ keyboard"
                                    } else {
                                        &d.product
                                    },
                                    short_path(&d.path)
                                )
                            })
                            .unwrap_or_else(|| "デバイスを選択".into()),
                    )
                    .show_ui(ui, |ui| {
                        for device in &self.controller.devices {
                            if ui
                                .selectable_value(
                                    &mut selected,
                                    Some(device.path.clone()),
                                    format!("{} · {}", device.product, short_path(&device.path)),
                                )
                                .clicked()
                            {
                                ui.close();
                            }
                        }
                    });
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
                        egui::Button::new("読み込む"),
                    )
                    .clicked()
                {
                    self.read_selected();
                }
            });
        });
        if self.controller.devices.is_empty() && self.controller.pending.is_none() {
            ui.small("デバイスが見つかりません。USB接続を確認し、再検索してください。");
        }
        if let Some(target) = &self.controller.attached_target {
            egui::CollapsingHeader::new(if self.controller.live {
                "読み込んだデバイス"
            } else {
                "最後に読み込んだデバイス（現在は未接続扱い）"
            })
            .show(ui, |ui| {
                target_details(ui, target);
                if let Some(editor) = &self.controller.editor {
                    ui.small(&editor.baseline.firmware);
                }
            });
        }
    }
}
impl eframe::App for App {
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
        egui::Frame::central_panel(root.style()).inner_margin(20.).show(root,|ui| {
            egui::ScrollArea::vertical().show(ui,|ui| {
                ui.horizontal_wrapped(|ui| {ui.heading("NIZ Tools");ui.small("キーの割り当てと反応の調整");});
                if self.demo {ui.colored_label(warning(),RichText::new("DEMO · 実際のキーボードには接続しません").strong());}
                self.device_controls(ui);
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut self.view,View::Assignment,"キーの割り当て");
                    ui.selectable_value(&mut self.view,View::Calibration,"Aキーの反応・調整");
                });ui.separator();
                match self.view {View::Assignment=>self.assignment(ui),View::Calibration=>self.calibration(ui)}
                ui.separator();ui.add_enabled_ui(!self.controller.blocked(), |ui| {
                    egui::CollapsingHeader::new("ファイル・バックアップ").show(ui, |ui| self.files(ui));
                });
                if self.controller.pending.is_some() {ui.horizontal(|ui| {ui.spinner();ui.label("デバイスの処理を待っています…");});}
                if self.controller.failure.is_some() {ui.colored_label(warning(),"処理を完了できませんでした。編集内容は保持しています。操作の記録を確認してください。");}
                if !self.controller.message.is_empty() {
                    egui::CollapsingHeader::new("操作の記録・詳細").show(ui,|ui| {
                        if self.demo {ui.small("シミュレーションの結果です。実機での動作確認ではありません。");}
                        ui.label(&self.controller.message);
                        if let Some(failure)=&self.controller.failure {
                            ui.label(format!("変更する出力の試行: {} / 保持した受信パケット: {}",failure.began_write,failure.raw_reports.len()));
                            if let Some(path)=&failure.backup {ui.small(format!("バックアップ: {}",path.display()));}
                            if let Some(status)=&failure.calibration {ui.label(format!("元の段階: {:?} / ロック解除: {:?} / トレース: {}パケット",status.failure_phase,status.unlock,status.trace.len()));}
                        }
                    });
                }
            });
        });
        self.confirmation(&ctx);
    }
}
fn warning() -> Color32 {
    Color32::from_rgb(154, 88, 22)
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
        .width(230.)
        .selected_text(format!(
            "位置 #{:02} · 現在 {}",
            position + 1,
            action_label(profile, layer * 66 + position)
        ))
        .show_ui(ui, |ui| {
            for i in 0..66 {
                if ui
                    .selectable_label(
                        position == i,
                        format!(
                            "位置 #{:02} · 現在 {}",
                            i + 1,
                            action_label(profile, layer * 66 + i)
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
    ui.strong(format!(
        "対象: {} · {}",
        target.product,
        short_path(&target.path)
    ));
    ui.small(format!(
        "識別情報: {:04x}:{:04x} / usage {:04x}:{}",
        target.identity.vendor_id,
        target.identity.product_id,
        target.identity.usage_page,
        target.identity.usage
    ));
    ui.small(format!("接続先: {}", target.path));
}
fn short_path(path: &str) -> String {
    let tail = path.chars().rev().take(24).collect::<String>();
    tail.chars().rev().collect()
}
