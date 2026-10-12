//! Native-boundary scripts catch extra commands, D8 acknowledgement reads,
//! queue drains, premature mutation, incomplete observations and lost evidence.
use crate::{SensitivityPhase, Session, transport::Transport};
use niz_core::{Profile, Report};
use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

const PHYSICAL: [u8; 66] = [
    0, 1, 2, 3, 4, 5, 29, 28, 27, 26, 25, 24, 60, 61, 62, 6, 7, 8, 9, 10, 11, 35, 34, 33, 32, 31,
    30, 63, 65, 12, 13, 14, 15, 16, 17, 41, 40, 39, 38, 37, 36, 64, 18, 19, 20, 21, 22, 23, 47, 46,
    45, 44, 43, 42, 58, 48, 49, 50, 51, 52, 53, 57, 56, 55, 54, 59,
];
fn report(op: u8, byte: u8) -> Report {
    let mut r = [0; 64];
    r[1] = op;
    r[2] = byte;
    r
}
fn profile() -> Profile {
    Profile::from_json(include_str!(
        "../../../docs/validation/fixtures/atom66-xrgb-ble-v1.0.48-settings.json"
    ))
    .unwrap()
}
struct Step {
    request: Report,
    replies: Vec<(u64, Vec<u8>)>,
    result: std::result::Result<usize, String>,
}
fn step(op: u8, byte: u8, replies: Vec<Vec<u8>>) -> Step {
    Step {
        request: report(op, byte),
        replies: replies.into_iter().map(|b| (0, b)).collect(),
        result: Ok(65),
    }
}
fn version(p: &Profile) -> Step {
    let mut r = report(0xf9, 0);
    r[2..2 + p.firmware.len()].copy_from_slice(p.firmware.as_bytes());
    step(0xf9, 0, vec![r.to_vec()])
}
fn config(p: &Profile) -> Vec<Step> {
    let mut rows = p.reports().iter().map(|r| r.to_vec()).collect::<Vec<_>>();
    rows.push(vec![0xf6; 64]);
    vec![version(p), step(0xf2, 0, rows)]
}
fn rgb(changed: bool) -> Step {
    let bytes = [if changed { 0x38 } else { 0x37 }; 198];
    let mut rows = vec![];
    for part in bytes.chunks(61) {
        let mut r = report(0xe0, part.len() as u8);
        r[3..3 + part.len()].copy_from_slice(part);
        rows.push(r.to_vec());
    }
    rows.push(report(0xe6, 0).to_vec());
    step(0xe2, 0, rows)
}
fn dc(change: bool) -> Step {
    let rows = (1..=11u8)
        .map(|row| {
            let mut r = report(0xdc, row);
            r[3] = 1;
            for col in 0..6 {
                r[4 + col] = (row - 1) * 6 + col as u8;
            }
            if change && row == 3 {
                r[5] = 201;
            }
            r[10..].fill(0xa0 + row);
            r.to_vec()
        })
        .collect();
    step(0xdc, 1, rows)
}
fn script(c: u8, changed: bool) -> Vec<Step> {
    let p = profile();
    let mut steps = config(&p); // Initial live baseline.
    steps.extend(config(&p));
    steps.push(rgb(false));
    steps.push(version(&p));
    steps.push(dc(false));
    steps.push(step(0xd8, c, vec![])); // Native firmware has no D8 acknowledgement.
    steps.push(version(&p));
    steps.push(dc(changed));
    steps.extend(config(&p));
    steps.push(rgb(false));
    steps
}
#[derive(Default)]
struct State {
    queued: VecDeque<(u64, Vec<u8>)>,
    sent: Vec<Report>,
    reads: Vec<u8>,
    now: u64,
    prepare: usize,
    current: u8,
    dc_count: usize,
    row: usize,
    cancel_dc: Option<usize>,
    cancel_output: Option<u8>,
    cancel: Option<Arc<AtomicBool>>,
    sleeps: u64,
    late: Option<(u64, Vec<u8>)>,
    unexpected: Vec<String>,
}
struct Script {
    state: Rc<RefCell<State>>,
    steps: VecDeque<Step>,
}
impl Transport for Script {
    fn prepare(&mut self) -> std::result::Result<(), String> {
        let mut s = self.state.borrow_mut();
        s.prepare += 1;
        if s.prepare > 2 {
            return Err("global transaction attempted native queue drain".into());
        }
        if !s.queued.is_empty() {
            return Err("baseline attempted to drain evidence".into());
        }
        Ok(())
    }
    fn queued_input(&mut self) -> std::result::Result<Option<Vec<u8>>, String> {
        Ok(self.state.borrow_mut().queued.pop_front().map(|(_, b)| b))
    }
    fn write(&mut self, b: &[u8]) -> std::result::Result<usize, String> {
        assert_eq!(b.len(), 65);
        assert_eq!(b[0], 0);
        let r: Report = b[1..].try_into().unwrap();
        let mut s = self.state.borrow_mut();
        s.sent.push(r);
        let Some(expected) = self.steps.pop_front() else {
            s.unexpected.push("extra output".into());
            return Err("extra output".into());
        };
        if r != expected.request {
            let e = format!(
                "expected {:02x}/{} got {:02x}/{}",
                expected.request[1], expected.request[2], r[1], r[2]
            );
            s.unexpected.push(e.clone());
            return Err(e);
        }
        assert!(
            s.queued.is_empty(),
            "new command before previous evidence consumed"
        );
        s.current = r[1];
        s.row = 0;
        if r[1] == 0xdc {
            s.dc_count += 1;
        }
        s.queued.extend(expected.replies);
        if s.cancel_output == Some(r[1]) {
            s.cancel.as_ref().unwrap().store(true, Ordering::Release);
        }
        expected.result
    }
    fn read(&mut self, b: &mut [u8], timeout: i32) -> std::result::Result<usize, String> {
        let mut s = self.state.borrow_mut();
        let op = s.current;
        s.reads.push(op);
        s.row += 1;
        if op == 0xdc && s.row == 3 && s.cancel_dc == Some(s.dc_count) {
            s.cancel.as_ref().unwrap().store(true, Ordering::Release);
        }
        if let Some((delay, r)) = s.queued.pop_front() {
            s.now += delay;
            b[..r.len()].copy_from_slice(&r);
            Ok(r.len())
        } else {
            s.now += timeout as u64;
            Ok(0)
        }
    }
    fn now_ms(&self) -> u64 {
        self.state.borrow().now
    }
    fn sleep_ms(&mut self, n: u64) {
        let mut s = self.state.borrow_mut();
        s.now += n;
        s.sleeps += n;
        if s.late.as_ref().is_some_and(|(at, _)| s.now >= *at) {
            let (_, bytes) = s.late.take().unwrap();
            s.queued.push_back((0, bytes));
        }
    }
}
fn setup(steps: Vec<Step>) -> (Session, Rc<RefCell<State>>) {
    let state = Rc::new(RefCell::new(State::default()));
    let session = Session::from_transport(
        Box::new(Script {
            state: state.clone(),
            steps: steps.into(),
        }),
        profile().identity,
    );
    state.borrow_mut().cancel = Some(session.cancellation_flag());
    (session, state)
}
fn allowed(s: &State) {
    assert!(s.unexpected.is_empty(), "{:?}", s.unexpected);
    assert!(
        s.sent
            .iter()
            .all(|r| matches!(r[1], 0xf9 | 0xf2 | 0xe2 | 0xdc | 0xd8))
    );
    assert_eq!(s.prepare, 2);
    assert!(!s.reads.contains(&0xd8));
}
#[test]
fn exact_no_ack_d8_once_preserves_settings_and_complete_physical_observations() {
    for c in [10, 24, 40] {
        let (mut session, state) = setup(script(c, true));
        let b = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let result = session.adjust_sensitivity(&b, dir.path(), c).unwrap();
        let status = result.status;
        assert_eq!(status.phase, SensitivityPhase::Complete);
        assert_eq!(status.coefficient, c);
        assert_eq!(status.before, Some(PHYSICAL.to_vec()));
        let mut post = PHYSICAL.to_vec();
        post[30] = 201;
        assert_eq!(status.after, Some(post));
        assert!(
            status.attempted && status.sent && status.settings_verified && status.lighting_verified
        );
        let paths = status.backup.unwrap();
        assert!(
            paths.config_path.exists()
                && paths.lighting_path.exists()
                && paths.manifest_path.exists()
        );
        assert_eq!(std::fs::read(paths.lighting_path).unwrap(), vec![0x37; 198]);
        let s = state.borrow();
        allowed(&s);
        assert_eq!(s.sent.iter().filter(|r| r[1] == 0xd8).count(), 1);
        assert_eq!(s.reads.iter().filter(|op| **op == 0xdc).count(), 22);
        drop(s);
        let sent = state.borrow().sent.clone();
        assert!(session.adjust_sensitivity(&b, dir.path(), c).is_err());
        assert_eq!(state.borrow().sent, sent);
    }
}
#[test]
fn successful_same_threshold_values_do_not_claim_a_change() {
    let (mut session, state) = setup(script(24, false));
    let b = session.read_baseline().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let status = session
        .adjust_sensitivity(&b, dir.path(), 24)
        .unwrap()
        .status;
    assert_eq!(status.before, status.after);
    assert_eq!(status.after, Some(PHYSICAL.to_vec()));
    allowed(&state.borrow());
}
#[test]
fn invalid_range_foreign_stale_and_wrong_identity_reject_without_traffic() {
    for c in [0, 9, 41, 255] {
        let (mut session, state) = setup(config(&profile()));
        let b = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let sent = state.borrow().sent.clone();
        assert!(session.adjust_sensitivity(&b, dir.path(), c).is_err());
        assert_eq!(state.borrow().sent, sent);
    }
    for foreign in [true, false] {
        let (mut session, state) = setup(config(&profile()));
        let b = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let sent = state.borrow().sent.clone();
        if foreign {
            let (mut other, _) = setup(vec![]);
            assert!(other.adjust_sensitivity(&b, dir.path(), 24).is_err());
        } else {
            session.identity.product_id = 0x512a;
            assert!(session.adjust_sensitivity(&b, dir.path(), 24).is_err());
        }
        assert_eq!(state.borrow().sent, sent);
    }
}
#[test]
fn changed_current_config_or_firmware_stops_before_rgb_and_d8() {
    for firmware in [false, true] {
        let mut p = profile();
        if firmware {
            p.firmware = "66EC(XRGB)BLe;V1.0.49;V1.0;".into();
        } else {
            p.set_definition(0, &p.definition(1).unwrap()).unwrap();
        }
        let mut steps = config(&profile());
        steps.extend(config(&p));
        let (mut session, state) = setup(steps);
        let b = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let error = session.adjust_sensitivity(&b, dir.path(), 24).unwrap_err();
        let status = error.sensitivity.unwrap();
        assert!(!status.attempted && !status.sent);
        assert!(status.before.is_none());
        assert!(
            state
                .borrow()
                .sent
                .iter()
                .all(|r| matches!(r[1], 0xf9 | 0xf2))
        );
        assert!(state.borrow().unexpected.is_empty());
    }
}
#[test]
fn durable_backup_failure_and_unexpected_queue_prevent_mutation() {
    for queue in [false, true] {
        let mut steps = script(24, false);
        steps.truncate(if queue { 7 } else { 5 });
        if queue {
            steps[6].replies.push((0, report(0xdc, 12).to_vec()));
        }
        let (mut session, state) = setup(steps);
        let b = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("file");
        std::fs::write(&bad, b"not a directory").unwrap();
        let error = session
            .adjust_sensitivity(&b, if queue { dir.path() } else { &bad }, 24)
            .unwrap_err();
        let status = error.sensitivity.unwrap();
        assert!(!status.attempted);
        assert!(status.before.is_none());
        assert!(!state.borrow().sent.iter().any(|r| r[1] == 0xd8));
        if queue {
            assert!(!error.raw_reports.is_empty());
            assert_eq!(state.borrow().sleeps, 2000);
        }
        allowed(&state.borrow());
    }
}
#[test]
fn uncertain_d8_delivery_retains_before_snapshot_and_never_retries_or_rolls_back() {
    for short in [false, true] {
        let mut steps = script(24, false);
        steps.truncate(8);
        steps[7].result = if short {
            Ok(64)
        } else {
            Err("physical delivery unknown".into())
        };
        let (mut session, state) = setup(steps);
        let b = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let error = session.adjust_sensitivity(&b, dir.path(), 24).unwrap_err();
        let status = error.sensitivity.unwrap();
        assert!(status.attempted && !status.sent);
        assert!(error.began_write);
        assert_eq!(status.before, Some(PHYSICAL.to_vec()));
        assert!(status.after.is_none() && status.backup.is_some());
        assert!(status.trace.iter().any(|t| {
            t.bytes.get(1) == Some(&0xd8)
                && t.outcome
                    .as_ref()
                    .is_some_and(|s| s.contains("unknown") || s.contains("64/65"))
        }));
        let sent = state.borrow().sent.clone();
        assert!(session.version().is_err());
        assert!(session.adjust_sensitivity(&b, dir.path(), 24).is_err());
        assert_eq!(state.borrow().sent, sent);
        allowed(&state.borrow());
    }
}
#[test]
fn malformed_partial_late_extra_and_short_post_dc_keep_raw_evidence_without_after() {
    for fault in 0..5 {
        let mut steps = script(24, false);
        steps.truncate(10);
        match fault {
            0 => steps[9].replies.truncate(4),
            1 => steps[9].replies[3].1[3] = 2,
            2 => steps[9].replies[10].0 = 30001,
            3 => steps[9].replies.push((0, report(0xdc, 12).to_vec())),
            _ => steps[9].result = Ok(64),
        }
        let (mut session, state) = setup(steps);
        let b = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let error = session.adjust_sensitivity(&b, dir.path(), 24).unwrap_err();
        let status = error.sensitivity.unwrap();
        assert_eq!(status.before, Some(PHYSICAL.to_vec()));
        assert!(status.after.is_none());
        assert!(status.attempted && status.sent);
        assert!(!error.raw_reports.is_empty());
        assert!(error.message.contains("passive settling"));
        assert_eq!(state.borrow().sleeps, 2000);
        allowed(&state.borrow());
    }
}
#[test]
fn complete_after_survives_later_settings_or_rgb_failure() {
    for lighting in [false, true] {
        let mut steps = script(24, true);
        if lighting {
            steps[12] = rgb(true);
        } else {
            let mut p = profile();
            p.set_definition(0, &p.definition(1).unwrap()).unwrap();
            let mut tail = config(&p);
            tail.push(rgb(false));
            steps.splice(10.., tail);
        }
        let (mut session, state) = setup(steps);
        let b = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let error = session.adjust_sensitivity(&b, dir.path(), 24).unwrap_err();
        let status = error.sensitivity.unwrap();
        let mut post = PHYSICAL.to_vec();
        post[30] = 201;
        assert_eq!(status.before, Some(PHYSICAL.to_vec()));
        assert_eq!(status.after, Some(post));
        assert!(!status.lighting_verified);
        assert_eq!(status.settings_verified, lighting);
        allowed(&state.borrow());
    }
}
#[test]
fn cancellation_during_each_dc_completes_owned_reader_then_stops_without_rollback() {
    for dc_number in [1, 2] {
        let mut steps = script(24, false);
        steps.truncate(if dc_number == 1 { 7 } else { 10 });
        let (mut session, state) = setup(steps);
        let b = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        state.borrow_mut().cancel_dc = Some(dc_number);
        let error = session.adjust_sensitivity(&b, dir.path(), 24).unwrap_err();
        let status = error.sensitivity.unwrap();
        assert_eq!(status.phase, SensitivityPhase::Aborted);
        assert!(status.cancellation_requested);
        assert_eq!(status.before, Some(PHYSICAL.to_vec()));
        assert_eq!(status.after, (dc_number == 2).then(|| PHYSICAL.to_vec()));
        assert_eq!(status.attempted, dc_number == 2);
        assert_eq!(
            state
                .borrow()
                .reads
                .iter()
                .filter(|op| **op == 0xdc)
                .count(),
            11 * dc_number
        );
        allowed(&state.borrow());
    }
}
#[test]
fn cancellation_immediately_after_d8_still_collects_bounded_post_dc() {
    let mut steps = script(24, false);
    steps.truncate(10);
    let (mut session, state) = setup(steps);
    let b = session.read_baseline().unwrap();
    let dir = tempfile::tempdir().unwrap();
    state.borrow_mut().cancel_output = Some(0xd8);
    let error = session.adjust_sensitivity(&b, dir.path(), 24).unwrap_err();
    let status = error.sensitivity.unwrap();
    assert!(status.sent && status.cancellation_requested);
    assert_eq!(status.after, Some(PHYSICAL.to_vec()));
    assert_eq!(
        state
            .borrow()
            .reads
            .iter()
            .filter(|op| **op == 0xdc)
            .count(),
        22
    );
    allowed(&state.borrow());
}

