use crate::{
    editor::{Editor, exact_trial},
    protocol::*,
};
use niz_core::Profile;
use niz_device::{
    CalibrationPhase, CalibrationStatus, CalibrationValues, RawThresholds, SensitivityStatus,
    supports_raw_thresholds,
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadChoice {
    Retain,
    Discard,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activation {
    Pointer,
    Keyboard,
}
#[derive(Clone)]
pub struct TrialApproval {
    pub target: Target,
    pub backup: PathBuf,
    pub baseline: Profile,
    pub draft: Profile,
    pub selection: crate::editor::Selection,
    generation: u64,
}
struct Pending {
    ticket: Ticket,
    command: Command,
    read_choice: Option<ReadChoice>,
    target: Option<Target>,
    source_baseline: Option<Profile>,
}
pub struct RawThresholdRead {
    pub snapshot: RawThresholds,
    pub target: Target,
    pub generation: u64,
    pub read_at: std::time::SystemTime,
    pub source: RawThresholdSource,
    baseline: Profile,
}
pub struct CalibrationValueRead {
    pub snapshot: CalibrationValues,
    pub target: Target,
    pub generation: u64,
    pub read_at: std::time::SystemTime,
    pub source: RawThresholdSource,
    baseline: Profile,
}
pub struct Controller<P: Port> {
    pub port: P,
    pub editor: Option<Editor>,
    pub generation: u64,
    pub pending: Option<Ticket>,
    pub live: bool,
    pub calibration: Option<CalibrationStatus>,
    pub sensitivity: Option<SensitivityStatus>,
    pub cancel: Arc<AtomicBool>,
    pub message: String,
    pub close_requested: bool,
    pub ready_close: bool,
    pub devices: Vec<niz_device::DeviceDescriptor>,
    pub failure: Option<Failure>,
    pub selected_path: Option<String>,
    pub attached_target: Option<Target>,
    raw_thresholds: Option<RawThresholdRead>,
    calibration_values: Option<CalibrationValueRead>,
    next_operation: u64,
    operation: Option<Pending>,
    editor_path: Option<String>,
    pub(crate) cancel_requested: bool,
}
impl<P: Port> Controller<P> {
    pub fn new(port: P) -> Self {
        Self {
            port,
            editor: None,
            generation: 0,
            pending: None,
            live: false,
            calibration: None,
            sensitivity: None,
            cancel: Arc::new(AtomicBool::new(false)),
            message: String::new(),
            close_requested: false,
            ready_close: false,
            devices: vec![],
            failure: None,
            selected_path: None,
            attached_target: None,
            raw_thresholds: None,
            calibration_values: None,
            next_operation: 0,
            operation: None,
            editor_path: None,
            cancel_requested: false,
        }
    }
    pub fn calibration_active(&self) -> bool {
        self.calibration.as_ref().is_some_and(|s| {
            !matches!(
                s.phase,
                CalibrationPhase::Complete | CalibrationPhase::Aborted | CalibrationPhase::Failed
            )
        }) || self.operation.as_ref().is_some_and(|p| {
            matches!(
                p.command,
                Command::Begin { .. }
                    | Command::BeginHeld { .. }
                    | Command::Held
                    | Command::Finish
                    | Command::Cancel
            )
        })
    }
    pub fn blocked(&self) -> bool {
        self.pending.is_some() || self.calibration_active() || self.close_requested
    }
    fn available(&self) -> Result<(), String> {
        if self.blocked() {
            Err("Wait for the owned device operation or calibration cleanup".into())
        } else {
            Ok(())
        }
    }
    fn dispatch(&mut self, command: Command, choice: Option<ReadChoice>) -> Result<(), String> {
        if self.pending.is_some() {
            return Err("An operation is already pending".into());
        }
        self.next_operation += 1;
        let ticket = Ticket {
            generation: self.generation,
            operation: self.next_operation,
        };
        let target = match &command {
            Command::Read { path } => self
                .devices
                .iter()
                .find(|d| &d.path == path)
                .map(Target::from),
            Command::ReadRawThresholds
            | Command::ReadCalibrationValues
            | Command::ReadCurrentSettings
            | Command::AdjustSensitivity { .. }
            | Command::Trial { .. }
            | Command::Begin { .. }
            | Command::BeginHeld { .. }
            | Command::Held
            | Command::Finish
            | Command::Cancel => self.attached_target.clone(),
            _ => None,
        };
        let source_baseline = if matches!(
            command,
            Command::ReadRawThresholds
                | Command::ReadCalibrationValues
                | Command::ReadCurrentSettings
        ) {
            self.editor.as_ref().map(|e| e.baseline.clone())
        } else {
            None
        };
        self.port.send(Request {
            ticket,
            command: command.clone(),
            target: target.clone(),
        })?;
        self.pending = Some(ticket);
        self.operation = Some(Pending {
            ticket,
            command,
            read_choice: choice,
            target,
            source_baseline,
        });
        Ok(())
    }
    pub fn selected_device(&self) -> Option<&niz_device::DeviceDescriptor> {
        let path = self.selected_path.as_ref()?;
        self.devices.iter().find(|d| &d.path == path)
    }
    pub fn select_device(&mut self, path: String) -> Result<(), String> {
        self.available()?;
        if !self.devices.iter().any(|d| d.path == path) {
            return Err("選択したデバイスが見つかりません。一覧を更新してください。".into());
        }
        if self.selected_path.as_ref() != Some(&path) {
            self.raw_thresholds = None;
            self.calibration_values = None;
            self.calibration = None;
            self.sensitivity = None;
            self.selected_path = Some(path);
            self.live = false;
            self.message="デバイスを切り替えました。編集内容は保持しています。選択したデバイスを読み込んでください。".into();
        }
        Ok(())
    }
    fn reconcile_devices(&mut self, devices: Vec<niz_device::DeviceDescriptor>) {
        if self.selected_path.is_none() && self.attached_target.is_none() {
            self.selected_path = devices.first().map(|d| d.path.clone());
        }
        if self
            .selected_path
            .as_ref()
            .is_some_and(|path| !devices.iter().any(|d| &d.path == path))
        {
            self.selected_path = None;
            self.calibration = None;
            self.sensitivity = None;
            self.live = false;
            self.message="選択したデバイスが見つかりません。編集内容は保持しています。接続を確認して読み込んでください。".into();
        }
        if self
            .attached_target
            .as_ref()
            .is_some_and(|target| !devices.iter().any(|d| Target::from(d) == *target))
        {
            self.calibration = None;
            self.sensitivity = None;
            self.live = false;
        }
        self.devices = devices;
        if !self.live {
            self.raw_thresholds = None;
            self.calibration_values = None;
        }
    }
    fn changing_target(&self) -> Result<&Target, String> {
        let target = self
            .attached_target
            .as_ref()
            .ok_or("先にデバイスを読み込んでください。")?;
        if !self.live
            || self.selected_path.as_ref() != Some(&target.path)
            || (!self.devices.is_empty()
                && self
                    .selected_device()
                    .is_none_or(|d| Target::from(d) != *target))
        {
            return Err(
                "選択したデバイスを再度読み込んでください。編集中の内容は保持されています。".into(),
            );
        }
        Ok(target)
    }
    pub fn can_read_raw_thresholds(&self) -> bool {
        !self.blocked()
            && self.changing_target().is_ok_and(|target| {
                self.editor.as_ref().is_some_and(|e| {
                    target.identity == e.baseline.identity
                        && supports_raw_thresholds(&e.baseline.identity, &e.baseline.firmware)
                })
            })
    }
    pub fn raw_thresholds(&self) -> Option<&RawThresholdRead> {
        self.raw_thresholds.as_ref().filter(|read| {
            self.live
                && !self.close_requested
                && self.generation == read.generation
                && self
                    .changing_target()
                    .is_ok_and(|target| target == &read.target)
                && self.editor.as_ref().is_some_and(|e| {
                    e.baseline.identity == read.baseline.identity
                        && e.baseline.firmware == read.baseline.firmware
                        && e.baseline.records == read.baseline.records
                })
        })
    }
    pub fn can_read_calibration_values(&self) -> bool {
        self.can_read_raw_thresholds()
    }
    pub fn can_read_current_settings(&self) -> bool {
        self.can_read_raw_thresholds()
    }
    pub fn read_current_settings(&mut self) -> Result<(), String> {
        self.available()?;
        if !self.can_read_current_settings() {
            return Err(
                "現在の設定の読み込みには、対象デバイスの新しい読み込みが必要です。".into(),
            );
        }
        self.dispatch(Command::ReadCurrentSettings, None)?;
        self.raw_thresholds = None;
        self.calibration_values = None;
        self.failure = None;
        self.message = "現在の設定を読み込み中".into();
        Ok(())
    }
    pub fn current_settings_pending(&self) -> bool {
        self.operation
            .as_ref()
            .is_some_and(|op| matches!(op.command, Command::ReadCurrentSettings))
    }
    pub fn calibration_values(&self) -> Option<&CalibrationValueRead> {
        self.calibration_values.as_ref().filter(|read| {
            self.live
                && !self.close_requested
                && self.generation == read.generation
                && self
                    .changing_target()
                    .is_ok_and(|target| target == &read.target)
                && self.editor.as_ref().is_some_and(|e| {
                    e.baseline.identity == read.baseline.identity
                        && e.baseline.firmware == read.baseline.firmware
                        && e.baseline.records == read.baseline.records
                })
        })
    }
    pub fn read_calibration_values(&mut self) -> Result<(), String> {
        self.available()?;
        if !self.can_read_calibration_values() {
            return Err(
                "Calibration values require the exact fresh XRGB V1.0.48 device baseline".into(),
            );
        }
        self.calibration_values = None;
        self.raw_thresholds = None;
        self.failure = None;
        self.dispatch(Command::ReadCalibrationValues, None)?;
        self.message = "校正値を読み込み中".into();
        Ok(())
    }
    pub fn calibration_values_pending(&self) -> bool {
        self.operation
            .as_ref()
            .is_some_and(|op| matches!(op.command, Command::ReadCalibrationValues))
    }
    pub fn read_raw_thresholds(&mut self) -> Result<(), String> {
        self.available()?;
        if !self.can_read_raw_thresholds() {
            return Err(
                "Read raw thresholds requires the exact fresh XRGB V1.0.48 device baseline".into(),
            );
        }
        self.raw_thresholds = None;
        self.calibration_values = None;
        self.failure = None;
        self.dispatch(Command::ReadRawThresholds, None)?;
        self.message = "しきい値を読み込み中".into();
        Ok(())
    }
    pub fn sensitivity_pending(&self) -> bool {
        self.operation
            .as_ref()
            .is_some_and(|op| matches!(op.command, Command::AdjustSensitivity { .. }))
    }
    pub fn raw_thresholds_pending(&self) -> bool {
        self.operation
            .as_ref()
            .is_some_and(|op| matches!(op.command, Command::ReadRawThresholds))
    }
    pub fn prepare_trial(&self, backup: PathBuf) -> Result<TrialApproval, String> {
        self.available()?;
        let target = self.changing_target()?.clone();
        let e = self.editor.as_ref().ok_or("No profile")?;
        exact_trial(&e.baseline, &e.draft, e.selection)?;
        Ok(TrialApproval {
            target,
            backup,
            baseline: e.baseline.clone(),
            draft: e.draft.clone(),
            selection: e.selection,
            generation: self.generation,
        })
    }
    pub fn approve_trial(&mut self, a: TrialApproval) -> Result<(), String> {
        self.available()?;
        if self.changing_target()? != &a.target
            || self.generation != a.generation
            || self.editor.as_ref().is_none_or(|e| {
                e.draft.records != a.draft.records
                    || e.baseline.records != a.baseline.records
                    || e.selection != a.selection
            })
        {
            return Err(
                "確認中にデバイスか編集内容が変わりました。内容を確認し直してください。".into(),
            );
        }
        self.raw_thresholds = None;
        self.calibration_values = None;
        self.sensitivity = None;
        self.dispatch(
            Command::Trial {
                draft: a.draft,
                selection: a.selection,
                backup: a.backup,
            },
            None,
        )
    }
    pub fn list(&mut self) -> Result<(), String> {
        self.available()?;
        self.dispatch(Command::List, None)
    }
    pub fn read(&mut self, path: String, choice: ReadChoice) -> Result<(), String> {
        self.available()?;
        if !self.devices.is_empty() && !self.devices.iter().any(|d| d.path == path) {
            return Err("選択したデバイスが一覧にありません。接続を確認してください。".into());
        }
        self.selected_path = Some(path.clone());
        self.raw_thresholds = None;
        self.calibration_values = None;
        self.calibration = None;
        self.sensitivity = None;
        self.generation += 1;
        self.live = false;
        self.dispatch(Command::Read { path }, Some(choice))
    }
    pub fn trial(&mut self, backup: PathBuf) -> Result<(), String> {
        let approval = self.prepare_trial(backup)?;
        self.approve_trial(approval)
    }
    pub fn adjust_sensitivity(
        &mut self,
        backup: PathBuf,
        coefficient: u8,
        activation: Activation,
    ) -> Result<(), String> {
        self.available()?;
        if activation != Activation::Pointer {
            return Err("Sensitivity requires a mouse or trackpad click".into());
        }
        if !(10..=40).contains(&coefficient) {
            return Err("Sensitivity coefficient must be in 10..40".into());
        }
        let target = self.changing_target()?;
        let baseline = &self.editor.as_ref().ok_or("No profile")?.baseline;
        if target.identity != baseline.identity
            || !supports_raw_thresholds(&baseline.identity, &baseline.firmware)
            || baseline.group_count() != 3
        {
            return Err("Sensitivity requires the exact fresh XRGB V1.0.48 baseline".into());
        }
        self.cancel.store(false, Ordering::Release);
        self.cancel_requested = false;
        self.dispatch(
            Command::AdjustSensitivity {
                backup,
                coefficient,
            },
            None,
        )?;
        self.raw_thresholds = None;
        self.calibration_values = None;
        self.sensitivity = None;
        self.calibration = None;
        self.failure = None;
        self.live = false;
        self.message = "全体の反応を適用中…".into();
        Ok(())
    }
    pub fn begin(&mut self, backup: PathBuf, a: Activation) -> Result<(), String> {
        self.prepare_calibration(a)?;
        self.dispatch(Command::Begin { backup }, None)?;
        self.live = false;
        self.message="Preparing durable backup and released calibration. Wait for the device operation to return.".into();
        Ok(())
    }
    pub fn begin_held(
        &mut self,
        backup: PathBuf,
        position: usize,
        a: Activation,
    ) -> Result<(), String> {
        if position >= 66 {
            return Err("Select a physical key in 0..65".into());
        }
        self.prepare_calibration(a)?;
        self.dispatch(Command::BeginHeld { backup, position }, None)?;
        self.live = false;
        self.message =
            "Preparing held-key calibration with all keys released; retaining release calibration."
                .into();
        Ok(())
    }
    fn prepare_calibration(&mut self, a: Activation) -> Result<(), String> {
        self.available()?;
        if a != Activation::Pointer {
            return Err("Calibration requires a mouse or trackpad click".into());
        }
        self.changing_target()?;
        let p = &self.editor.as_ref().ok_or("No profile")?.baseline;
        if p.firmware != "66EC(XRGB)BLe;V1.0.48;V1.0;"
            || p.identity
                != (niz_core::DeviceIdentity {
                    vendor_id: 0x0483,
                    product_id: 0x502a,
                    usage_page: 0x8c,
                    usage: 1,
                })
            || p.group_count() != 3
        {
            return Err(
                "Calibration is experimental for exact XRGB V1.0.48 / 0483:502a only".into(),
            );
        }
        self.raw_thresholds = None;
        self.calibration_values = None;
        self.cancel.store(false, Ordering::Release);
        self.cancel_requested = false;
        self.failure = None;
        self.calibration = None;
        self.sensitivity = None;
        Ok(())
    }
    fn stage(&self, phase: CalibrationPhase, a: Activation) -> Result<(), String> {
        if a != Activation::Pointer {
            return Err("Calibration requires a mouse or trackpad click".into());
        }
        if self.pending.is_some() || self.close_requested || self.cancel_requested {
            return Err("Wait for the owned operation/cleanup".into());
        }
        if self.calibration.as_ref().is_none_or(|s| s.phase != phase) {
            return Err("This calibration stage is not ready".into());
        }
        Ok(())
    }
    pub fn held(&mut self, a: Activation) -> Result<(), String> {
        self.stage(CalibrationPhase::AwaitingHeldKeys, a)?;
        self.dispatch(Command::Held, None)
    }
    pub fn finish(&mut self, a: Activation) -> Result<(), String> {
        self.stage(CalibrationPhase::AwaitingFinish, a)?;
        self.dispatch(Command::Finish, None)
    }
    pub fn cancel_calibration(&mut self) {
        if !self.calibration_active() {
            return;
        }
        self.cancel_requested = true;
        self.cancel.store(true, Ordering::Release);
        self.message="Cancellation requested. A pending physical send is not canceled; waiting for the worker before safe cleanup.".into();
        self.queue_cleanup();
    }
    fn queue_cleanup(&mut self) {
        if self.pending.is_none()
            && self.calibration.as_ref().is_some_and(|s| {
                matches!(
                    s.phase,
                    CalibrationPhase::AwaitingHeldKeys | CalibrationPhase::AwaitingFinish
                )
            })
            && let Err(e) = self.dispatch(Command::Cancel, None)
        {
            self.message = e;
        }
    }
    pub fn close(&mut self) {
        self.raw_thresholds = None;
        self.calibration_values = None;
        self.close_requested = true;
        if self.calibration_active() {
            self.cancel_calibration();
        } else if self.pending.is_none() {
            self.ready_close = true;
        } else {
            if self
                .operation
                .as_ref()
                .is_some_and(|op| matches!(op.command, Command::AdjustSensitivity { .. }))
            {
                self.cancel.store(true, Ordering::Release);
                self.cancel_requested = true;
            }
            self.message = "Waiting for the device operation to return before closing.".into();
        }
    }
    pub fn process(&mut self, event: Event) {
        if self.pending != Some(event.ticket) || event.ticket.generation != self.generation {
            return;
        }
        let Some(op) = self.operation.take() else {
            return;
        };
        debug_assert_eq!(op.ticket, event.ticket);
        self.pending = None;
        let raw_reply = matches!(event.response, Response::RawThresholds { .. });
        let raw_command = matches!(op.command, Command::ReadRawThresholds);
        let calibration_reply = matches!(event.response, Response::CalibrationValues { .. });
        let calibration_command = matches!(op.command, Command::ReadCalibrationValues);
        let current_reply = matches!(event.response, Response::CurrentSettings { .. });
        let current_command = matches!(op.command, Command::ReadCurrentSettings);
        if (raw_reply && !raw_command)
            || (raw_command && !raw_reply && !matches!(event.response, Response::Error(_)))
            || (calibration_reply && !calibration_command)
            || (calibration_command
                && !calibration_reply
                && !matches!(event.response, Response::Error(_)))
            || (current_reply && !current_command)
            || (current_command && !current_reply && !matches!(event.response, Response::Error(_)))
        {
            self.raw_thresholds = None;
            self.calibration_values = None;
            self.live = false;
            self.message =
                "Unexpected response for the owned raw threshold operation; read the device again."
                    .into();
            let mut failure = Failure::from(self.message.clone());
            if let Response::RawThresholds { snapshot, .. } = &event.response {
                failure.raw_reports = snapshot
                    .raw_reports()
                    .iter()
                    .map(|report| report.to_vec())
                    .collect();
            }
            if let Response::CalibrationValues { snapshot, .. } = &event.response {
                failure.raw_reports = snapshot
                    .raw_reports()
                    .iter()
                    .map(|report| report.to_vec())
                    .collect();
            }
            if let Response::CurrentSettings {
                raw, calibration, ..
            } = &event.response
            {
                failure.raw_reports = raw
                    .raw_reports()
                    .iter()
                    .chain(calibration.raw_reports())
                    .map(|report| report.to_vec())
                    .collect();
            }
            self.failure = Some(failure);
            if self.close_requested {
                self.ready_close = true;
            }
            return;
        }
        let sensitivity_reply = matches!(event.response, Response::Sensitivity(_));
        let sensitivity_command = matches!(op.command, Command::AdjustSensitivity { .. });
        if (sensitivity_reply && !sensitivity_command)
            || (sensitivity_command
                && !sensitivity_reply
                && !matches!(event.response, Response::Error(_)))
        {
            self.sensitivity = None;
            self.live = false;
            self.message =
                "Unexpected response for the owned sensitivity operation; read the device again."
                    .into();
            self.failure = Some(Failure::from(self.message.clone()));
            if self.close_requested {
                self.ready_close = true;
            }
            return;
        }
        match event.response {
            Response::Devices(devices) => self.reconcile_devices(devices),
            Response::CurrentSettings {
                raw,
                calibration,
                target,
                source,
            } => {
                let baseline = op.source_baseline.as_ref();
                let valid = !self.close_requested
                    && self.live
                    && op.target.as_ref() == Some(&target)
                    && self
                        .changing_target()
                        .is_ok_and(|current| current == &target)
                    && baseline.is_some_and(|baseline| {
                        baseline.identity == target.identity
                            && raw.identity == baseline.identity
                            && calibration.identity == baseline.identity
                            && raw.firmware == baseline.firmware
                            && calibration.firmware == baseline.firmware
                            && supports_raw_thresholds(&raw.identity, &raw.firmware)
                            && self.editor.as_ref().is_some_and(|e| {
                                e.baseline.identity == baseline.identity
                                    && e.baseline.firmware == baseline.firmware
                                    && e.baseline.records == baseline.records
                            })
                    });
                if valid {
                    let read_at = std::time::SystemTime::now();
                    let baseline = baseline.unwrap().clone();
                    self.raw_thresholds = Some(RawThresholdRead {
                        snapshot: raw,
                        target: target.clone(),
                        source,
                        generation: op.ticket.generation,
                        read_at,
                        baseline: baseline.clone(),
                    });
                    self.calibration_values = Some(CalibrationValueRead {
                        snapshot: calibration,
                        target,
                        source,
                        generation: op.ticket.generation,
                        read_at,
                        baseline,
                    });
                    self.message = match source {
                        RawThresholdSource::Device => "現在の設定を読み込みました".into(),
                        RawThresholdSource::Simulation => "DEMO · 現在の設定を表示しました".into(),
                    };
                } else {
                    self.raw_thresholds = None;
                    self.calibration_values = None;
                    self.live = false;
                    self.message =
                        "現在の設定が対象と一致しません。デバイスを読み込み直してください。".into();
                    let mut failure = Failure::from(self.message.clone());
                    failure.raw_reports = raw
                        .raw_reports()
                        .iter()
                        .chain(calibration.raw_reports())
                        .map(|report| report.to_vec())
                        .collect();
                    self.failure = Some(failure);
                }
            }
            Response::RawThresholds {
                snapshot,
                target,
                source,
            } => {
                let baseline = op.source_baseline.as_ref();
                let valid = !self.close_requested
                    && self.live
                    && op.target.as_ref() == Some(&target)
                    && self
                        .changing_target()
                        .is_ok_and(|current| current == &target)
                    && baseline.is_some_and(|baseline| {
                        baseline.identity == target.identity
                            && snapshot.identity == baseline.identity
                            && snapshot.firmware == baseline.firmware
                            && supports_raw_thresholds(&snapshot.identity, &snapshot.firmware)
                            && self.editor.as_ref().is_some_and(|e| {
                                e.baseline.identity == baseline.identity
                                    && e.baseline.firmware == baseline.firmware
                                    && e.baseline.records == baseline.records
                            })
                    });
                if valid {
                    self.raw_thresholds = Some(RawThresholdRead {
                        snapshot,
                        target,
                        source,
                        generation: op.ticket.generation,
                        read_at: std::time::SystemTime::now(),
                        baseline: baseline.unwrap().clone(),
                    });
                    self.message = match source {
                        RawThresholdSource::Device => "しきい値を読み込みました".into(),
                        RawThresholdSource::Simulation => "DEMO · しきい値を表示しました".into(),
                    };
                } else {
                    self.raw_thresholds = None;
                    self.calibration_values = None;
                    self.live = false;
                    self.message = "Raw threshold source or reply changed; discard the snapshot and read the device again.".into();
                    let mut failure = Failure::from(self.message.clone());
                    failure.raw_reports = snapshot
                        .raw_reports()
                        .iter()
                        .map(|report| report.to_vec())
                        .collect();
                    self.failure = Some(failure);
                }
            }
            Response::CalibrationValues {
                snapshot,
                target,
                source,
            } => {
                let baseline = op.source_baseline.as_ref();
                let valid = !self.close_requested
                    && self.live
                    && op.target.as_ref() == Some(&target)
                    && self
                        .changing_target()
                        .is_ok_and(|current| current == &target)
                    && baseline.is_some_and(|baseline| {
                        baseline.identity == target.identity
                            && snapshot.identity == baseline.identity
                            && snapshot.firmware == baseline.firmware
                            && supports_raw_thresholds(&snapshot.identity, &snapshot.firmware)
                            && self.editor.as_ref().is_some_and(|e| {
                                e.baseline.identity == baseline.identity
                                    && e.baseline.firmware == baseline.firmware
                                    && e.baseline.records == baseline.records
                            })
                    });
                if valid {
                    self.calibration_values = Some(CalibrationValueRead {
                        snapshot,
                        target,
                        source,
                        generation: op.ticket.generation,
                        read_at: std::time::SystemTime::now(),
                        baseline: baseline.unwrap().clone(),
                    });
                    self.message = match source {
                        RawThresholdSource::Device => "校正値を読み込みました".into(),
                        RawThresholdSource::Simulation => "DEMO · 校正値を表示しました".into(),
                    };
                } else {
                    self.calibration_values = None;
                    self.live = false;
                    self.message =
                        "Calibration value source or reply changed; read the device again.".into();
                    let mut failure = Failure::from(self.message.clone());
                    failure.raw_reports = snapshot
                        .raw_reports()
                        .iter()
                        .map(|report| report.to_vec())
                        .collect();
                    self.failure = Some(failure);
                }
            }
            Response::Read { profile, cancel } => {
                self.raw_thresholds = None;
                self.calibration_values = None;
                self.cancel = cancel;
                self.calibration = None;
                self.sensitivity = None;
                self.cancel_requested = false;
                self.failure = None;
                let path = match &op.command {
                    Command::Read { path } => Some(path.clone()),
                    _ => None,
                };
                self.attached_target = path.as_ref().map(|path| Target {
                    path: path.clone(),
                    product: op
                        .target
                        .as_ref()
                        .map(|t| t.product.clone())
                        .unwrap_or_else(|| "NIZ keyboard".into()),
                    identity: profile.identity.clone(),
                });
                if op
                    .target
                    .as_ref()
                    .is_some_and(|t| t.identity != profile.identity)
                {
                    self.live = false;
                    self.message =
                        "読み込んだデバイスの識別情報が変わりました。編集内容を保持しました。"
                            .into();
                    return;
                }
                if op.read_choice == Some(ReadChoice::Discard) || self.editor.is_none() {
                    self.editor = Some(Editor::new(profile));
                    self.editor_path = path;
                    self.live = true;
                    self.message = "Fresh device baseline read. No settings were written.".into();
                } else if self.editor_path == path
                    && self.editor.as_ref().is_some_and(|e| {
                        e.baseline.identity == profile.identity
                            && e.baseline.firmware == profile.firmware
                            && e.baseline.records == profile.records
                    })
                {
                    self.live = true;
                    self.message =
                        "Fresh baseline matched this source; draft and undo history retained."
                            .into();
                } else {
                    self.live = false;
                    self.message="Draft retained and detached: source path, firmware, identity or baseline changed. Save it, or explicitly discard and read again.".into();
                }
            }
            Response::Applied {
                profile,
                outcome,
                backup,
                snapshot: _,
            } => {
                if let (Some(e), Command::Trial { draft, .. }) = (&mut self.editor, &op.command)
                    && e.draft.records == draft.records
                {
                    e.accept_applied(profile);
                }
                // The old native token is invalidated after a changing transaction. A fresh read is explicit.
                self.live = false;
                self.message = match outcome {
                    niz_device::ApplyOutcome::NoChange => format!(
                        "No changes; no hardware write or postflight/RGB readback performed. Configuration backup: {}",
                        backup.display()
                    ),
                    niz_device::ApplyOutcome::SettingsVerified => format!(
                        "Settings readback verified. Backup: {}. Read again before another trial.",
                        backup.display()
                    ),
                    niz_device::ApplyOutcome::SettingsAndLightingVerified => format!(
                        "Settings and lighting bytes verified. Backup: {}. Physical output and persistence remain experimental; read again before another trial.",
                        backup.display()
                    ),
                };
            }
            Response::Sensitivity(s) => {
                self.live = false;
                let expected = match op.command {
                    Command::AdjustSensitivity { coefficient, .. } => coefficient,
                    _ => unreachable!(),
                };
                if s.coefficient != expected
                    || s.phase != niz_device::SensitivityPhase::Complete
                    || !s.sent
                    || !s.settings_verified
                    || !s.lighting_verified
                    || s.before.as_ref().is_none_or(|v| v.len() != 66)
                    || s.after.as_ref().is_none_or(|v| v.len() != 66)
                {
                    self.message = "Sensitivity reply was incomplete or differed from the requested coefficient; read the device again.".into();
                    let mut failure = Failure::from(self.message.clone());
                    failure.began_write = s.attempted;
                    failure.sensitivity = Some(Box::new(s.clone()));
                    self.failure = Some(failure);
                } else {
                    self.message = sensitivity_message(&s);
                }
                self.sensitivity = Some(s);
            }
            Response::Calibration(s) => {
                self.live = false;
                self.message = calibration_message(&s);
                self.calibration = Some(s);
            }
            Response::Finished(s) => {
                self.live = false;
                self.message = calibration_message(&s);
                self.calibration = Some(s);
            }
            Response::Error(e) => {
                self.raw_thresholds = None;
                self.calibration_values = None;
                if !matches!(op.command, Command::List) {
                    self.live = false;
                }
                self.message = e.message.clone();
                if let Some(s) = &e.calibration {
                    self.calibration = Some((**s).clone());
                }
                if let Some(s) = &e.sensitivity {
                    self.sensitivity = Some((**s).clone());
                }
                self.failure = Some(e);
            }
            Response::Shutdown => {
                self.raw_thresholds = None;
                self.calibration_values = None;
                self.ready_close = true;
            }
        }
        if self.cancel_requested {
            self.queue_cleanup();
        }
        if self.close_requested && self.pending.is_none() && !self.calibration_active() {
            self.ready_close = true;
        }
    }
    pub fn load(&mut self, p: Profile, discard: bool) -> Result<(), String> {
        self.available()?;
        if self.editor.as_ref().is_some_and(Editor::dirty) && !discard {
            return Err("Explicitly retain or replace the dirty draft before loading".into());
        }
        if let Some(e) = &mut self.editor {
            e.import(&p)?;
        } else {
            self.editor = Some(Editor::new(p));
            self.editor_path = None;
        }
        self.raw_thresholds = None;
        self.calibration_values = None;
        self.message =
            "Loaded into the offline draft. A file does not create a live device baseline.".into();
        Ok(())
    }
    pub fn edit(&mut self) -> Result<(), String> {
        self.available()?;
        self.editor
            .as_mut()
            .ok_or("Read or load a profile first")?
            .edit()
    }
}
pub fn calibration_message(s: &CalibrationStatus) -> String {
    if let Some(held) = &s.held_key {
        if s.phase == CalibrationPhase::AwaitingHeldKeys {
            return format!("#{} を押したまま実行してください", held.position + 1);
        }
        if s.phase == CalibrationPhase::Complete {
            return "校正が完了しました".into();
        }
    }
    match s.phase {
 CalibrationPhase::AwaitingHeldKeys=>"Released stage returned DA. Hold the affected physical key(s), then click the held-stage button with the mouse.".into(),
 CalibrationPhase::AwaitingFinish=>"Held stage returned DE. Release every key, then click Finish.".into(),
 CalibrationPhase::Complete=>"Calibration sequence completed; settings and lighting bytes were preserved. Unlock was sent, not acknowledged; verify typing and analog behavior physically. Read a fresh baseline before editing the device.".into(),
 CalibrationPhase::Aborted=>"Calibration aborted. Settings snapshots cannot roll back analog calibration. Check the unlock result and verify typing physically; reconnect before further device work.".into(),
 CalibrationPhase::Failed=>format!("Calibration failed during {:?}: {}. Unlock result: {:?}. Reconnect before further device work.",s.failure_phase,s.original_error.as_deref().unwrap_or("unknown failure"),s.unlock),
 _=>format!("Device operation in progress: {:?}",s.phase)
 }
}

pub fn sensitivity_message(s: &SensitivityStatus) -> String {
    if let (Some(before), Some(after)) = (&s.before, &s.after)
        && before.len() == 66
        && after.len() == 66
    {
        let changed = before.iter().zip(after).filter(|(b, a)| b != a).count();
        if changed == 0 {
            "適用しました · しきい値の変化なし".into()
        } else {
            format!("適用しました · {changed}キーのしきい値が変化")
        }
    } else {
        "適用結果の読み取りが完了していません".into()
    }
}
