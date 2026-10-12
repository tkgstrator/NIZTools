use crate::transport::Transport;
use crate::{RawThresholds, Session, supports_raw_thresholds};
use niz_core::{DeviceIdentity, Report, command};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

const FIRMWARE: &str = "66EC(XRGB)BLe;V1.0.48;V1.0;";

#[derive(Default)]
struct State {
    sent: Vec<Vec<u8>>,
    time: u64,
    read_timeouts: Vec<i32>,
    prepares: usize,
    fail_prepare: Option<usize>,
    short_write: Option<usize>,
    fail_write: Option<usize>,
    queue: VecDeque<Vec<u8>>,
    queue_after_f9: VecDeque<Vec<u8>>,
    queue_after_dc: VecDeque<Vec<u8>>,
    queue_probes: usize,
    fail_queue_probe: Option<usize>,
    capture_remaining: bool,
    freeze_clock: bool,
    sleeps: Vec<u64>,
    lock_probe: Option<(std::path::PathBuf, String)>,
    held_lock_observations: Vec<bool>,
    dropped_at: Option<u64>,
    overrun_read: Option<usize>,
}

struct Fake {
    state: Rc<RefCell<State>>,
    incoming: VecDeque<std::result::Result<Vec<u8>, String>>,
    _lock: Option<std::fs::File>,
}
impl Drop for Fake {
    fn drop(&mut self) {
        let mut state = self.state.borrow_mut();
        state.dropped_at = Some(state.time);
    }
}
impl Transport for Fake {
    fn prepare(&mut self) -> std::result::Result<(), String> {
        let mut state = self.state.borrow_mut();
        state.prepares += 1;
        if state.fail_prepare == Some(state.prepares) {
            return Err("bounded queue preparation failed".into());
        }
        state.queue.clear();
        Ok(())
    }
    fn queued_input(&mut self) -> std::result::Result<Option<Vec<u8>>, String> {
        let mut state = self.state.borrow_mut();
        state.queue_probes += 1;
        if state.fail_queue_probe == Some(state.queue_probes) {
            return Err("queued input disconnected".into());
        }
        if let Some(report) = state.queue.pop_front() {
            return Ok(Some(report));
        }
        if state.capture_remaining && state.sent.len() >= 2 {
            return self.incoming.pop_front().transpose();
        }
        Ok(None)
    }
    fn write(&mut self, bytes: &[u8]) -> std::result::Result<usize, String> {
        let mut state = self.state.borrow_mut();
        state.sent.push(bytes.to_vec());
        if bytes[2] == 0xf9 {
            let extra = std::mem::take(&mut state.queue_after_f9);
            state.queue.extend(extra);
        }
        if bytes[2] == 0xdc {
            let extra = std::mem::take(&mut state.queue_after_dc);
            state.queue.extend(extra);
        }
        if state.fail_write == Some(state.sent.len()) {
            Err("write disconnected; delivery unknown".into())
        } else if state.short_write == Some(state.sent.len()) {
            Ok(64)
        } else {
            Ok(bytes.len())
        }
    }
    fn read(&mut self, bytes: &mut [u8], timeout: i32) -> std::result::Result<usize, String> {
        let mut state = self.state.borrow_mut();
        state.read_timeouts.push(timeout);
        if state.overrun_read == Some(state.read_timeouts.len()) {
            state.time += 30_001;
        }
        match self.incoming.pop_front() {
            Some(Ok(report)) => {
                bytes[..report.len()].copy_from_slice(&report);
                Ok(report.len())
            }
            Some(Err(error)) => Err(error),
            None => {
                state.time += timeout as u64;
                Ok(0)
            }
        }
    }
    fn now_ms(&self) -> u64 {
        self.state.borrow().time
    }
    fn sleep_ms(&mut self, milliseconds: u64) {
        let mut state = self.state.borrow_mut();
        state.sleeps.push(milliseconds);
        if !state.freeze_clock {
            state.time += milliseconds;
        }
        if let Some((directory, path)) = &state.lock_probe {
            let held = crate::transport::acquire_lock(directory, path).is_err();
            state.held_lock_observations.push(held);
        }
    }
}