#[test]
fn cancellation_before_start_and_queued_preflight_input_dispatch_nothing() {
    for queued in [false, true] {
        let (mut session, state) = setup(config(&profile()));
        let baseline = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let sent = state.borrow().sent.clone();
        if queued {
            state.borrow_mut().queued.push_back((0, vec![0, 0xda, 19]));
        } else {
            session.cancellation_flag().store(true, Ordering::Release);
        }
        let error = session
            .adjust_sensitivity(&baseline, dir.path(), 24)
            .unwrap_err();
        let status = error.sensitivity.unwrap();
        assert!(!status.attempted && !status.sent);
        assert!(status.before.is_none() && status.after.is_none());
        assert_eq!(status.cancellation_requested, !queued);
        assert_eq!(state.borrow().sent, sent);
        assert_eq!(state.borrow().prepare, 2);
        if queued {
            assert_eq!(error.raw_reports, vec![vec![0, 0xda, 19]]);
            assert_eq!(status.trace.len(), 1);
        }
    }
}

#[test]
fn active_sensitivity_ownership_blocks_every_public_read_and_other_mutation() {
    let (mut session, state) = setup(config(&profile()));
    let baseline = session.read_baseline().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let sent = state.borrow().sent.clone();
    session.sensitivity = Some(crate::SensitivityStatus::preparing(24));
    assert!(session.version().is_err());
    assert!(session.read_profile().is_err());
    assert!(session.read_baseline().is_err());
    assert!(session.read_counters().is_err());
    assert!(session.read_lighting().is_err());
    assert!(session.read_raw_thresholds().is_err());
    assert!(
        session
            .apply(&baseline, baseline.profile(), dir.path())
            .is_err()
    );
    assert!(session.begin_calibration(&baseline, dir.path()).is_err());
    assert!(
        session
            .begin_held_key_calibration(&baseline, dir.path(), 30)
            .is_err()
    );
    assert!(
        session
            .adjust_sensitivity(&baseline, dir.path(), 24)
            .is_err()
    );
    assert_eq!(state.borrow().sent, sent);
    assert_eq!(state.borrow().prepare, 2);
}

