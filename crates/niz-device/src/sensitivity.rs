use crate::{Error, LiveBaseline, PacketTrace, Result, Session, SnapshotPaths, TraceDirection};
use serde::Serialize;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SensitivityPhase {
    Preparing,
    Before,
    Sending,
    After,
    Verifying,
    Complete,
    Aborted,
    Failed,
}
#[derive(Clone, Debug, Serialize)]
pub struct SensitivityStatus {
    pub coefficient: u8,
    pub before: Option<Vec<u8>>,
    pub after: Option<Vec<u8>>,
    pub attempted: bool,
    pub sent: bool,
    pub settings_verified: bool,
    pub lighting_verified: bool,
    pub cancellation_requested: bool,
    pub backup: Option<SnapshotPaths>,
    pub phase: SensitivityPhase,
    pub failure_phase: Option<SensitivityPhase>,
    pub original_error: Option<String>,
    pub trace: Vec<PacketTrace>,
}
impl SensitivityStatus {
    pub fn preparing(coefficient: u8) -> Self {
        Self {
            coefficient,
            before: None,
            after: None,
            attempted: false,
            sent: false,
            settings_verified: false,
            lighting_verified: false,
            cancellation_requested: false,
            backup: None,
            phase: SensitivityPhase::Preparing,
            failure_phase: None,
            original_error: None,
            trace: vec![],
        }
    }
}
#[derive(Debug)]
pub struct SensitivityResult {
    pub status: SensitivityStatus,
}
impl Session {
    pub(crate) fn sensitivity_active(&self) -> bool {
        self.sensitivity.as_ref().is_some_and(|status| {
            !matches!(
                status.phase,
                SensitivityPhase::Complete | SensitivityPhase::Aborted | SensitivityPhase::Failed
            )
        })
    }
    fn sensitivity_mut(&mut self) -> &mut SensitivityStatus {
        self.sensitivity
            .as_mut()
            .expect("owned sensitivity transaction")
    }
    fn sensitivity_cancelled(&self) -> bool {
        self.cancel_flag.load(std::sync::atomic::Ordering::Acquire)
    }
    fn check_sensitivity_cancellation(&self) -> Result<()> {
        if self.sensitivity_cancelled() {
            Err(Error::new(
                "sensitivity cancellation requested; physical commands are not rolled back",
            ))
        } else {
            Ok(())
        }
    }
    pub(crate) fn require_quiet_sensitivity_input(&mut self) -> Result<()> {
        if let Some(bytes) = self.io.queued_input().map_err(Error::new)? {
            self.trace(
                TraceDirection::Input,
                bytes.clone(),
                Some("unexpected queued input in owned sensitivity transaction".into()),
            );
            let mut error =
                Error::new("unexpected queued sensitivity input; next command not dispatched");
            error.raw_reports.push(bytes);
            return Err(error);
        }
        Ok(())
    }
    fn sensitivity_error(&mut self, mut error: Error) -> Error {
        let original_error = error.to_string();
        // Unknown D8 delivery and later failures before DC dispatch also need
        // bounded owned passive settling. Keep the status active throughout,
        // so late bytes remain traced. A failed DC reader already performed
        // this same hold; retain its evidence without another two-second wait.
        if self
            .sensitivity
            .as_ref()
            .is_some_and(|status| status.attempted)
            && !error.passive_settle_complete
        {
            self.settle_failed_transaction_input(
                &mut error,
                "D8 delivery or effects may have occurred",
                "passive capture after failed D8 transaction",
            );
        }
        let cancelled = self.sensitivity_cancelled();
        let status = self.sensitivity_mut();
        status.failure_phase = Some(status.phase);
        status.original_error = Some(original_error);
        status.cancellation_requested = cancelled;
        status.phase = if cancelled {
            SensitivityPhase::Aborted
        } else {
            SensitivityPhase::Failed
        };
        error.began_write = status.attempted;
        error.backup_path = status.backup.as_ref().map(|p| p.config_path.clone());
        // Preserve all captured input, including earlier complete observations,
        // along with the reader's partial/late evidence and passive capture.
        let mut raw = status
            .trace
            .iter()
            .filter(|t| t.direction == TraceDirection::Input)
            .map(|t| t.bytes.clone())
            .collect::<Vec<_>>();
        for report in std::mem::take(&mut error.raw_reports) {
            if !raw.contains(&report) {
                raw.push(report);
            }
        }
        error.raw_reports = raw;
        if status.attempted {
            error.message.push_str("; D8 physical delivery or effects may have occurred; no acknowledgement, retry or analog rollback; settings/RGB backups cannot restore analog state");
        }
        error.sensitivity = Some(Box::new(status.clone()));
        self.poisoned = true;
        self.last_read = None;
        self.generation += 1;
        error
    }
    /// Set the pinned stock firmware's temporary global coefficient once.
    /// D8 has no acknowledgement. Complete before/after R observations and
    /// settings/RGB verification describe what was observed, not a guarantee of
    /// a changed threshold or of current active scanner compensation. DC1 also
    /// changes scan/output/compensation state and never recalculates R itself.
    /// No calibration, saved threshold, firmware, settings or RGB setter is used.
    pub fn adjust_sensitivity(
        &mut self,
        baseline: &LiveBaseline,
        directory: &Path,
        coefficient: u8,
    ) -> Result<SensitivityResult> {
        let gate = (|| {
            self.check_baseline(baseline)?;
            self.exact_target(baseline.profile())?;
            if self.identity != baseline.profile().identity {
                return Err(Error::new(
                    "sensitivity baseline descriptor identity mismatch",
                ));
            }
            if !(10..=40).contains(&coefficient) {
                return Err(Error::new(
                    "temporary sensitivity coefficient must be 10 through 40",
                ));
            }
            Ok(())
        })();
        if let Err(mut error) = gate {
            let mut status = SensitivityStatus::preparing(coefficient);
            status.failure_phase = Some(status.phase);
            status.phase = SensitivityPhase::Failed;
            status.original_error = Some(error.to_string());
            status.cancellation_requested = self.sensitivity_cancelled();
            error.sensitivity = Some(Box::new(status));
            return Err(error);
        }
        self.sensitivity = Some(SensitivityStatus::preparing(coefficient));
        let transaction = (|| {
            self.check_sensitivity_cancellation()?;
            let current = self.profile_inner()?;
            self.exact_target(&current)?;
            if current.identity != self.identity
                || current.identity != baseline.profile().identity
                || current.firmware != baseline.profile().firmware
                || current.records != baseline.profile().records
            {
                return Err(Error::new(
                    "external settings/firmware changed before sensitivity adjustment",
                ));
            }
            self.check_sensitivity_cancellation()?;
            let lighting = self.lighting_inner()?;
            self.check_sensitivity_cancellation()?;
            let paths = crate::backup::save_snapshot(&current, &lighting, directory, self.backup)?;
            crate::backup::verify_snapshot(&paths, &current, &lighting)?;
            self.sensitivity_mut().backup = Some(paths);
            self.check_sensitivity_cancellation()?;
            self.sensitivity_mut().phase = SensitivityPhase::Before;
            // An in-flight DC reader remains bounded and owned on cancellation.
            let before = self.raw_thresholds_inner()?;
            self.sensitivity_mut().before = Some(before.values().to_vec());
            self.require_quiet_sensitivity_input()?;
            self.check_sensitivity_cancellation()?;
            // Invalidate the live authorization before even attempting D8:
            // short/error native writes can still reach the firmware.
            self.last_read = None;
            self.generation += 1;
            self.sensitivity_mut().phase = SensitivityPhase::Sending;
            self.sensitivity_mut().attempted = true;
            let mut request = niz_core::command(0xd8);
            request[2] = coefficient;
            self.send(&request)?;
            self.sensitivity_mut().sent = true;
            self.sensitivity_mut().phase = SensitivityPhase::After;
            // No D8 ack/read, retry, or recovery command. Even cancellation at
            // D8 completion permits this one bounded owned post-observation.
            let after = self.raw_thresholds_inner()?;
            self.sensitivity_mut().after = Some(after.values().to_vec());
            self.check_sensitivity_cancellation()?;
            self.sensitivity_mut().phase = SensitivityPhase::Verifying;
            let verified = self.profile_inner()?;
            if verified.firmware != current.firmware
                || verified.identity != current.identity
                || verified.records != current.records
            {
                return Err(Error::new("sensitivity settings preservation check failed"));
            }
            self.sensitivity_mut().settings_verified = true;
            self.check_sensitivity_cancellation()?;
            let rgb = self.lighting_inner()?;
            if rgb != lighting {
                return Err(Error::new(
                    "sensitivity lighting preservation check failed; RGB restore unavailable",
                ));
            }
            self.sensitivity_mut().lighting_verified = true;
            self.check_sensitivity_cancellation()?;
            self.sensitivity_mut().phase = SensitivityPhase::Complete;
            Ok(SensitivityResult {
                status: self.sensitivity.as_ref().unwrap().clone(),
            })
        })();
        transaction.map_err(|error| self.sensitivity_error(error))
    }
}