fn identity() -> DeviceIdentity {
    DeviceIdentity {
        vendor_id: 0x0483,
        product_id: 0x502a,
        usage_page: 0x8c,
        usage: 1,
    }
}
fn version(firmware: &str) -> Vec<u8> {
    let mut report = command(0xf9);
    report[2..2 + firmware.len()].copy_from_slice(firmware.as_bytes());
    report.to_vec()
}
fn rows() -> Vec<Report> {
    (1..=11u8)
        .map(|row| {
            let mut report = command(0xdc);
            report[2] = row;
            report[3] = 1;
            for column in 0..6usize {
                report[4 + column] = (row - 1) * 6 + column as u8;
            }
            report[10..].fill(0xc0 + row);
            report
        })
        .collect()
}
fn replies(firmware: &str) -> Vec<std::result::Result<Vec<u8>, String>> {
    std::iter::once(Ok(version(firmware)))
        .chain(rows().into_iter().map(|report| Ok(report.to_vec())))
        .collect()
}
fn session(incoming: Vec<std::result::Result<Vec<u8>, String>>) -> (Session, Rc<RefCell<State>>) {
    let state = Rc::new(RefCell::new(State::default()));
    let transport = Fake {
        state: state.clone(),
        incoming: incoming.into(),
        _lock: None,
    };
    (
        Session::from_transport(Box::new(transport), identity()),
        state,
    )
}
fn expected_commands() -> Vec<Vec<u8>> {
    let mut getter = [0; 65];
    getter[2] = 0xdc;
    getter[3] = 1;
    let mut fresh_version = [0; 65];
    fresh_version[2] = 0xf9;
    vec![fresh_version.to_vec(), getter.to_vec()]
}

fn calibration_rows() -> Vec<Report> {
    rows()
        .into_iter()
        .map(|mut report| {
            report[3] = 2;
            report
        })
        .collect()
}

fn calibration_replies() -> Vec<std::result::Result<Vec<u8>, String>> {
    std::iter::once(Ok(version(FIRMWARE)))
        .chain(
            calibration_rows()
                .into_iter()
                .map(|report| Ok(report.to_vec())),
        )
        .collect()
}

#[test]
fn calibration_getter_reads_ram_t_with_only_f9_and_dc_mode_two() {
    let (mut session, state) = session(calibration_replies());
    let snapshot = session.read_calibration_values().unwrap();
    assert_eq!(snapshot.identity, identity());
    assert_eq!(snapshot.firmware, FIRMWARE);
    assert_eq!(snapshot.raw_reports(), calibration_rows());
    assert_eq!(snapshot.value_for_position(30), Some(13));
    assert_eq!(snapshot.value_for_position(31), Some(14));
    assert_eq!(snapshot.value_for_position(65), Some(59));
    assert_eq!(snapshot.value_for_position(66), None);
    let mut expected = expected_commands();
    expected[1][3] = 2;
    let state = state.borrow();
    assert_eq!(state.sent, expected);
    assert_eq!(state.prepares, 1);
    assert_eq!(state.queue_probes, 2);
    assert!(state.sleeps.is_empty());
    assert_eq!(state.read_timeouts, vec![1000; 12]);
}

#[test]
fn calibration_getter_rejects_wrong_mode_and_keeps_evidence_without_retry() {
    let mut incoming = calibration_replies();
    incoming[3].as_mut().unwrap()[3] = 1;
    let expected = incoming[..=3]
        .iter()
        .map(|item| item.as_ref().unwrap().clone())
        .collect::<Vec<_>>();
    let (mut session, state) = session(incoming);
    let error = session.read_calibration_values().unwrap_err();
    assert_eq!(error.raw_reports, expected);
    assert!(error.message.contains("restoration not confirmed"));
    assert!(!error.began_write);
    assert!(session.poisoned);
    assert!(session.read_calibration_values().is_err());
    let mut commands = expected_commands();
    commands[1][3] = 2;
    assert_eq!(state.borrow().sent, commands);
    assert_eq!(state.borrow().sleeps.iter().sum::<u64>(), 2000);
}

#[test]
fn calibration_getter_rejects_firmware_drift_before_dc() {
    let incoming = std::iter::once(Ok(version("66EC(XRGB)BLe;V1.0.49;V1.0;")))
        .chain(
            calibration_rows()
                .into_iter()
                .map(|report| Ok(report.to_vec())),
        )
        .collect();
    let (mut session, state) = session(incoming);
    let error = session.read_calibration_values().unwrap_err();
    assert_eq!(
        error.raw_reports,
        vec![version("66EC(XRGB)BLe;V1.0.49;V1.0;")]
    );
    assert_eq!(state.borrow().sent, vec![expected_commands()[0].clone()]);
    assert!(state.borrow().sleeps.is_empty());
}

