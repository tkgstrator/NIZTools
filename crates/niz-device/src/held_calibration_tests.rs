//! Strict native-boundary scripts: no reply exists until its exact request.
//! Mutations caught: DB insertion, early unlock, extra DD, incomplete snapshots,
//! physical/matrix confusion, cancellation interrupting DC, and lost evidence.
use crate::transport::Transport;
use crate::{CalibrationPhase, CalibrationStatus, LiveBaseline, Session, UnlockOutcome};
use niz_core::{Profile, Report, command};
use std::{
    cell::RefCell,
    collections::VecDeque,
    path::Path,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

const PHYSICAL_VALUES: [u8; 66] = [
    0, 1, 2, 3, 4, 5, 29, 28, 27, 26, 25, 24, 60, 61, 62, 6, 7, 8, 9, 10, 11, 35, 34, 33, 32, 31,
    30, 63, 65, 12, 13, 14, 15, 16, 17, 41, 40, 39, 38, 37, 36, 64, 18, 19, 20, 21, 22, 23, 47, 46,
    45, 44, 43, 42, 58, 48, 49, 50, 51, 52, 53, 57, 56, 55, 54, 59,
];
#[derive(Default)]
struct State {
    queue: VecDeque<std::result::Result<(u64, Vec<u8>), String>>,
    sent: Vec<(u64, Report)>,
    reads: Vec<(u8, bool)>,
    time: u64,
    locked: bool,
    current: u8,
    current_mode: u8,
    read_index: usize,
    cancel_read: Option<usize>,
    cancel_op: Option<u8>,
    cancel_mode: Option<u8>,
    cancel_output: Option<u8>,
    cancel: Option<Arc<AtomicBool>>,
    sleeps: Vec<(u64, bool)>,
    unexpected: Vec<String>,
}
struct Step {
    request: Report,
    replies: Vec<std::result::Result<(u64, Vec<u8>), String>>,
    short: bool,
    fail: bool,
}
fn step(op: u8, replies: Vec<Vec<u8>>) -> Step {
    Step {
        request: command(op),
        replies: replies.into_iter().map(|b| Ok((0, b))).collect(),
        short: false,
        fail: false,
    }
}
fn dc(rows: Vec<Vec<u8>>) -> Step {
    let mut s = step(0xdc, rows);
    s.request[2] = 1;
    s
}
fn dc2(rows: Vec<Vec<u8>>) -> Step {
    let mut s = step(0xdc, rows);
    s.request[2] = 2;
    s
}
fn unlock() -> Step {
    let mut s = step(0xd9, vec![]);
    s.request[2] = 1;
    s
}
struct Script {
    state: Rc<RefCell<State>>,
    steps: VecDeque<Step>,
}
impl Transport for Script {
    fn prepare(&mut self) -> std::result::Result<(), String> {
        if self.state.borrow().queue.is_empty() {
            Ok(())
        } else {
            Err("prepare attempted to discard queued evidence".into())
        }
    }
    fn queued_input(&mut self) -> std::result::Result<Option<Vec<u8>>, String> {
        self.state
            .borrow_mut()
            .queue
            .pop_front()
            .map(|r| r.map(|(_, b)| b))
            .transpose()
    }
    fn write(&mut self, bytes: &[u8]) -> std::result::Result<usize, String> {
        let report: Report = bytes[1..].try_into().unwrap();
        let mut s = self.state.borrow_mut();
        let now = s.time;
        s.sent.push((now, report));
        let Some(expected) = self.steps.pop_front() else {
            s.unexpected.push("extra output".into());
            return Err("extra output".into());
        };
        if report != expected.request {
            let err = format!(
                "expected {:02x}/{}, got {:02x}/{}",
                expected.request[1], expected.request[2], report[1], report[2]
            );
            s.unexpected.push(err.clone());
            return Err(err);
        }
        assert!(
            s.queue.is_empty() || report == unlock().request,
            "new command while earlier replies queued"
        );
        s.current = report[1];
        s.current_mode = report[2];
        s.read_index = 0;
        if report[1] == 0xd9 {
            s.locked = report[2] == 0;
        }
        s.queue.extend(expected.replies);
        if s.cancel_output == Some(report[1]) {
            s.cancel.as_ref().unwrap().store(true, Ordering::Release);
        }
        if expected.fail {
            return Err("output delivery unknown".into());
        }
        Ok(if expected.short { 64 } else { bytes.len() })
    }
    fn read(&mut self, bytes: &mut [u8], timeout: i32) -> std::result::Result<usize, String> {
        let mut s = self.state.borrow_mut();
        let current = s.current;
        let locked = s.locked;
        s.reads.push((current, locked));
        s.read_index += 1;
        if s.cancel_read == Some(s.read_index)
            && current == s.cancel_op.unwrap_or(0xdc)
            && s.cancel_mode.is_none_or(|mode| mode == s.current_mode)
        {
            s.cancel.as_ref().unwrap().store(true, Ordering::Release);
        }
        match s.queue.pop_front() {
            Some(Ok((delay, report))) => {
                s.time += delay;
                bytes[..report.len()].copy_from_slice(&report);
                Ok(report.len())
            }
            Some(Err(err)) => Err(err),
            None => {
                s.time += timeout as u64;
                Ok(0)
            }
        }
    }
    fn now_ms(&self) -> u64 {
        self.state.borrow().time
    }
    fn sleep_ms(&mut self, n: u64) {
        let mut s = self.state.borrow_mut();
        let locked = s.locked;
        s.sleeps.push((n, locked));
        s.time += n;
    }
}
fn profile() -> Profile {
    Profile::from_json(include_str!(
        "../../../docs/validation/fixtures/atom66-xrgb-ble-v1.0.48-settings.json"
    ))
    .unwrap()
}
fn version(p: &Profile) -> Vec<u8> {
    let mut r = command(0xf9);
    r[2..2 + p.firmware.len()].copy_from_slice(p.firmware.as_bytes());
    r.to_vec()
}
fn profile_steps(p: &Profile) -> Vec<Step> {
    let mut reports = p.reports().iter().map(|r| r.to_vec()).collect::<Vec<_>>();
    reports.push(vec![0xf6; 64]);
    vec![step(0xf9, vec![version(p)]), step(0xf2, reports)]
}
fn rgb_step(changed: bool) -> Step {
    let bytes = [if changed { 0x38 } else { 0x37 }; 198];
    let mut reports = vec![];
    for part in bytes.chunks(61) {
        let mut r = command(0xe0);
        r[2] = part.len() as u8;
        r[3..3 + part.len()].copy_from_slice(part);
        reports.push(r.to_vec());
    }
    reports.push(command(0xe6).to_vec());
    step(0xe2, reports)
}
fn rows() -> Vec<Vec<u8>> {
    (1..=11u8)
        .map(|row| {
            let mut r = command(0xdc);
            r[2] = row;
            r[3] = 1;
            for col in 0..6 {
                r[4 + col] = (row - 1) * 6 + col as u8;
            }
            r[10..].fill(0xa0 + row);
            r.to_vec()
        })
        .collect()
}
fn t_rows() -> Vec<Vec<u8>> {
    rows()
        .into_iter()
        .map(|mut row| {
            row[3] = 2;
            for value in &mut row[4..10] {
                *value += 100;
            }
            row
        })
        .collect()
}
fn t_values() -> Vec<u8> {
    PHYSICAL_VALUES.iter().map(|value| value + 100).collect()
}
fn prepare_steps() -> Vec<Step> {
    let p = profile();
    let mut steps = profile_steps(&p);
    steps.extend(profile_steps(&p));
    steps.push(rgb_step(false));
    steps.push(step(0xf9, vec![version(&p)]));
    steps.push(dc(rows()));
    steps.push(step(0xf9, vec![version(&p)]));
    steps.push(dc2(t_rows()));
    steps.push(step(0xd9, vec![]));
    steps
}
fn finish_steps(post_rows: Vec<Vec<u8>>, post_profile: Profile, changed_rgb: bool) -> Vec<Step> {
    let p = profile();
    let mut steps = vec![
        step(0xdd, vec![command(0xde).to_vec()]),
        step(0xf9, vec![version(&p)]),
        dc(post_rows),
        step(0xf9, vec![version(&p)]),
        dc2(t_rows()),
        unlock(),
    ];
    steps.extend(profile_steps(&post_profile));
    steps.push(rgb_step(changed_rgb));
    steps
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
    session.cancel_flag.store(false, Ordering::Release);
    (session, state)
}
fn begin(
    session: &mut Session,
    b: &LiveBaseline,
    path: &Path,
    position: usize,
) -> crate::Result<CalibrationStatus> {
    session.begin_held_key_calibration(b, path, position)
}
fn capture(status: &CalibrationStatus, which: &str) -> Option<Vec<u8>> {
    let held = status.held_key.as_ref()?;
    match which {
        "before" => held.before.clone(),
        "after" => held.after.clone(),
        "t_before" => held.t_before.clone(),
        "t_after" => held.t_after.clone(),
        _ => panic!("unknown capture"),
    }
}
fn assert_terminal_output(state: &State) {
    assert!(state.unexpected.is_empty(), "{:?}", state.unexpected);
    assert_eq!(state.sent.iter().filter(|(_, r)| r[1] == 0xdd).count(), 1);
    assert_eq!(
        state
            .sent
            .iter()
            .filter(|(_, r)| r[1] == 0xd9 && r[2] == 1)
            .count(),
        1
    );
    assert!(
        state
            .sent
            .iter()
            .all(|(_, r)| matches!(r[1], 0xf9 | 0xf2 | 0xe2 | 0xdc | 0xd9 | 0xdd))
    );
}
#[test]
fn held_only_captures_every_physical_position_and_keeps_post_dc_locked_until_one_unlock() {
    let mut steps = prepare_steps();
    steps.extend(finish_steps(rows(), profile(), false));
    let (mut session, state) = setup(steps);
    let b = session.read_baseline().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let prepared = begin(&mut session, &b, dir.path(), 30).unwrap();
    assert_eq!(prepared.phase, CalibrationPhase::AwaitingHeldKeys);
    assert_eq!(prepared.held_key.as_ref().unwrap().position, 30);
    assert!(!prepared.release_attempted && !prepared.release_sent && !prepared.release_completed);
    assert_eq!(capture(&prepared, "before"), Some(PHYSICAL_VALUES.to_vec()));
    assert_eq!(capture(&prepared, "after"), None);
    assert_eq!(capture(&prepared, "t_before"), Some(t_values()));
    assert_eq!(capture(&prepared, "t_after"), None);
    let sent = state.borrow().sent.clone();
    assert!(session.read_raw_thresholds().is_err());
    assert_eq!(state.borrow().sent, sent);
    session.calibrate_held_keys().unwrap();
    let sent = state.borrow().sent.clone();
    assert!(session.read_raw_thresholds().is_err());
    assert_eq!(state.borrow().sent, sent);
    let result = session.finish_calibration().unwrap();
    assert_eq!(result.status.phase, CalibrationPhase::Complete);
    assert_eq!(
        capture(&result.status, "after"),
        Some(PHYSICAL_VALUES.to_vec())
    );
    assert_eq!(capture(&result.status, "t_before"), Some(t_values()));
    assert_eq!(capture(&result.status, "t_after"), Some(t_values()));
    assert!(result.status.settings_verified && result.status.lighting_verified);
    assert_terminal_output(&state.borrow());
    let state = state.borrow();
    let dc_reads = state
        .reads
        .iter()
        .filter(|(op, _)| *op == 0xdc)
        .collect::<Vec<_>>();
    assert_eq!(dc_reads.len(), 44);
    assert!(dc_reads[..22].iter().all(|(_, locked)| !*locked));
    assert!(dc_reads[22..].iter().all(|(_, locked)| *locked));
}
#[test]
fn held_comparison_distinguishes_target_and_neighbor_changes_with_literal_physical_values() {
    for (matrix, position) in [(13, 30), (14, 31), (64, 41), (59, 65)] {
        let mut post = rows();
        post[matrix / 6][4 + matrix % 6] = 200;
        let mut expected = PHYSICAL_VALUES.to_vec();
        expected[position] = 200;
        let mut steps = prepare_steps();
        steps.extend(finish_steps(post, profile(), false));
        let (mut session, state) = setup(steps);
        let b = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        begin(&mut session, &b, dir.path(), 30).unwrap();
        session.calibrate_held_keys().unwrap();
        let result = session.finish_calibration().unwrap();
        assert_eq!(capture(&result.status, "after"), Some(expected));
        assert_eq!(
            capture(&result.status, "before"),
            Some(PHYSICAL_VALUES.to_vec())
        );
        assert_terminal_output(&state.borrow());
    }
}
#[test]
fn held_calibration_reports_selected_and_other_key_t_changes_without_conflating_r() {
    let mut post_t = t_rows();
    post_t[13 / 6][4 + 13 % 6] = 180; // Physical #30: A.
    post_t[59 / 6][4 + 59 % 6] = 190; // Physical #65: another key.
    let mut steps = prepare_steps();
    steps.extend(finish_steps(rows(), profile(), false));
    // The first DC2 is the baseline. The postflight DC2 is the second one.
    let post_t_index = steps
        .iter()
        .rposition(|step| step.request[1] == 0xdc && step.request[2] == 2)
        .unwrap();
    steps[post_t_index] = dc2(post_t);
    let (mut session, state) = setup(steps);
    let baseline = session.read_baseline().unwrap();
    let dir = tempfile::tempdir().unwrap();
    begin(&mut session, &baseline, dir.path(), 30).unwrap();
    session.calibrate_held_keys().unwrap();
    let result = session.finish_calibration().unwrap();
    let held = result.status.held_key.unwrap();
    assert_eq!(held.before, Some(PHYSICAL_VALUES.to_vec()));
    assert_eq!(held.after, Some(PHYSICAL_VALUES.to_vec()));
    let before = t_values();
    let mut after = before.clone();
    after[30] = 180;
    after[65] = 190;
    assert_eq!(held.t_before, Some(before.clone()));
    assert_eq!(held.t_after, Some(after.clone()));
    assert_eq!(
        before
            .iter()
            .zip(after.iter())
            .filter(|(a, b)| a != b)
            .count(),
        2
    );
    assert_terminal_output(&state.borrow());
}
#[test]
fn complete_threshold_observations_survive_later_settings_or_lighting_failure() {
    for rgb in [false, true] {
        let mut p = profile();
        if !rgb {
            p.set_definition(0, &p.definition(1).unwrap()).unwrap();
        }
        let mut post = rows();
        post[2][5] = 201;
        let mut steps = prepare_steps();
        steps.extend(finish_steps(post, p, rgb));
        let (mut session, state) = setup(steps);
        let b = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        begin(&mut session, &b, dir.path(), 30).unwrap();
        session.calibrate_held_keys().unwrap();
        let err = session.finish_calibration().unwrap_err();
        let status = err.calibration.unwrap();
        assert_eq!(status.phase, CalibrationPhase::Failed);
        assert_eq!(status.unlock, UnlockOutcome::Sent);
        assert!(status.held_completed);
        assert_eq!(capture(&status, "before"), Some(PHYSICAL_VALUES.to_vec()));
        assert_eq!(capture(&status, "t_before"), Some(t_values()));
        let mut expected = PHYSICAL_VALUES.to_vec();
        expected[30] = 201;
        assert_eq!(capture(&status, "after"), Some(expected));
        assert_eq!(capture(&status, "t_after"), Some(t_values()));
        assert!(!status.settings_verified && !status.lighting_verified);
        assert_terminal_output(&state.borrow());
        assert!(err.message.contains("cannot be restored"));
    }
}
#[test]
fn incomplete_malformed_late_and_extra_post_capture_never_publish_after_and_settle_before_unlock() {
    for failure in 0..5 {
        let mut steps = prepare_steps();
        steps.push(step(0xdd, vec![command(0xde).to_vec()]));
        steps.push(step(0xf9, vec![version(&profile())]));
        let mut post = dc(rows());
        match failure {
            0 => post.replies.truncate(10),
            1 => post.replies[3].as_mut().unwrap().1[3] = 2,
            2 => post.replies[10].as_mut().unwrap().0 = 30_001,
            3 => post.replies.push(Ok((0, command(0xdc).to_vec()))),
            _ => post.short = true,
        };
        steps.push(post);
        steps.push(unlock());
        let (mut session, state) = setup(steps);
        let b = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        begin(&mut session, &b, dir.path(), 30).unwrap();
        session.calibrate_held_keys().unwrap();
        let err = session.finish_calibration().unwrap_err();
        let status = err.calibration.unwrap();
        assert_eq!(capture(&status, "before"), Some(PHYSICAL_VALUES.to_vec()));
        assert_eq!(capture(&status, "after"), None);
        assert_eq!(capture(&status, "t_before"), Some(t_values()));
        assert_eq!(capture(&status, "t_after"), None);
        assert!(status.held_sent && status.held_completed);
        assert_eq!(status.unlock, UnlockOutcome::Sent);
        assert!(err.message.contains("passive settling"));
        let state = state.borrow();
        assert_terminal_output(&state);
        assert_eq!(state.sleeps.iter().map(|(n, _)| n).sum::<u64>(), 2000);
        assert!(state.sleeps.iter().all(|(_, locked)| *locked));
        assert!(!err.raw_reports.is_empty());
    }
}
#[test]
fn failed_pre_capture_cannot_lock_or_dispatch_dd_and_does_not_publish_partial_values() {
    let mut steps = prepare_steps();
    steps.truncate(7);
    steps[6].replies.truncate(4);
    let (mut session, state) = setup(steps);
    let b = session.read_baseline().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let err = begin(&mut session, &b, dir.path(), 30).unwrap_err();
    let status = err.calibration.unwrap();
    assert_eq!(capture(&status, "before"), None);
    assert_eq!(capture(&status, "t_before"), None);
    assert_eq!(status.unlock, UnlockOutcome::NotAttempted);
    assert!(!status.lock_may_have_been_sent);
    assert!(
        state
            .borrow()
            .sent
            .iter()
            .all(|(_, r)| !matches!(r[1], 0xd9 | 0xdd | 0xdb))
    );
    assert_eq!(
        state.borrow().sleeps.iter().map(|(n, _)| n).sum::<u64>(),
        2000
    );
}
#[test]
fn failed_pre_t_capture_cannot_lock_or_claim_complete_t_values() {
    let mut steps = prepare_steps();
    steps[8].replies.truncate(5);
    let (mut session, state) = setup(steps);
    let baseline = session.read_baseline().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let error = begin(&mut session, &baseline, dir.path(), 30).unwrap_err();
    let status = error.calibration.unwrap();
    assert_eq!(status.phase, CalibrationPhase::Failed);
    assert_eq!(capture(&status, "before"), Some(PHYSICAL_VALUES.to_vec()));
    assert_eq!(capture(&status, "t_before"), None);
    assert!(!status.lock_may_have_been_sent);
    assert_eq!(status.unlock, UnlockOutcome::NotAttempted);
    assert!(
        state
            .borrow()
            .sent
            .iter()
            .all(|(_, r)| r[1] != 0xd9 && r[1] != 0xdd)
    );
    assert!(error.message.contains("passive settling"));
}
#[test]
fn failed_post_t_capture_does_not_claim_complete_t_values_and_unlocks_once() {
    let mut steps = prepare_steps();
    steps.extend(finish_steps(rows(), profile(), false));
    let post_t = steps
        .iter()
        .rposition(|step| step.request[1] == 0xdc && step.request[2] == 2)
        .unwrap();
    steps[post_t].replies[4].as_mut().unwrap().1[3] = 1;
    let (mut session, state) = setup(steps);
    let baseline = session.read_baseline().unwrap();
    let dir = tempfile::tempdir().unwrap();
    begin(&mut session, &baseline, dir.path(), 30).unwrap();
    session.calibrate_held_keys().unwrap();
    let error = session.finish_calibration().unwrap_err();
    let status = error.calibration.unwrap();
    assert_eq!(status.phase, CalibrationPhase::Failed);
    assert!(status.held_completed);
    assert_eq!(capture(&status, "t_before"), Some(t_values()));
    assert_eq!(capture(&status, "t_after"), None);
    assert_eq!(status.unlock, UnlockOutcome::Sent);
    assert!(error.message.contains("passive settling"));
    assert_terminal_output(&state.borrow());
}
#[test]
fn cancellation_during_pre_and_post_dc_completes_the_bounded_reader_before_cleanup() {
    for post in [false, true] {
        let mut steps = prepare_steps();
        if post {
            steps.extend(finish_steps(rows(), profile(), false));
            steps.truncate(13);
            steps.push(unlock());
        } else {
            steps.truncate(7);
        }
        let (mut session, state) = setup(steps);
        let b = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        if post {
            begin(&mut session, &b, dir.path(), 30).unwrap();
            session.calibrate_held_keys().unwrap();
        }
        state.borrow_mut().cancel_read = Some(3);
        let err = if post {
            session.finish_calibration().unwrap_err()
        } else {
            begin(&mut session, &b, dir.path(), 30).unwrap_err()
        };
        let status = err.calibration.unwrap();
        assert_eq!(status.phase, CalibrationPhase::Aborted);
        assert!(status.cancellation_requested);
        assert_eq!(capture(&status, "before"), Some(PHYSICAL_VALUES.to_vec()));
        assert_eq!(
            capture(&status, "after"),
            post.then(|| PHYSICAL_VALUES.to_vec())
        );
        assert_eq!(
            capture(&status, "t_before"),
            if post { Some(t_values()) } else { None }
        );
        assert_eq!(capture(&status, "t_after"), None);
        let state = state.borrow();
        assert_eq!(
            state.reads.iter().filter(|(op, _)| *op == 0xdc).count(),
            if post { 33 } else { 11 }
        );
        assert!(state.unexpected.is_empty());
        if post {
            assert_terminal_output(&state);
        } else {
            assert!(!state.sent.iter().any(|(_, r)| matches!(r[1], 0xd9 | 0xdd)));
        }
    }
}
#[test]
fn cancellation_during_pre_dc2_finishes_t_snapshot_without_locking() {
    let mut steps = prepare_steps();
    steps.truncate(9); // Through DC2; D9 must never be requested.
    let (mut session, state) = setup(steps);
    let baseline = session.read_baseline().unwrap();
    let dir = tempfile::tempdir().unwrap();
    {
        let mut script = state.borrow_mut();
        script.cancel_read = Some(3);
        script.cancel_op = Some(0xdc);
        script.cancel_mode = Some(2);
    }
    let error = begin(&mut session, &baseline, dir.path(), 30).unwrap_err();
    let status = error.calibration.unwrap();
    assert_eq!(status.phase, CalibrationPhase::Aborted);
    assert!(status.cancellation_requested);
    assert_eq!(status.unlock, UnlockOutcome::NotAttempted);
    assert!(!status.lock_may_have_been_sent);
    assert_eq!(capture(&status, "before"), Some(PHYSICAL_VALUES.to_vec()));
    assert_eq!(capture(&status, "t_before"), Some(t_values()));
    assert_eq!(capture(&status, "after"), None);
    assert_eq!(capture(&status, "t_after"), None);
    let script = state.borrow();
    assert_eq!(
        script.reads.iter().filter(|(op, _)| *op == 0xdc).count(),
        22
    );
    assert_eq!(
        status
            .trace
            .iter()
            .filter(|packet| {
                packet.direction == crate::TraceDirection::Input
                    && packet.bytes.get(1) == Some(&0xdc)
                    && packet.bytes.get(3) == Some(&2)
            })
            .count(),
        11
    );
    assert!(script.unexpected.is_empty());
    assert!(
        script
            .sent
            .iter()
            .all(|(_, report)| report[1] != 0xd9 && report[1] != 0xdd)
    );
}
#[test]
fn cancellation_during_post_dc2_finishes_t_snapshot_then_unlocks_once() {
    let mut steps = prepare_steps();
    steps.extend(finish_steps(rows(), profile(), false));
    steps.truncate(16); // Through the one cleanup unlock, without postflight commands.
    let (mut session, state) = setup(steps);
    let baseline = session.read_baseline().unwrap();
    let dir = tempfile::tempdir().unwrap();
    begin(&mut session, &baseline, dir.path(), 30).unwrap();
    session.calibrate_held_keys().unwrap();
    {
        let mut script = state.borrow_mut();
        script.cancel_read = Some(3);
        script.cancel_op = Some(0xdc);
        script.cancel_mode = Some(2);
    }
    let error = session.finish_calibration().unwrap_err();
    let status = error.calibration.unwrap();
    assert_eq!(status.phase, CalibrationPhase::Aborted);
    assert!(status.cancellation_requested && status.held_completed);
    assert_eq!(status.unlock, UnlockOutcome::Sent);
    assert_eq!(capture(&status, "before"), Some(PHYSICAL_VALUES.to_vec()));
    assert_eq!(capture(&status, "t_before"), Some(t_values()));
    assert_eq!(capture(&status, "after"), Some(PHYSICAL_VALUES.to_vec()));
    assert_eq!(capture(&status, "t_after"), Some(t_values()));
    let script = state.borrow();
    assert_eq!(
        script.reads.iter().filter(|(op, _)| *op == 0xdc).count(),
        44
    );
    assert_eq!(
        status
            .trace
            .iter()
            .filter(|packet| {
                packet.direction == crate::TraceDirection::Input
                    && packet.bytes.get(1) == Some(&0xdc)
                    && packet.bytes.get(3) == Some(&2)
            })
            .count(),
        22
    );
    assert_terminal_output(&script);
    assert_eq!(script.sent.last().unwrap().1, unlock().request);
}
#[test]
fn invalid_physical_position_is_rejected_before_preparation_traffic() {
    for position in [66, usize::MAX] {
        let (mut session, state) = setup(profile_steps(&profile()));
        let b = session.read_baseline().unwrap();
        let sent = state.borrow().sent.clone();
        let dir = tempfile::tempdir().unwrap();
        assert!(begin(&mut session, &b, dir.path(), position).is_err());
        assert_eq!(state.borrow().sent, sent);
        assert!(session.calibration_status().is_none());
    }
}

#[test]
fn held_only_unknown_dd_delivery_retains_before_without_cleanup_retry_or_false_completion() {
    for short in [false, true] {
        let mut steps = prepare_steps();
        let mut dd = step(0xdd, vec![command(0xde).to_vec()]);
        dd.short = short;
        dd.fail = !short;
        steps.push(dd);
        let (mut session, state) = setup(steps);
        let b = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        begin(&mut session, &b, dir.path(), 65).unwrap();
        let err = session.calibrate_held_keys().unwrap_err();
        let status = err.calibration.unwrap();
        assert_eq!(status.phase, CalibrationPhase::Failed);
        assert!(status.held_attempted && !status.held_sent && !status.held_completed);
        assert_eq!(capture(&status, "before"), Some(PHYSICAL_VALUES.to_vec()));
        assert_eq!(capture(&status, "after"), None);
        assert_eq!(capture(&status, "t_before"), Some(t_values()));
        assert_eq!(capture(&status, "t_after"), None);
        assert_eq!(status.unlock, UnlockOutcome::NotAttempted);
        assert!(err.began_write);
        assert!(err.message.contains("cannot be restored"));
        let sent = state.borrow().sent.clone();
        assert!(session.calibrate_held_keys().is_err());
        assert!(session.finish_calibration().is_err());
        assert!(session.cancel_calibration().is_err());
        assert!(session.read_raw_thresholds().is_err());
        assert_eq!(state.borrow().sent, sent);
        assert!(state.borrow().unexpected.is_empty());
    }
}
#[test]
fn held_only_late_malformed_and_timed_out_de_never_start_post_dc_and_unlock_once() {
    for failure in 0..3 {
        let mut steps = prepare_steps();
        let mut dd = step(0xdd, vec![command(0xde).to_vec()]);
        match failure {
            0 => dd.replies[0].as_mut().unwrap().0 = 10_001,
            1 => dd.replies[0].as_mut().unwrap().1[1] = 0xda,
            _ => dd.replies.clear(),
        };
        steps.push(dd);
        steps.push(unlock());
        let (mut session, state) = setup(steps);
        let b = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        begin(&mut session, &b, dir.path(), 0).unwrap();
        let err = session.calibrate_held_keys().unwrap_err();
        let status = err.calibration.unwrap();
        assert_eq!(status.phase, CalibrationPhase::Failed);
        assert_eq!(status.failure_phase, Some(CalibrationPhase::HeldWaiting));
        assert!(status.held_attempted && status.held_sent && !status.held_completed);
        assert_eq!(status.unlock, UnlockOutcome::Sent);
        assert_eq!(capture(&status, "before"), Some(PHYSICAL_VALUES.to_vec()));
        assert_eq!(capture(&status, "after"), None);
        assert_eq!(capture(&status, "t_before"), Some(t_values()));
        assert_eq!(capture(&status, "t_after"), None);
        let state = state.borrow();
        assert_terminal_output(&state);
        assert_eq!(state.sent.iter().filter(|(_, r)| r[1] == 0xdc).count(), 2);
    }
}
#[test]
fn held_only_cancel_before_dd_after_dispatch_and_after_de_retains_exact_command_state() {
    for stage in 0..4 {
        let mut steps = prepare_steps();
        if stage > 0 {
            steps.push(step(0xdd, vec![command(0xde).to_vec()]));
        }
        steps.push(unlock());
        let (mut session, state) = setup(steps);
        let b = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        begin(&mut session, &b, dir.path(), 31).unwrap();
        let status = match stage {
            0 => session.cancel_calibration().unwrap(),
            1 => {
                state.borrow_mut().cancel_output = Some(0xdd);
                *session
                    .calibrate_held_keys()
                    .unwrap_err()
                    .calibration
                    .unwrap()
            }
            2 => {
                let mut s = state.borrow_mut();
                s.cancel_read = Some(1);
                s.cancel_op = Some(0xdd);
                drop(s);
                *session
                    .calibrate_held_keys()
                    .unwrap_err()
                    .calibration
                    .unwrap()
            }
            _ => {
                session.calibrate_held_keys().unwrap();
                session.cancel_calibration().unwrap()
            }
        };
        assert_eq!(status.phase, CalibrationPhase::Aborted);
        assert!(status.cancellation_requested);
        assert_eq!(status.unlock, UnlockOutcome::Sent);
        assert_eq!(capture(&status, "before"), Some(PHYSICAL_VALUES.to_vec()));
        assert_eq!(capture(&status, "after"), None);
        assert_eq!(capture(&status, "t_before"), Some(t_values()));
        assert_eq!(capture(&status, "t_after"), None);
        assert_eq!(status.held_attempted, stage > 0);
        assert_eq!(status.held_sent, stage > 0);
        assert_eq!(status.held_completed, stage == 3);
        let state = state.borrow();
        assert!(state.unexpected.is_empty());
        assert_eq!(state.sent.iter().filter(|(_, r)| r[1] == 0xdc).count(), 2);
        assert_eq!(
            state.sent.iter().filter(|(_, r)| r[1] == 0xdd).count(),
            usize::from(stage > 0)
        );
        assert_eq!(
            state
                .sent
                .iter()
                .filter(|(_, r)| r[1] == 0xd9 && r[2] == 1)
                .count(),
            1
        );
    }
}
