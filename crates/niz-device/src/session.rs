use crate::transport::{self, Transport};
use crate::{Error, Result, save_backup};
use niz_core::{Definition, DeviceIdentity, MAX_REPORTS, Profile, Report, command};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
pub const EXPERIMENTAL_FIRMWARE: &str = "66EC(XRGB)BLe;V1.0.48;V1.0;";
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);
#[derive(Debug)]
pub struct LiveBaseline {
    profile: Profile,
    session_id: u64,
    generation: u64,
}
impl LiveBaseline {
    pub fn profile(&self) -> &Profile {
        &self.profile
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyOutcome {
    /// A configuration backup was saved; no fresh readback or hardware write occurred.
    NoChange,
    SettingsVerified,
    SettingsAndLightingVerified,
}
#[derive(Debug)]
pub struct ApplyResult {
    pub profile: Profile,
    pub outcome: ApplyOutcome,
    pub backup_path: PathBuf,
    pub snapshot: Option<crate::SnapshotPaths>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    BasicSingle,
    Fn,
    SpecialSingle,
    Empty,
    Combo,
    Repeat,
    Macro,
}
pub(crate) struct Qualification {
    pub(crate) firmware: &'static str,
    pub(crate) product: u16,
    pub(crate) groups: usize,
    pub(crate) actions: &'static [Action],
}
// Only real, independently reviewed hardware evidence may add an entry here.
const QUALIFIED: &[Qualification] = &[];
type BackupWriter = fn(&Profile, &Path) -> Result<PathBuf>;
pub struct Session {
    pub(crate) io: Box<dyn Transport>,
    pub(crate) identity: DeviceIdentity,
    id: u64,
    pub(crate) generation: u64,
    pub(crate) poisoned: bool,
    pub(crate) last_read: Option<Profile>,
    pub(crate) backup: BackupWriter,
    pub(crate) calibration: Option<crate::calibration::CalibrationRun>,
    pub(crate) cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl Session {
    pub fn open(path: &str) -> Result<Self> {
        let (io, identity) = transport::open(path)?;
        Ok(Self::from_transport(io, identity))
    }
    pub(crate) fn from_transport(io: Box<dyn Transport>, identity: DeviceIdentity) -> Self {
        Self {
            io,
            identity,
            id: NEXT_SESSION.fetch_add(1, Ordering::Relaxed),
            generation: 0,
            poisoned: false,
            last_read: None,
            backup: save_backup,
            calibration: None,
            cancel_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    #[cfg(test)]
    pub(crate) fn backup_for_test(&mut self, backup: BackupWriter) {
        self.backup = backup;
    }
    pub(crate) fn ready(&self) -> Result<()> {
        if self
            .calibration
            .as_ref()
            .is_some_and(|run| run.status.phase != crate::CalibrationPhase::Complete)
        {
            return Err(Error::new(
                "session exclusively owned by calibration; ordinary operations unavailable",
            ));
        }
        if self.poisoned {
            Err(Error::new(
                "session invalid after I/O or read failure; reconnect and capture a new baseline",
            ))
        } else {
            Ok(())
        }
    }
    fn poison<T>(&mut self, r: Result<T>) -> Result<T> {
        if r.is_err() {
            self.poisoned = true;
            self.last_read = None;
            self.generation += 1;
        }
        r
    }
    pub(crate) fn send(&mut self, r: &Report) -> Result<()> {
        let mut framed = [0; 65];
        framed[1..].copy_from_slice(r);
        self.trace(crate::TraceDirection::Output, r.to_vec(), None);
        let result = self.io.write(&framed);
        if let Some(run) = self.calibration.as_mut()
            && !matches!(
                run.status.phase,
                crate::CalibrationPhase::Complete
                    | crate::CalibrationPhase::Aborted
                    | crate::CalibrationPhase::Failed
            )
            && let Some(trace) = run.status.trace.last_mut()
        {
            trace.outcome = Some(match &result {
                Ok(n) => format!("output returned {n}/65 bytes"),
                Err(e) => format!("output returned error: {e}; physical delivery unknown"),
            });
        }
        let n = result.map_err(Error::new)?;
        if n != 65 {
            return Err(Error::new(format!(
                "short HID write: {n}/65; physical cancellation is unknown"
            )));
        }
        Ok(())
    }
    fn receive(&mut self, deadline: u64) -> Result<Report> {
        let remain = deadline.saturating_sub(self.io.now_ms());
        if remain == 0 {
            return Err(Error::new("stream exceeded 30-second deadline"));
        }
        let mut r = [0; 65];
        let n = self
            .io
            .read(&mut r, remain.min(1000) as i32)
            .map_err(Error::new)?;
        if n > 0 {
            self.trace(crate::TraceDirection::Input, r[..n].to_vec(), None);
        }
        if self.io.now_ms() > deadline {
            let mut e = Error::new("stream exceeded 30-second deadline");
            if n > 0 {
                e.raw_reports.push(r[..n].to_vec());
            }
            return Err(e);
        }
        if n != 64 {
            let mut e = Error::new(if n == 0 {
                "HID response timeout".into()
            } else {
                format!("short/oversize HID read: {n}/64")
            });
            if n > 0 {
                e.raw_reports.push(r[..n].to_vec());
            }
            return Err(e);
        }
        Ok(r[..64].try_into().expect("64-byte slice"))
    }
    fn prepare_transaction(&mut self) -> Result<()> {
        if self
            .calibration
            .as_ref()
            .is_some_and(|run| run.status.phase != crate::CalibrationPhase::Complete)
        {
            self.require_quiet_calibration_input()
        } else {
            self.io.prepare().map_err(Error::new)
        }
    }
    fn version_inner(&mut self) -> Result<String> {
        self.version_capture_inner().map(|(version, _)| version)
    }
    fn version_capture_inner(&mut self) -> Result<(String, Report)> {
        self.prepare_transaction()?;
        self.send(&command(0xf9))?;
        let r = self.receive(self.io.now_ms() + 1000)?;
        let parse = || -> Result<String> {
            if r[..2] != [0, 0xf9] {
                return Err(Error::new("incorrect F9 version response"));
            }
            let end = r[2..]
                .iter()
                .position(|b| *b == 0)
                .map(|n| n + 2)
                .unwrap_or(64);
            let s =
                std::str::from_utf8(&r[2..end]).map_err(|_| Error::new("version is not UTF-8"))?;
            if s.is_empty() || !s.is_ascii() || s.bytes().any(|b| b.is_ascii_control()) {
                return Err(Error::new("invalid empty/control firmware version"));
            }
            Ok(s.into())
        };
        parse().map(|version| (version, r)).map_err(|mut e| {
            e.raw_reports.push(r.to_vec());
            e
        })
    }
    pub fn version(&mut self) -> Result<String> {
        self.ready()?;
        let r = self.version_inner();
        self.poison(r)
    }
    /// Manually read the exact pinned firmware's RAM R table with one fresh F9
    /// request and one DC mode-1 request. The getter pauses/resets scanner output
    /// and restores active compensation from its configured/default byte, not
    /// from the prior active compensation. It does not change B/T/R tables, the
    /// global factor, or flash. Any failure after a possible DC dispatch holds the
    /// owned transport/lock through bounded passive capture/settling; this cannot
    /// confirm scanner restoration. No retry or firmware recovery command is sent.
    pub fn read_raw_thresholds(&mut self) -> Result<crate::RawThresholds> {
        self.ready()?;
        if !crate::supports_raw_thresholds(&self.identity, EXPERIMENTAL_FIRMWARE) {
            return Err(Error::new(
                "raw thresholds require exact 0483:502a 008c:1 identity",
            ));
        }
        let result = self.raw_thresholds_inner();
        self.poison(result)
    }
    fn raw_thresholds_inner(&mut self) -> Result<crate::RawThresholds> {
        let deadline = self.io.now_ms().saturating_add(30_000);
        let (firmware, version_report) = self.version_capture_inner()?;
        let mut evidence = vec![version_report.to_vec()];
        let mut dc_attempted = false;
        let transaction = (|| {
            if !crate::supports_raw_thresholds(&self.identity, &firmware) {
                return Err(Error::new(
                    "fresh F9 firmware is not qualified for raw thresholds",
                ));
            }
            if self.last_read.as_ref().is_some_and(|profile| {
                profile.identity != self.identity || profile.firmware != firmware
            }) {
                return Err(Error::new(
                    "fresh F9 identity/firmware differs from the retained baseline",
                ));
            }
            // After a fresh version response, excess input is a failed boundary,
            // not a stale queue that may be silently drained before dispatch.
            if let Some(extra) = self.io.queued_input().map_err(Error::new)? {
                evidence.push(extra);
                return Err(Error::new(
                    "unexpected queued input after fresh F9; DC not sent",
                ));
            }
            if self.io.now_ms() >= deadline {
                return Err(Error::new("stream exceeded 30-second deadline"));
            }
            let mut request = command(0xdc);
            request[2] = 1;
            // Even an error or short write can have delivered the read request.
            dc_attempted = true;
            self.send(&request)?;
            let mut reports = Vec::with_capacity(11);
            let mut decoder = niz_core::RawThresholdDecoder::default();
            for _ in 0..11 {
                let report = self.receive(deadline)?;
                evidence.push(report.to_vec());
                decoder.push(&report)?;
                reports.push(report);
            }
            // Mode 1 has no terminator or reference frame. A nonblocking probe
            // prevents accepting queued excess data as a future transaction.
            if let Some(extra) = self.io.queued_input().map_err(Error::new)? {
                evidence.push(extra);
                return Err(Error::new(
                    "unexpected queued input after raw threshold rows",
                ));
            }
            if self.io.now_ms() > deadline {
                return Err(Error::new("stream exceeded 30-second deadline"));
            }
            decoder.finish()?;
            crate::RawThresholds::from_reports(self.identity.clone(), firmware, reports)
        })();
        transaction.map_err(|mut error: Error| {
            evidence.append(&mut error.raw_reports);
            error.raw_reports = evidence;
            if dc_attempted {
                self.settle_failed_raw_threshold_read(&mut error);
            }
            error
        })
    }
    fn settle_failed_raw_threshold_read(&mut self, error: &mut Error) {
        const BUDGET_MS: u64 = 2000;
        const STEP_MS: u64 = 50;
        const MAX_ATTEMPTS: usize = 40;
        const MAX_CAPTURED_REPORTS: usize = 16;
        let started = self.io.now_ms();
        let mut captured = 0;
        let mut attempts = 0;
        let mut requested_wait = 0;
        let mut probe_error = None;
        // Keep self.io and its native per-path lock owned during this phase.
        // No output is permitted. Attempt/report caps also bound a broken clock.
        for _ in 0..MAX_ATTEMPTS {
            let elapsed = self.io.now_ms().saturating_sub(started);
            if elapsed >= BUDGET_MS {
                break;
            }
            if probe_error.is_none() && captured < MAX_CAPTURED_REPORTS {
                attempts += 1;
                match self.io.queued_input() {
                    Ok(Some(bytes)) => {
                        error.raw_reports.push(bytes);
                        captured += 1;
                    }
                    Ok(None) => {}
                    Err(message) => probe_error = Some(message),
                }
            }
            let remaining = BUDGET_MS.saturating_sub(self.io.now_ms().saturating_sub(started));
            if remaining == 0 {
                break;
            }
            let wait = STEP_MS.min(remaining);
            self.io.sleep_ms(wait);
            requested_wait += wait;
        }
        let elapsed = self.io.now_ms().saturating_sub(started);
        error.message.push_str(&format!(
            "; DC delivery may have occurred; passive settling held the owned session/lock: {elapsed} ms elapsed, {requested_wait} ms requested waits, {attempts} probes, {captured} captured reports; scanner restoration not confirmed",
        ));
        if captured == MAX_CAPTURED_REPORTS {
            error.message.push_str("; passive report limit reached");
        }
        if let Some(message) = probe_error {
            error
                .message
                .push_str(&format!("; passive capture stopped: {message}"));
        }
    }
    fn recognized(version: &str) -> bool {
        let v = version.to_ascii_uppercase();
        ["66EC(S)", "66EC(XRGB)", "66EC(RGB)"]
            .iter()
            .any(|prefix| v.starts_with(prefix))
    }
    pub(crate) fn profile_inner(&mut self) -> Result<Profile> {
        let version = self.version_inner()?;
        if !Self::recognized(&version) {
            return Err(Error::new(format!(
                "unsupported firmware family: {version}"
            )));
        }
        self.key_reports(version).map(|(profile, _)| profile)
    }
    fn key_reports(&mut self, version: String) -> Result<(Profile, Vec<Report>)> {
        self.prepare_transaction()?;
        self.send(&command(0xf2))?;
        let deadline = self.io.now_ms() + 30000;
        let mut raw = vec![];
        loop {
            let r = match self.receive(deadline) {
                Ok(r) => r,
                Err(mut e) => {
                    let partial = std::mem::take(&mut e.raw_reports);
                    e.raw_reports = raw.iter().map(|r: &Report| r.to_vec()).collect();
                    e.raw_reports.extend(partial);
                    return Err(e);
                }
            };
            // Device configuration reads terminate with the native F6 marker in byte 0.
            if r[0] == 0xf6 {
                let profile = Profile::from_reports(version, self.identity.clone(), raw.clone())
                    .map_err(|e| {
                        let mut err = Error::from(e);
                        err.raw_reports = raw.iter().map(|r| r.to_vec()).collect();
                        err
                    })?;
                return Ok((profile, raw));
            }
            raw.push(r);
            if r[..2] != [0, 0xf0] || raw.len() > MAX_REPORTS {
                let mut e = Error::new("unknown packet or configuration report limit exceeded");
                e.raw_reports = raw.iter().map(|r| r.to_vec()).collect();
                return Err(e);
            }
        }
    }
    pub fn read_profile(&mut self) -> Result<Profile> {
        self.ready()?;
        self.last_read = None;
        self.generation += 1;
        let r = self.profile_inner();
        let p = self.poison(r)?;
        self.last_read = Some(p.clone());
        Ok(p)
    }
    pub fn read_baseline(&mut self) -> Result<LiveBaseline> {
        let profile = self.read_profile()?;
        Ok(LiveBaseline {
            profile,
            session_id: self.id,
            generation: self.generation,
        })
    }
    pub fn read_counters(&mut self) -> Result<Vec<u32>> {
        self.ready()?;
        let r = self.counters_inner();
        self.poison(r)
    }
    fn counters_inner(&mut self) -> Result<Vec<u32>> {
        let v = self.version_inner()?;
        if !v.to_ascii_uppercase().starts_with("66EC(S)BLE;") {
            return Err(Error::new(
                "counters require identified 66EC(S)BLe firmware",
            ));
        }
        self.prepare_transaction()?;
        self.send(&command(0xe3))?;
        let deadline = self.io.now_ms() + 30000;
        let mut data = vec![];
        let mut raw = vec![];
        let parse = (|| {
            for _ in 0..64 {
                let r = self.receive(deadline)?;
                raw.push(r.to_vec());
                if is_byte_stream_terminator(&r) {
                    if data.len() != 264 {
                        return Err(Error::new("counter byte count must be 264"));
                    }
                    return Ok(data
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|c| u32::from_le_bytes(*c))
                        .collect());
                }
                let n = r[2] as usize;
                if r[..2] != [0, 0xe3] || !(1..=61).contains(&n) || data.len() + n > 264 {
                    return Err(Error::new("invalid counter chunk/overflow"));
                }
                data.extend_from_slice(&r[3..3 + n]);
            }
            Err(Error::new("missing counter terminator"))
        })();
        parse.map_err(|mut e| {
            raw.append(&mut e.raw_reports);
            e.raw_reports = raw;
            e
        })
    }
    pub(crate) fn check_baseline(&self, b: &LiveBaseline) -> Result<()> {
        self.ready()?;
        if b.session_id != self.id
            || b.generation != self.generation
            || self.last_read.as_ref().is_none_or(|p| {
                p.firmware != b.profile.firmware
                    || p.identity != b.profile.identity
                    || p.records != b.profile.records
            })
        {
            return Err(Error::new(
                "foreign/stale baseline; capture current device in this session",
            ));
        }
        Ok(())
    }
    pub fn apply(&mut self, b: &LiveBaseline, d: &Profile, dir: &Path) -> Result<ApplyResult> {
        self.apply_impl(b, d, dir, QUALIFIED, false)
    }
    pub(crate) fn exact_target(&self, p: &Profile) -> Result<()> {
        if p.firmware != EXPERIMENTAL_FIRMWARE
            || p.identity
                != (DeviceIdentity {
                    vendor_id: 0x0483,
                    product_id: 0x502a,
                    usage_page: 0x8c,
                    usage: 1,
                })
            || p.group_count() != 3
        {
            return Err(Error::new(
                "experimental target requires exact 66EC(XRGB)BLe;V1.0.48;V1.0; / 0483:502a usage008c/1, three groups",
            ));
        }
        Ok(())
    }
    pub fn trial_swap(
        &mut self,
        b: &LiveBaseline,
        from: usize,
        to: usize,
        dir: &Path,
    ) -> Result<ApplyResult> {
        self.check_baseline(b)?;
        self.exact_target(&b.profile)?;
        if from >= 198 || to >= 198 || from / 66 != to / 66 {
            return Err(Error::new(
                "trial swap requires two editable indices on the same layer",
            ));
        }
        for index in [from, to] {
            let d = b.profile.definition(index)?;
            if d.kind != 0 || d.keys.len() != 1 || !(1..=74).contains(&d.keys[0]) {
                return Err(Error::new("trial swap requires two basic single keys"));
            }
            for layer in 0..3 {
                if b.profile
                    .definition(layer * 66 + index % 66)?
                    .keys
                    .iter()
                    .any(|k| [156, 166].contains(k))
                {
                    return Err(Error::new(
                        "trial swap excludes every physical Fn-bearing column",
                    ));
                }
            }
        }
        let mut desired = b.profile.clone();
        desired.swap_keys(from, to)?;
        self.apply_impl(b, &desired, dir, &[], true)
    }
    pub fn read_lighting(&mut self) -> Result<Vec<u8>> {
        self.ready()?;
        let result = (|| {
            let version = self.version_inner()?;
            if version != EXPERIMENTAL_FIRMWARE
                || self.identity
                    != (DeviceIdentity {
                        vendor_id: 0x0483,
                        product_id: 0x502a,
                        usage_page: 0x8c,
                        usage: 1,
                    })
            {
                return Err(Error::new(
                    "lighting read requires exact experimental XRGB target",
                ));
            }
            self.lighting_inner()
        })();
        self.poison(result)
    }
    pub(crate) fn lighting_inner(&mut self) -> Result<Vec<u8>> {
        self.lighting_capture_inner().map(|(bytes, _)| bytes)
    }
    pub(crate) fn lighting_capture_inner(&mut self) -> Result<(Vec<u8>, Vec<Vec<u8>>)> {
        self.prepare_transaction()?;
        self.send(&command(0xe2))?;
        let deadline = self.io.now_ms() + 30000;
        let mut bytes = vec![];
        let mut raw = vec![];
        let result = (|| {
            for _ in 0..64 {
                let r = self.receive(deadline)?;
                raw.push(r.to_vec());
                if is_byte_stream_terminator(&r) {
                    if bytes.len() != 198 {
                        return Err(Error::new("lighting byte count must be exactly 198"));
                    }
                    return Ok((bytes, raw.clone()));
                }
                let n = r[2] as usize;
                if r[..2] != [0, 0xe0] || !(1..=61).contains(&n) || bytes.len() + n > 198 {
                    return Err(Error::new("invalid lighting chunk/overflow"));
                }
                bytes.extend_from_slice(&r[3..3 + n]);
            }
            Err(Error::new("missing lighting terminator"))
        })();
        result.map_err(|mut e| {
            raw.append(&mut e.raw_reports);
            e.raw_reports = raw;
            e
        })
    }
    pub fn trial_single_key(
        &mut self,
        b: &LiveBaseline,
        index: usize,
        code: u8,
        dir: &Path,
    ) -> Result<ApplyResult> {
        self.check_baseline(b)?;
        self.exact_target(&b.profile)?;
        if index >= 198 || !(1..=74).contains(&code) {
            return Err(Error::new(
                "trial requires one editable key and basic code 1..74",
            ));
        }
        let old = b.profile.definition(index)?;
        if old.kind != 0 || old.keys.len() != 1 || !(1..=74).contains(&old.keys[0]) {
            return Err(Error::new("trial source must be one basic non-Fn key"));
        }
        for layer in 0..3 {
            let d = b.profile.definition(layer * 66 + index % 66)?;
            if d.keys.iter().any(|k| [156, 166].contains(k)) {
                return Err(Error::new("trial cannot affect a physical Fn location"));
            }
        }
        let mut desired = b.profile.clone();
        let d = Definition {
            kind: 0,
            keys: vec![code],
            ..old
        };
        desired.set_definition(index, &d)?;
        if desired.differences(&b.profile)?.iter().any(|i| *i != index) {
            return Err(Error::new("trial changed more than its single key"));
        }
        self.apply_impl(b, &desired, dir, &[], true)
    }
    /// Read-only local gate. It performs no HID traffic and does not authorize a later write.
    pub fn check_apply(&self, b: &LiveBaseline, d: &Profile) -> Result<()> {
        self.validate_apply(b, d, QUALIFIED, false).map(|_| ())
    }
    fn validate_apply(
        &self,
        b: &LiveBaseline,
        d: &Profile,
        qualifications: &[Qualification],
        trial: bool,
    ) -> Result<Vec<usize>> {
        self.check_baseline(b)?;
        if b.profile.identity != self.identity || !transport::supported(&self.identity) {
            return Err(Error::new("baseline descriptor identity mismatch"));
        }
        d.validate_changes_against(&b.profile)?;
        let changed = b.profile.differences(d)?;
        if !trial {
            let mut semantic = b.profile.clone();
            for index in &changed {
                semantic.set_definition(*index, &d.definition(*index)?)?;
            }
            let raw_swap = if changed.len() == 2 {
                let mut swap = b.profile.clone();
                swap.swap_keys(changed[0], changed[1]).is_ok() && swap.records == d.records
            } else {
                false
            };
            if semantic.records != d.records && !raw_swap {
                return Err(Error::new("unqualified raw padding/reserved-byte change"));
            }
            for index in &changed {
                let old = action(&b.profile.definition(*index)?);
                let new = action(&d.definition(*index)?);
                if !qualifications.iter().any(|q| {
                    q.firmware == d.firmware
                        && q.product == d.identity.product_id
                        && q.groups == d.group_count()
                        && q.actions.contains(&old)
                        && q.actions.contains(&new)
                }) {
                    return Err(Error::new(format!(
                        "unqualified action at record {index}; ordinary hardware writes remain unqualified"
                    )));
                }
            }
        }
        Ok(changed)
    }
    pub(crate) fn apply_impl(
        &mut self,
        b: &LiveBaseline,
        d: &Profile,
        dir: &Path,
        qualifications: &[Qualification],
        trial: bool,
    ) -> Result<ApplyResult> {
        let changed = self.validate_apply(b, d, qualifications, trial)?;
        // No-op produces a durable snapshot, but never starts a physical write.
        if changed.is_empty() {
            let path = (self.backup)(&b.profile, dir)?;
            return Ok(ApplyResult {
                profile: b.profile.clone(),
                outcome: ApplyOutcome::NoChange,
                backup_path: path,
                snapshot: None,
            });
        }
        let current = match self.profile_inner() {
            Ok(p) => p,
            Err(e) => return self.poison(Err(e)),
        };
        if current.firmware != b.profile.firmware
            || current.identity != b.profile.identity
            || current.records != b.profile.records
        {
            self.last_read = None;
            self.generation += 1;
            return Err(Error::new(
                "external configuration/firmware change; capture a fresh baseline",
            ));
        }
        let lighting = if trial || current.firmware == EXPERIMENTAL_FIRMWARE {
            let result = self.lighting_inner();
            Some(self.poison(result)?)
        } else {
            None
        };
        let snapshot = if let Some(rgb) = &lighting {
            Some(crate::backup::save_snapshot(
                &current,
                rgb,
                dir,
                self.backup,
            )?)
        } else {
            None
        };
        let path = if let Some(snapshot) = &snapshot {
            snapshot.config_path.clone()
        } else {
            (self.backup)(&current, dir)?
        };
        // Independent reparse is a final gate even for an injected backup writer.
        let backed = crate::load_profile(&path)?;
        if backed.firmware != current.firmware
            || backed.identity != current.identity
            || backed.records != current.records
        {
            return Err(Error::new("durable backup reparse mismatch"));
        }
        if let (Some(paths), Some(rgb)) = (&snapshot, &lighting) {
            crate::backup::verify_snapshot(paths, &current, rgb)?;
        }
        self.last_read = None;
        self.generation += 1;
        let transaction = (|| {
            self.send(&command(0xf1))?;
            for r in d.reports() {
                self.send(&r)?;
                self.io.sleep_ms(3);
            }
            let mut end = [0xf6; 64];
            end[0] = 0;
            self.send(&end)?;
            self.io.sleep_ms(300);
            let firmware = self.version_inner()?;
            let (verified, readback_raw) = self.key_reports(firmware)?;
            if verified.firmware != d.firmware
                || verified.identity != d.identity
                || verified.records != d.records
            {
                let mut error =
                    Error::new("readback byte mismatch, including opaque/unmodified bytes");
                error.raw_reports = readback_raw.iter().map(|r| r.to_vec()).collect();
                return Err(error);
            }
            if let Some(before) = &lighting {
                let (after, raw) = self.lighting_capture_inner()?;
                if after != *before {
                    let mut e =
                        Error::new("lighting readback mismatch; RGB restore is not implemented");
                    e.raw_reports = raw;
                    return Err(e);
                }
            }
            Ok(verified)
        })();
        match transaction {
            Ok(profile) => {
                self.last_read = Some(profile.clone());
                Ok(ApplyResult {
                    profile,
                    outcome: if lighting.is_some() {
                        ApplyOutcome::SettingsAndLightingVerified
                    } else {
                        ApplyOutcome::SettingsVerified
                    },
                    backup_path: path,
                    snapshot,
                })
            }
            Err(mut e) => {
                e.began_write = true;
                e.backup_path = Some(path);
                e.message.push_str("; write may have partially reached hardware; no automatic retry or rollback; reconnect and read before restoring");
                self.poison(Err(e))
            }
        }
    }
}
fn action(d: &Definition) -> Action {
    match d.kind {
        0 if d.keys.is_empty() => Action::Empty,
        0 if d.keys.len() == 1 && (1..=74).contains(&d.keys[0]) => Action::BasicSingle,
        0 if d.keys.len() == 1 && [156, 166].contains(&d.keys[0]) => Action::Fn,
        0 if d.keys.len() == 1 => Action::SpecialSingle,
        0 => Action::Combo,
        1 => Action::Repeat,
        _ => Action::Macro,
    }
}

// Read-only streams have two observed framing forms: the prefixed E6 marker
// used by existing fixtures and the native 64-byte repeated E6 marker.
fn is_byte_stream_terminator(report: &Report) -> bool {
    report == &[0xe6; 64] || report[..2] == [0, 0xe6]
}