#[test]
fn calibration_getter_rejects_excess_input_after_eleven_rows() {
    let (mut session, state) = session(calibration_replies());
    let extra = vec![0, 0xdc, 12, 2];
    state.borrow_mut().queue_after_dc.push_back(extra.clone());
    let error = session.read_calibration_values().unwrap_err();
    assert_eq!(error.raw_reports.first(), Some(&version(FIRMWARE)));
    assert!(error.raw_reports.contains(&extra));
    assert!(error.message.contains("restoration not confirmed"));
    assert!(!error.began_write);
    assert!(session.poisoned);
    assert_eq!(state.borrow().sleeps.iter().sum::<u64>(), 2000);
}

#[test]
fn calibration_getter_short_dc_write_keeps_lock_during_bounded_settle() {
    let directory = tempfile::tempdir().unwrap();
    let path = "fake-calibration-value-native-path";
    let lock = crate::transport::acquire_lock(directory.path(), path).unwrap();
    let state = Rc::new(RefCell::new(State::default()));
    {
        let mut state = state.borrow_mut();
        state.lock_probe = Some((directory.path().into(), path.into()));
        state.short_write = Some(2);
        state.capture_remaining = true;
    }
    let mut session = Session::from_transport(
        Box::new(Fake {
            state: state.clone(),
            incoming: calibration_replies().into(),
            _lock: Some(lock),
        }),
        identity(),
    );
    let error = session.read_calibration_values().unwrap_err();
    assert_eq!(error.raw_reports.len(), 12);
    assert!(error.message.contains("restoration not confirmed"));
    assert!(!error.began_write);
    let state = state.borrow();
    assert_eq!(state.sleeps.iter().sum::<u64>(), 2000);
    assert!(!state.held_lock_observations.is_empty());
    assert!(state.held_lock_observations.iter().all(|held| *held));
    assert_eq!(state.sent.len(), 2);
    assert_eq!(state.sent[1][3], 2);
}

#[test]
fn calibration_getter_uses_one_absolute_deadline() {
    let (mut session, state) = session(calibration_replies());
    state.borrow_mut().overrun_read = Some(2);
    let error = session.read_calibration_values().unwrap_err();
    assert!(error.message.contains("deadline"));
    assert!(session.poisoned);
    assert_eq!(state.borrow().sent.len(), 2);
    assert_eq!(state.borrow().sleeps.iter().sum::<u64>(), 2000);
}

#[test]
fn getter_uses_only_fresh_f9_and_exact_zero_padded_dc_mode_one() {
    let (mut session, state) = session(replies(FIRMWARE));
    let snapshot = session.read_raw_thresholds().unwrap();
    assert_eq!(snapshot.identity, identity());
    assert_eq!(snapshot.firmware, FIRMWARE);
    assert_eq!(snapshot.raw_reports(), rows());
    assert_eq!(snapshot.value_for_position(30), Some(13));
    assert_eq!(snapshot.value_for_position(31), Some(14));
    assert_eq!(snapshot.value_for_position(41), Some(64));
    assert_eq!(snapshot.value_for_position(65), Some(59));
    assert_eq!(snapshot.value_for_position(66), None);
    assert_eq!(snapshot.value_for_position(usize::MAX), None);
    let state = state.borrow();
    assert_eq!(state.sent, expected_commands());
    assert_eq!(state.prepares, 1);
    assert_eq!(state.queue_probes, 2);
    assert!(state.sleeps.is_empty());
    assert_eq!(state.read_timeouts, vec![1000; 12]);
}

#[test]
fn eligibility_and_read_reject_each_non_exact_identity_before_any_command() {
    assert!(supports_raw_thresholds(&identity(), FIRMWARE));
    let mut identities = vec![];
    for value in [0, 0x512a, 0x522a, 0xffff] {
        let mut wrong = identity();
        wrong.product_id = value;
        identities.push(wrong);
    }
    for offset in 0..3 {
        let mut wrong = identity();
        match offset {
            0 => wrong.vendor_id = 0,
            1 => wrong.usage_page = 0,
            _ => wrong.usage = 0,
        }
        identities.push(wrong);
    }
    for wrong in identities {
        assert!(!supports_raw_thresholds(&wrong, FIRMWARE));
        let (mut session, state) = session(replies(FIRMWARE));
        session.identity = wrong;
        assert!(session.read_raw_thresholds().is_err());
        assert!(state.borrow().sent.is_empty());
    }
}