#[test]
fn retained_calibration_owner_cannot_be_replaced_by_sensitivity() {
    let mut steps = script(24, false);
    steps.truncate(7);
    steps.push(version(&profile()));
    let mut dc2 = dc(false);
    dc2.request[2] = 2;
    for (_, row) in &mut dc2.replies {
        row[3] = 2;
    }
    steps.push(dc2);
    steps.push(step(0xd9, 0, vec![]));
    let (mut session, state) = setup(steps);
    let baseline = session.read_baseline().unwrap();
    let dir = tempfile::tempdir().unwrap();
    session
        .begin_held_key_calibration(&baseline, dir.path(), 30)
        .unwrap();
    let sent = state.borrow().sent.clone();
    let error = session
        .adjust_sensitivity(&baseline, dir.path(), 24)
        .unwrap_err();
    assert!(!error.sensitivity.unwrap().attempted);
    assert_eq!(
        session.calibration_status().unwrap().phase,
        crate::CalibrationPhase::AwaitingHeldKeys
    );
    assert_eq!(state.borrow().sent, sent);
    assert!(session.sensitivity.is_none());
}

#[test]
fn uncertain_d8_delivery_holds_owned_passive_capture_before_return_without_extra_output() {
    for short in [false, true] {
        let mut steps = script(24, false);
        steps.truncate(8);
        steps[7].result = if short {
            Ok(64)
        } else {
            Err("D8 native error; delivery unknown".into())
        };
        let (mut session, state) = setup(steps);
        let baseline = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let late = vec![0, 0xa7, 0x91, 0x25];
        state.borrow_mut().late = Some((100, late.clone()));
        let error = session
            .adjust_sensitivity(&baseline, dir.path(), 24)
            .unwrap_err();
        // This catches an immediate return/drop after an unknown native write.
        assert_eq!(state.borrow().sleeps, 2000);
        assert!(error.raw_reports.contains(&late));
        let status = error.sensitivity.unwrap();
        assert!(status.trace.iter().any(|t| {
            t.bytes == late
                && t.outcome
                    .as_ref()
                    .is_some_and(|s| s.contains("passive capture"))
        }));
        assert!(status.attempted && !status.sent && status.after.is_none());
        assert_eq!(status.before, Some(PHYSICAL.to_vec()));
        assert!(status.backup.is_some());
        assert_eq!(state.borrow().sent.len(), 8);
        assert!(error.message.contains("passive settling"));
        assert!(!error.message.contains("DC delivery may have occurred"));
        allowed(&state.borrow());
    }
}

