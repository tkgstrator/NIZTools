//! Event-driven single-queue regressions: replies exist only after the matching
//! output, and production-equivalent preparation can drain that same queue.
use crate::transport::Transport;
use crate::{CalibrationPhase, Session, UnlockOutcome};
use niz_core::{Profile, command};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};
#[derive(Default)]
struct Events {
    queue: VecDeque<Vec<u8>>,
    sent: Vec<Vec<u8>>,
    drained: Vec<Vec<u8>>,
    inject_after: Option<u8>,
    inject_packet: Option<Vec<u8>>,
}
struct Device {
    profile: Profile,
    events: Rc<RefCell<Events>>,
    time: u64,
}
impl Transport for Device {
    fn prepare(&mut self) -> std::result::Result<(), String> {
        for _ in 0..=niz_core::MAX_REPORTS {
            let mut bytes = [0; 65];
            let n = self.read(&mut bytes, 0)?;
            if n == 0 {
                return Ok(());
            }
            self.events.borrow_mut().drained.push(bytes[..n].to_vec());
        }
        Err("native-equivalent drain exceeded bound".into())
    }
    fn write(&mut self, bytes: &[u8]) -> std::result::Result<usize, String> {
        let mut state = self.events.borrow_mut();
        state.sent.push(bytes.to_vec());
        match bytes[2] {
            0xf9 => {
                let mut response = command(0xf9);
                response[2..2 + self.profile.firmware.len()]
                    .copy_from_slice(self.profile.firmware.as_bytes());
                state.queue.push_back(response.to_vec());
            }
            0xf2 => {
                state
                    .queue
                    .extend(self.profile.reports().iter().map(|r| r.to_vec()));
                state.queue.push_back(vec![0xf6; 64]);
            }
            0xe2 => {
                for part in [0x37; 198].chunks(61) {
                    let mut response = command(0xe0);
                    response[2] = part.len() as u8;
                    response[3..3 + part.len()].copy_from_slice(part);
                    state.queue.push_back(response.to_vec());
                }
                state.queue.push_back(command(0xe6).to_vec());
            }
            0xdb => state.queue.push_back(command(0xda).to_vec()),
            0xdd => state.queue.push_back(command(0xde).to_vec()),
            _ => {}
        }
        if state.inject_after == Some(bytes[2]) {
            state.inject_after = None;
            let mut stale = command(0xda);
            stale[63] = 0xa5;
            let packet = state.inject_packet.take().unwrap_or_else(|| stale.to_vec());
            state.queue.push_back(packet);
        }
        Ok(bytes.len())
    }
    fn read(&mut self, bytes: &mut [u8], timeout: i32) -> std::result::Result<usize, String> {
        if let Some(response) = self.events.borrow_mut().queue.pop_front() {
            bytes[..response.len()].copy_from_slice(&response);
            Ok(response.len())
        } else {
            self.time += timeout as u64;
            Ok(0)
        }
    }
    fn now_ms(&self) -> u64 {
        self.time
    }
    fn sleep_ms(&mut self, n: u64) {
        self.time += n;
    }
}
fn setup() -> (Session, Rc<RefCell<Events>>) {
    let profile = Profile::from_json(include_str!(
        "../../../docs/validation/fixtures/atom66-xrgb-ble-v1.0.48-settings.json"
    ))
    .unwrap();
    setup_profile(profile)
}
fn setup_profile(profile: Profile) -> (Session, Rc<RefCell<Events>>) {
    let events = Rc::new(RefCell::new(Events::default()));
    (
        Session::from_transport(
            Box::new(Device {
                profile: profile.clone(),
                events: events.clone(),
                time: 0,
            }),
            profile.identity,
        ),
        events,
    )
}
#[test]
fn preflight_single_queue_never_drains_stale_input_or_dispatches_calibration() {
    for stale in [vec![0, 0xda, 0xda], command(0xde).to_vec(), {
        let mut r = command(0xda);
        r[63] = 0xa5;
        r.to_vec()
    }] {
        for after in [None, Some(0xf9), Some(0xf2), Some(0xe2)] {
            let (mut session, events) = setup();
            let baseline = session.read_baseline().unwrap();
            if let Some(op) = after {
                events.borrow_mut().inject_after = Some(op);
                events.borrow_mut().inject_packet = Some(stale.clone());
            } else {
                events.borrow_mut().queue.push_back(stale.to_vec());
            }
            let dir = tempfile::tempdir().unwrap();
            let result = session.begin_calibration(&baseline, dir.path());
            assert!(
                result.is_err(),
                "queued notification was silently discarded after {after:?}"
            );
            let error = result.unwrap_err();
            let status = error.calibration.unwrap();
            assert_eq!(status.phase, CalibrationPhase::Failed);
            assert_eq!(status.unlock, UnlockOutcome::NotAttempted);
            assert!(events.borrow().drained.is_empty());
            assert!(
                !events
                    .borrow()
                    .sent
                    .iter()
                    .any(|r| matches!(r[2], 0xd9 | 0xdb | 0xdd))
            );
            assert!(status.trace.iter().any(|r| r.bytes == stale));
            assert_eq!(error.raw_reports, vec![stale.to_vec()]);
        }
    }
}
#[test]
fn verifying_single_queue_never_drains_stale_input_between_transactions() {
    for after in [None, Some(0xf9), Some(0xf2)] {
        let (mut session, events) = setup();
        let baseline = session.read_baseline().unwrap();
        let dir = tempfile::tempdir().unwrap();
        session.begin_calibration(&baseline, dir.path()).unwrap();
        session.calibrate_held_keys().unwrap();
        let mut stale = command(0xda);
        stale[63] = 0xa5;
        if let Some(op) = after {
            events.borrow_mut().inject_after = Some(op)
        } else {
            events.borrow_mut().queue.push_back(stale.to_vec());
        }
        let result = session.finish_calibration();
        assert!(
            result.is_err(),
            "verification discarded stale input after {after:?}"
        );
        let error = result.unwrap_err();
        let status = error.calibration.unwrap();
        assert_eq!(status.unlock, UnlockOutcome::Sent);
        assert_eq!(status.failure_phase, Some(CalibrationPhase::Verifying));
        assert!(events.borrow().drained.is_empty());
        assert!(status.trace.iter().any(|r| r.bytes == stale));
        assert_eq!(
            events
                .borrow()
                .sent
                .iter()
                .filter(|r| r[2] == 0xd9 && r[3] == 1)
                .count(),
            1
        );
    }
}
#[test]
fn event_driven_success_consumes_only_replies_generated_after_commands() {
    let (mut session, events) = setup();
    let baseline = session.read_baseline().unwrap();
    let dir = tempfile::tempdir().unwrap();
    session.begin_calibration(&baseline, dir.path()).unwrap();
    assert!(events.borrow().queue.is_empty());
    session.calibrate_held_keys().unwrap();
    assert!(events.borrow().queue.is_empty());
    let result = session.finish_calibration().unwrap();
    assert_eq!(result.status.phase, CalibrationPhase::Complete);
    assert!(events.borrow().drained.is_empty());
    assert!(events.borrow().queue.is_empty());
}
#[test]
fn bounded_trial_noops_emit_no_postflight_or_lighting_traffic() {
    for mode in 0..3 {
        let (mut session, events) = if mode == 2 {
            let mut p = Profile::from_json(include_str!(
                "../../../docs/validation/fixtures/atom66-xrgb-ble-v1.0.48-settings.json"
            ))
            .unwrap();
            p.set_definition(1, &p.definition(0).unwrap()).unwrap();
            setup_profile(p)
        } else {
            setup()
        };
        let baseline = session.read_baseline().unwrap();
        let count = events.borrow().sent.len();
        let dir = tempfile::tempdir().unwrap();
        let result = if mode > 0 {
            session
                .trial_swap(&baseline, 0, usize::from(mode == 2), dir.path())
                .unwrap()
        } else {
            session
                .trial_single_key(
                    &baseline,
                    0,
                    baseline.profile().definition(0).unwrap().keys[0],
                    dir.path(),
                )
                .unwrap()
        };
        assert_eq!(result.outcome, crate::ApplyOutcome::NoChange);
        assert!(result.snapshot.is_none());
        assert!(result.backup_path.exists());
        assert_eq!(events.borrow().sent.len(), count);
    }
}
#[test]
fn single_queue_positive_control_mirrors_native_read_only_drain() {
    let (mut session, events) = setup();
    let stale = command(0xda).to_vec();
    events.borrow_mut().queue.push_back(stale.clone());
    session.read_baseline().unwrap();
    assert_eq!(events.borrow().drained, vec![stale]);
    assert_eq!(
        events
            .borrow()
            .sent
            .iter()
            .map(|r| r[2])
            .collect::<Vec<_>>(),
        vec![0xf9, 0xf2]
    );
}