#[test]
fn eligibility_and_fresh_version_gate_reject_every_non_exact_firmware_before_dc() {
    for firmware in [
        "",
        "66EC(XRGB)BLe;V1.0.49;V1.0;",
        "66EC(XRGB)BLe;V1.0.48;V1.1;",
        "66EC(XRGB)BLe;V1.0.48;V1.0",
        "66EC(XRGB)BLe;V1.0.48;V1.0;extra",
        "66EC(XRGB)BLE;V1.0.48;V1.0;",
        "66EC(S)BLe;V1.0.48;V1.0;",
    ] {
        assert!(!supports_raw_thresholds(&identity(), firmware));
        let (mut session, state) = session(replies(firmware));
        let error = session.read_raw_thresholds().unwrap_err();
        assert!(!error.began_write);
        assert_eq!(error.raw_reports, vec![version(firmware)]);
        assert_eq!(state.borrow().sent, vec![expected_commands()[0].clone()]);
        assert!(session.read_raw_thresholds().is_err());
        assert_eq!(state.borrow().sent.len(), 1);
    }
}

#[test]
fn stale_baseline_firmware_cannot_authorize_a_fresh_different_device_firmware() {
    let (mut session, state) = session(replies("66EC(XRGB)BLe;V1.0.49;V1.0;"));
    let mut baseline = niz_core::Profile::demo();
    baseline.identity = identity();
    baseline.firmware = FIRMWARE.into();
    session.last_read = Some(baseline);
    assert!(session.read_raw_thresholds().is_err());
    assert_eq!(state.borrow().sent.len(), 1);
    assert!(session.last_read.is_none());

    let (mut session, state) = self::session(replies(FIRMWARE));
    let mut baseline = niz_core::Profile::demo();
    baseline.identity = identity();
    baseline.firmware = "66EC(XRGB)BLe;V1.0.49;V1.0;".into();
    session.last_read = Some(baseline);
    assert!(session.read_raw_thresholds().is_err());
    assert_eq!(state.borrow().sent.len(), 1);
}

#[test]
fn successful_getter_preserves_existing_baseline_and_generation() {
    let (mut session, _) = session(replies(FIRMWARE));
    let mut baseline = niz_core::Profile::demo();
    baseline.identity = identity();
    baseline.firmware = FIRMWARE.into();
    session.last_read = Some(baseline.clone());
    session.generation = 17;
    session.read_raw_thresholds().unwrap();
    assert_eq!(session.generation, 17);
    assert_eq!(session.last_read.unwrap().reports(), baseline.reports());
}

#[test]
fn malformed_stream_keeps_every_received_frame_and_poison_prevents_retry() {
    for (offset, value) in [(0, 1), (1, 0xdf), (2, 0), (2, 1), (2, 12), (3, 0), (3, 2)] {
        let mut incoming = replies(FIRMWARE);
        let bad = incoming[4].as_mut().unwrap();
        bad[offset] = value;
        let expected = incoming[..=4]
            .iter()
            .map(|r| r.as_ref().unwrap().clone())
            .collect::<Vec<_>>();
        let (mut session, state) = session(incoming);
        let error = session.read_raw_thresholds().unwrap_err();
        assert_eq!(error.raw_reports, expected);
        assert!(!error.began_write);
        assert!(session.poisoned);
        assert!(session.read_raw_thresholds().is_err());
        assert_eq!(state.borrow().sent, expected_commands());
    }
}

#[test]
fn truncated_oversize_missing_and_io_failed_streams_preserve_partial_evidence() {
    for length in [0, 3, 63, 65] {
        let mut incoming = replies(FIRMWARE);
        incoming[3] = Ok(vec![0x9a; length]);
        let mut expected = incoming[..3]
            .iter()
            .map(|r| r.as_ref().unwrap().clone())
            .collect::<Vec<_>>();
        if length > 0 {
            expected.push(vec![0x9a; length]);
        }
        let (mut session, state) = session(incoming);
        let error = session.read_raw_thresholds().unwrap_err();
        assert_eq!(error.raw_reports, expected);
        assert!(!error.began_write);
        assert!(session.poisoned);
        assert_eq!(state.borrow().sent, expected_commands());
    }
    for io_error in [false, true] {
        let mut incoming = replies(FIRMWARE);
        incoming.truncate(4);
        if io_error {
            incoming.push(Err("read disconnected".into()));
        }
        let expected = incoming
            .iter()
            .filter_map(|r| r.clone().ok())
            .collect::<Vec<_>>();
        let (mut session, state) = session(incoming);
        let error = session.read_raw_thresholds().unwrap_err();
        assert_eq!(error.raw_reports, expected);
        assert!(session.poisoned);
        assert!(!error.began_write);
        assert_eq!(state.borrow().sent, expected_commands());
    }
}

