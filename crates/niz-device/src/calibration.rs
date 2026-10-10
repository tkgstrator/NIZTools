use crate::{Error, LiveBaseline, Result, Session, SnapshotPaths};
use niz_core::Profile;
use serde::Serialize;
use std::{
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum CalibrationPhase {
    Preparing,
    Locking,
    ReleaseWaiting,
    AwaitingHeldKeys,
    HeldWaiting,
    AwaitingFinish,
    Unlocking,
    Verifying,
    Complete,
    Aborted,
    Failed,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum UnlockOutcome {
    NotAttempted,
    Sent,
    Uncertain(String),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum TraceDirection {
    Input,
    Output,
}
#[derive(Clone, Debug, Serialize)]
pub struct PacketTrace {
    pub direction: TraceDirection,
    pub timestamp_ms: u64,
    pub bytes: Vec<u8>,
    pub outcome: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct CalibrationStatus {
    pub phase: CalibrationPhase,
    pub failure_phase: Option<CalibrationPhase>,
    pub backup: Option<SnapshotPaths>,
    pub lock_may_have_been_sent: bool,
    pub release_attempted: bool,
    pub release_sent: bool,
    pub release_completed: bool,
    pub held_attempted: bool,
    pub held_sent: bool,
    pub held_completed: bool,
    pub cancellation_requested: bool,
    pub unlock: UnlockOutcome,
    pub settings_verified: bool,
    pub lighting_verified: bool,
    pub original_error: Option<String>,
    pub trace: Vec<PacketTrace>,
}
impl CalibrationStatus {
    fn preparing() -> Self {
        Self {
            phase: CalibrationPhase::Preparing,
            failure_phase: None,
            backup: None,
            lock_may_have_been_sent: false,
            release_attempted: false,
            release_sent: false,
            release_completed: false,
            held_attempted: false,
            held_sent: false,
            held_completed: false,
            cancellation_requested: false,
            unlock: UnlockOutcome::NotAttempted,
            settings_verified: false,
            lighting_verified: false,
            original_error: None,
            trace: vec![],
        }
    }
}
#[derive(Debug)]
pub struct CalibrationResult {
    pub status: CalibrationStatus,
    pub profile: Profile,
    pub lighting: Vec<u8>,
}
pub(crate) struct CalibrationRun {
    pub(crate) status: CalibrationStatus,
    profile: Profile,
    lighting: Vec<u8>,
}
impl Session {
    pub fn cancellation_flag(&self) -> Arc<AtomicBool> {
        self.cancel_flag.clone()
    }
    pub fn calibration_status(&self) -> Option<CalibrationStatus> {
        self.calibration.as_ref().map(|r| r.status.clone())
    }
    pub(crate) fn trace(
        &mut self,
        direction: TraceDirection,
        bytes: Vec<u8>,
        outcome: Option<String>,
    ) {
        let timestamp_ms = self.io.now_ms();
        if let Some(run) = self.calibration.as_mut()
            && !matches!(
                run.status.phase,
                CalibrationPhase::Complete | CalibrationPhase::Aborted | CalibrationPhase::Failed
            )
        {
            run.status.trace.push(PacketTrace {
                direction,
                timestamp_ms,
                bytes,
                outcome,
            });
        }
    }
    fn status_mut(&mut self) -> &mut CalibrationStatus {
        &mut self
            .calibration
            .as_mut()
            .expect("owned calibration run")
            .status
    }
    fn cancelled(&self) -> bool {
        self.cancel_flag.load(std::sync::atomic::Ordering::Acquire)
    }
    fn calibration_phase(&self, expected: CalibrationPhase) -> Result<()> {
        if self
            .calibration
            .as_ref()
            .is_none_or(|r| r.status.phase != expected)
        {
            return Err(Error::new(
                "calibration stage unavailable; no command was dispatched",
            ));
        }
        Ok(())
    }
    fn unlock_once(&mut self) -> Result<()> {
        if self
            .calibration
            .as_ref()
            .is_none_or(|r| r.status.unlock != UnlockOutcome::NotAttempted)
        {
            return Err(Error::new("unlock was already attempted; no retry"));
        }
        self.status_mut().unlock =
            UnlockOutcome::Uncertain("unlock output is pending; no acknowledgement exists".into());
        let mut report = niz_core::command(0xd9);
        report[2] = 1;
        let result = self.send(&report);
        self.status_mut().unlock = match &result {
            Ok(()) => UnlockOutcome::Sent,
            Err(e) => UnlockOutcome::Uncertain(e.to_string()),
        };
        result
    }
    fn calibration_error(&mut self, mut error: Error, cleanup: bool, aborted: bool) -> Error {
        let phase = self.status_mut().phase;
        self.status_mut().failure_phase = Some(phase);
        if self.status_mut().original_error.is_none() {
            self.status_mut().original_error = Some(error.to_string());
        }
        let can_unlock = cleanup
            && self.io.connected()
            && self.status_mut().lock_may_have_been_sent
            && self.status_mut().unlock == UnlockOutcome::NotAttempted;
        if can_unlock {
            let _ = self.unlock_once();
        }
        self.status_mut().phase = if aborted {
            CalibrationPhase::Aborted
        } else {
            CalibrationPhase::Failed
        };
        self.status_mut().cancellation_requested = self.cancelled();
        self.poisoned = true;
        self.last_read = None;
        self.generation += 1;
        let status = self.calibration_status().unwrap();
        error.began_write = status.lock_may_have_been_sent;
        error.backup_path = status.backup.as_ref().map(|p| p.config_path.clone());
        if error.raw_reports.is_empty() {
            error.raw_reports = status
                .trace
                .iter()
                .filter(|t| t.direction == TraceDirection::Input)
                .map(|t| t.bytes.clone())
                .collect();
        }
        if status.lock_may_have_been_sent && status.unlock != UnlockOutcome::Sent {
            error
                .message
                .push_str("; typing may remain locked; unlock not confirmed or safely sent");
        }
        error
            .message
            .push_str("; analog calibration cannot be restored from settings backups");
        error.calibration = Some(Box::new(status));
        error
    }
    fn cancellation_error(&mut self) -> Error {
        self.calibration_error(Error::new("calibration cancellation requested; completed physical commands are not rolled back"),true,true)
    }
    pub(crate) fn require_quiet_calibration_input(&mut self) -> Result<()> {
        if let Some(bytes) = self.io.queued_input().map_err(Error::new)? {
            self.trace(
                TraceDirection::Input,
                bytes.clone(),
                Some("unexpected queued input before changing command".into()),
            );
            let mut error =
                Error::new("unexpected queued calibration input; command not dispatched");
            error.raw_reports.push(bytes);
            return Err(error);
        }
        Ok(())
    }
    fn wait_calibration(&mut self, expected: u8) -> Result<()> {
        let deadline = self.io.now_ms() + 10000;
        loop {
            if self.cancelled() {
                return Err(Error::new("calibration cancellation requested"));
            }
            let remaining = deadline.saturating_sub(self.io.now_ms());
            if remaining == 0 {
                return Err(Error::new(
                    "calibration response absolute 10-second deadline exceeded",
                ));
            }
            let mut buffer = [0; 65];
            let n = self
                .io
                .read(&mut buffer, remaining.min(500) as i32)
                .map_err(Error::new)?;
            if n > 0 {
                self.trace(TraceDirection::Input, buffer[..n].to_vec(), None);
            }
            if self.cancelled() {
                return Err(Error::new("calibration cancellation requested"));
            }
            if self.io.now_ms() > deadline {
                return Err(Error::new(
                    "late calibration response beyond absolute deadline",
                ));
            }
            if n == 0 {
                continue;
            }
            if n != 64 {
                let mut e = Error::new("calibration response must be64 native bytes");
                e.raw_reports.push(buffer[..n].to_vec());
                return Err(e);
            }
            if buffer[0] != 0 || buffer[1] != expected {
                let mut e = Error::new(format!(
                    "unexpected calibration reply; expected {expected:02x}"
                ));
                e.raw_reports.push(buffer[..n].to_vec());
                return Err(e);
            }
            return Ok(());
        }
    }
    pub fn begin_calibration(
        &mut self,
        baseline: &LiveBaseline,
        directory: &Path,
    ) -> Result<CalibrationStatus> {
        self.check_baseline(baseline)?;
        self.exact_target(baseline.profile())?;
        if self
            .calibration
            .as_ref()
            .is_some_and(|r| r.status.phase == CalibrationPhase::Complete)
        {
            self.cancel_flag
                .store(false, std::sync::atomic::Ordering::Release);
        }
        self.calibration = Some(CalibrationRun {
            status: CalibrationStatus::preparing(),
            profile: baseline.profile().clone(),
            lighting: vec![],
        });
        let prepare = (|| {
            if self.cancelled() {
                return Err(Error::new(
                    "calibration cancellation requested before mutation",
                ));
            }
            let current = self.profile_inner()?;
            self.exact_target(&current)?;
            if current.records != baseline.profile().records
                || current.firmware != baseline.profile().firmware
                || current.identity != baseline.profile().identity
            {
                return Err(Error::new(
                    "external settings/firmware changed before calibration",
                ));
            }
            let lighting = self.lighting_inner()?;
            let paths = crate::backup::save_snapshot(&current, &lighting, directory, self.backup)?;
            crate::backup::verify_snapshot(&paths, &current, &lighting)?;
            self.calibration.as_mut().unwrap().profile = current;
            self.calibration.as_mut().unwrap().lighting = lighting;
            self.status_mut().backup = Some(paths);
            self.require_quiet_calibration_input()?;
            Ok(())
        })();
        if let Err(error) = prepare {
            return Err(self.calibration_error(error, false, self.cancelled()));
        }
        if self.cancelled() {
            return Err(self.cancellation_error());
        }
        self.last_read = None;
        self.generation += 1;
        self.status_mut().phase = CalibrationPhase::Locking;
        self.status_mut().lock_may_have_been_sent = true;
        if let Err(error) = self.send(&niz_core::command(0xd9)) {
            return Err(self.calibration_error(error, false, false));
        }
        if self.cancelled() {
            return Err(self.cancellation_error());
        }
        self.status_mut().phase = CalibrationPhase::ReleaseWaiting;
        if let Err(error) = self.require_quiet_calibration_input() {
            return Err(self.calibration_error(error, true, false));
        }
        if self.cancelled() {
            return Err(self.cancellation_error());
        }
        self.status_mut().release_attempted = true;
        if let Err(error) = self.send(&niz_core::command(0xdb)) {
            return Err(self.calibration_error(error, false, false));
        }
        self.status_mut().release_sent = true;
        if let Err(error) = self.wait_calibration(0xda) {
            return Err(self.calibration_error(error, true, self.cancelled()));
        }
        self.status_mut().release_completed = true;
        self.status_mut().phase = CalibrationPhase::AwaitingHeldKeys;
        Ok(self.calibration_status().unwrap())
    }
    pub fn calibrate_held_keys(&mut self) -> Result<CalibrationStatus> {
        self.calibration_phase(CalibrationPhase::AwaitingHeldKeys)?;
        if self.cancelled() {
            return Err(self.cancellation_error());
        }
        if let Err(error) = self.require_quiet_calibration_input() {
            return Err(self.calibration_error(error, true, false));
        }
        if self.cancelled() {
            return Err(self.cancellation_error());
        }
        self.status_mut().phase = CalibrationPhase::HeldWaiting;
        self.status_mut().held_attempted = true;
        if let Err(error) = self.send(&niz_core::command(0xdd)) {
            return Err(self.calibration_error(error, false, false));
        }
        self.status_mut().held_sent = true;
        if let Err(error) = self.wait_calibration(0xde) {
            return Err(self.calibration_error(error, true, self.cancelled()));
        }
        self.status_mut().held_completed = true;
        self.status_mut().phase = CalibrationPhase::AwaitingFinish;
        Ok(self.calibration_status().unwrap())
    }
    pub fn finish_calibration(&mut self) -> Result<CalibrationResult> {
        self.calibration_phase(CalibrationPhase::AwaitingFinish)?;
        if self.cancelled() {
            return Err(self.cancellation_error());
        }
        self.status_mut().phase = CalibrationPhase::Unlocking;
        if let Err(error) = self.unlock_once() {
            return Err(self.calibration_error(error, false, false));
        }
        self.status_mut().phase = CalibrationPhase::Verifying;
        let postflight = (|| {
            let current = self.profile_inner()?;
            let (rgb, rgb_raw) = self.lighting_capture_inner()?;
            let original = &self.calibration.as_ref().unwrap().profile;
            let before = &self.calibration.as_ref().unwrap().lighting;
            if current.firmware != original.firmware
                || current.identity != original.identity
                || current.records != original.records
            {
                let mut error = Error::new("calibration settings preservation check failed");
                error.raw_reports = current.reports().iter().map(|r| r.to_vec()).collect();
                return Err(error);
            }
            if rgb != *before {
                let mut error = Error::new(
                    "calibration lighting preservation check failed; RGB restore unavailable",
                );
                error.raw_reports = rgb_raw;
                return Err(error);
            }
            Ok((current, rgb))
        })();
        match postflight {
            Ok((profile, lighting)) => {
                self.status_mut().phase = CalibrationPhase::Complete;
                self.status_mut().settings_verified = true;
                self.status_mut().lighting_verified = true;
                self.last_read = None;
                self.generation += 1;
                Ok(CalibrationResult {
                    status: self.calibration_status().unwrap(),
                    profile,
                    lighting,
                })
            }
            Err(error) => Err(self.calibration_error(error, false, false)),
        }
    }
    pub fn cancel_calibration(&mut self) -> Result<CalibrationStatus> {
        if self.calibration.as_ref().is_none_or(|r| {
            !matches!(
                r.status.phase,
                CalibrationPhase::AwaitingHeldKeys | CalibrationPhase::AwaitingFinish
            )
        }) {
            return Err(Error::new(
                "calibration cancel unavailable/already terminal; no second unlock",
            ));
        }
        self.cancel_flag
            .store(true, std::sync::atomic::Ordering::Release);
        let error = self.cancellation_error();
        let status = error.calibration.unwrap();
        if status.unlock == UnlockOutcome::Sent {
            Ok(*status)
        } else {
            let mut error = Error::new(
                "calibration aborted; cleanup unlock failed or unavailable; typing may remain locked",
            );
            error.began_write = status.lock_may_have_been_sent;
            error.backup_path = status.backup.as_ref().map(|p| p.config_path.clone());
            error.calibration = Some(status);
            Err(error)
        }
    }
}