#[test]
fn failed_post_f9_write_short_read_and_timeout_hold_owned_d8_settle_without_dc_dispatch() {
    for fault in 0..4 {
        let mut steps = script(24, false);
        steps.truncate(9);
        match fault {
            0 => steps[8].result = Err("post F9 native write error".into()),
            1 => steps[8].result = Ok(64),
            2 => steps[8].replies[0].1.truncate(5),
            _ => steps[8].replies.clear(),
        }
        let (mut session, state) = setup(steps);
        let baseline = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let late = vec![0, 0xf9, 0x43, 0x61, 0x70];
        state.borrow_mut().late = Some((1100, late.clone()));
        let error = session
            .adjust_sensitivity(&baseline, dir.path(), 24)
            .unwrap_err();
        assert_eq!(state.borrow().sleeps, 2000);
        assert!(error.raw_reports.contains(&late));
        let status = error.sensitivity.unwrap();
        assert!(status.trace.iter().any(|t| {
            t.bytes == late
                && t.outcome
                    .as_ref()
                    .is_some_and(|s| s.contains("passive capture"))
        }));
        assert!(status.attempted && status.sent && status.after.is_none());
        assert!(status.original_error.is_some());
        assert_eq!(state.borrow().sent.len(), 9);
        assert_eq!(
            state.borrow().sent.iter().filter(|r| r[1] == 0xdc).count(),
            1
        );
        assert!(error.message.contains("passive settling"));
        assert!(!error.message.contains("DC delivery may have occurred"));
        allowed(&state.borrow());
    }
}