#[test]
fn failed_queue_preparation_short_output_and_unknown_delivery_never_retry() {
    for phase in 1..=2 {
        for failure in 0..3 {
            let (mut session, state) = session(replies(FIRMWARE));
            match failure {
                0 if phase == 1 => state.borrow_mut().fail_prepare = Some(1),
                0 => state.borrow_mut().fail_queue_probe = Some(1),
                1 => state.borrow_mut().short_write = Some(phase),
                _ => state.borrow_mut().fail_write = Some(phase),
            }
            let error = session.read_raw_thresholds().unwrap_err();
            assert!(!error.began_write);
            assert!(session.poisoned);
            let before = state.borrow().sent.len();
            assert_eq!(before, phase - usize::from(failure == 0));
            assert!(session.read_raw_thresholds().is_err());
            assert_eq!(state.borrow().sent.len(), before);
        }
    }
}

#[test]
fn overall_fake_clock_deadline_rejects_even_complete_last_row_with_evidence() {
    let incoming = replies(FIRMWARE);
    let expected = incoming
        .iter()
        .map(|r| r.as_ref().unwrap().clone())
        .collect::<Vec<_>>();
    let (mut session, state) = session(incoming);
    state.borrow_mut().overrun_read = Some(12);
    let error = session.read_raw_thresholds().unwrap_err();
    assert!(error.message.contains("deadline"));
    assert_eq!(error.raw_reports, expected);
    assert!(session.poisoned);
    assert!(state.borrow().read_timeouts.iter().all(|n| *n <= 1000));
    assert_eq!(state.borrow().sent, expected_commands());
}

#[test]
fn stale_queued_frames_are_drained_and_post_stream_extras_fail_closed() {
    let (mut session, state) = session(replies(FIRMWARE));
    state
        .borrow_mut()
        .queue
        .extend(rows().into_iter().map(|r| r.to_vec()));
    session.read_raw_thresholds().unwrap();
    assert!(state.borrow().queue.is_empty());

    let (mut session, state) = self::session(replies(FIRMWARE));
    let extra = vec![0, 0xdf, 7];
    state.borrow_mut().queue_after_dc.push_back(extra.clone());
    let error = session.read_raw_thresholds().unwrap_err();
    assert_eq!(error.raw_reports.last(), Some(&extra));
    assert!(session.poisoned);
    assert_eq!(state.borrow().sent, expected_commands());
}

#[test]
fn offline_snapshot_requires_exact_metadata_and_complete_valid_rows() {
    let snapshot = RawThresholds::from_reports(identity(), FIRMWARE.into(), rows()).unwrap();
    assert_eq!(snapshot.values()[30], 13);
    assert_eq!(snapshot.raw_reports(), rows());
    assert!(
        RawThresholds::from_reports(identity(), FIRMWARE.into(), rows()[..10].to_vec()).is_err()
    );
    let mut wrong = identity();
    wrong.product_id = 0x512a;
    assert!(RawThresholds::from_reports(wrong, FIRMWARE.into(), rows()).is_err());
    assert!(RawThresholds::from_reports(identity(), "DEMO".into(), rows()).is_err());
}

#[test]
fn post_f9_queued_input_is_evidence_of_a_failed_transaction_and_prevents_dc() {
    for unexpected in [rows()[0].to_vec(), vec![0, 0xdc, 1], vec![0x91; 65]] {
        let (mut session, state) = session(replies(FIRMWARE));
        state
            .borrow_mut()
            .queue_after_f9
            .push_back(unexpected.clone());
        let error = session.read_raw_thresholds().unwrap_err();
        assert_eq!(error.raw_reports, vec![version(FIRMWARE), unexpected]);
        assert_eq!(state.borrow().sent, vec![expected_commands()[0].clone()]);
        assert!(state.borrow().sleeps.is_empty(), "DC was never attempted");
        assert!(session.poisoned);
        assert!(!error.began_write);
    }
}

#[test]
fn uncertain_dc_delivery_passively_captures_remaining_reports_while_retaining_path_lock() {
    for short in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = "fake-raw-threshold-native-path";
        let lock = crate::transport::acquire_lock(directory.path(), path).unwrap();
        let state = Rc::new(RefCell::new(State::default()));
        {
            let mut state = state.borrow_mut();
            state.lock_probe = Some((directory.path().into(), path.into()));
            state.capture_remaining = true;
            if short {
                state.short_write = Some(2);
            } else {
                state.fail_write = Some(2);
            }
        }
        let mut session = Session::from_transport(
            Box::new(Fake {
                state: state.clone(),
                incoming: replies(FIRMWARE).into(),
                _lock: Some(lock),
            }),
            identity(),
        );
        let error = session.read_raw_thresholds().unwrap_err();
        let mut expected = vec![version(FIRMWARE)];
        expected.extend(rows().into_iter().map(|r| r.to_vec()));
        assert_eq!(error.raw_reports, expected);
        assert!(error.message.contains("restoration not confirmed"));
        assert!(!error.began_write);
        assert!(session.poisoned);
        {
            let state = state.borrow();
            assert_eq!(state.sent, expected_commands());
            assert_eq!(state.sleeps.iter().sum::<u64>(), 2000);
            assert!(state.held_lock_observations.iter().all(|held| *held));
            assert!(!state.held_lock_observations.is_empty());
            assert!(state.dropped_at.is_none());
        }
        drop(session);
        assert_eq!(state.borrow().dropped_at, Some(2000));
        assert!(crate::transport::acquire_lock(directory.path(), path).is_ok());
    }
}

#[test]
fn malformed_short_timeout_extra_and_io_failed_dc_responses_all_settle_without_output() {
    for failure in 0..5 {
        let mut incoming = replies(FIRMWARE);
        match failure {
            0 => incoming[1].as_mut().unwrap()[3] = 2,
            1 => incoming[1] = Ok(vec![0, 0xdc, 1]),
            2 => incoming[1] = Ok(vec![]),
            3 => incoming[1] = Err("read disconnected".into()),
            _ => {}
        }
        let expected = incoming
            .iter()
            .filter_map(|r| r.clone().ok())
            .filter(|r| !r.is_empty())
            .collect::<Vec<_>>();
        let (mut session, state) = session(incoming);
        state.borrow_mut().capture_remaining = true;
        if failure == 4 {
            state
                .borrow_mut()
                .queue_after_dc
                .push_back(vec![0, 0xdf, 0x91]);
        }
        let error = session.read_raw_thresholds().unwrap_err();
        if failure < 4 {
            assert_eq!(error.raw_reports, expected);
        }
        assert!(error.message.contains("restoration not confirmed"));
        assert_eq!(state.borrow().sleeps.iter().sum::<u64>(), 2000);
        assert_eq!(state.borrow().sent, expected_commands());
        assert!(session.read_raw_thresholds().is_err());
        assert_eq!(state.borrow().sent, expected_commands());
    }
}

#[test]
fn passive_settle_has_attempt_and_report_limits_even_if_clock_never_advances() {
    let (mut session, state) = session(replies(FIRMWARE));
    {
        let mut state = state.borrow_mut();
        state.fail_write = Some(2);
        state.freeze_clock = true;
        state
            .queue_after_dc
            .extend((0..1000).map(|_| vec![0, 0xdc, 0x91]));
    }
    let error = session.read_raw_thresholds().unwrap_err();
    assert!(error.message.contains("restoration not confirmed"));
    assert_eq!(
        error.raw_reports.len(),
        17,
        "F9 plus at most 16 passive reports"
    );
    let state = state.borrow();
    assert_eq!(state.time, 0);
    assert!(state.sleeps.len() <= 40);
    assert_eq!(state.sleeps.iter().sum::<u64>(), 2000);
    assert!(
        state.queue_probes <= 41,
        "one pre-DC probe plus at most forty passive attempts"
    );
    assert_eq!(state.sent, expected_commands());
}

#[test]
fn post_dc_passive_queue_failure_keeps_original_error_and_still_holds_bounded_wait() {
    let (mut session, state) = session(replies(FIRMWARE));
    {
        let mut state = state.borrow_mut();
        state.fail_write = Some(2);
        state.fail_queue_probe = Some(2);
    }
    let error = session.read_raw_thresholds().unwrap_err();
    assert!(
        error
            .message
            .starts_with("write disconnected; delivery unknown")
    );
    assert!(error.message.contains("queued input disconnected"));
    assert!(error.message.contains("restoration not confirmed"));
    assert_eq!(
        state.borrow().queue_probes,
        2,
        "stop probing unusable transport"
    );
    assert_eq!(state.borrow().sleeps.iter().sum::<u64>(), 2000);
    assert_eq!(state.borrow().sent, expected_commands());
}
