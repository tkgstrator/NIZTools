use super::*;
use crate::transport::Transport;
use niz_core::{DeviceIdentity, Profile, command};
use std::{cell::RefCell, collections::VecDeque, path::Path, rc::Rc};
#[derive(Default)]
struct State {
    sent: Vec<Vec<u8>>,
    time: u64,
    fail_send: Option<usize>,
    short_send: Option<usize>,
    cancel_on_send: Option<usize>,
    cancel_at: Option<u64>,
    cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    disconnected: bool,
    unsolicited: VecDeque<Vec<u8>>,
    unsolicited_after_lock: Option<Vec<u8>>,
}
impl State {
    fn check_cancel(&self) {
        if self.cancel_at.is_some_and(|at| self.time >= at)
            && let Some(flag) = &self.cancel_flag
        {
            flag.store(true, std::sync::atomic::Ordering::Release);
        }
    }
}
struct Fake {
    state: Rc<RefCell<State>>,
    incoming: VecDeque<(u64, std::result::Result<Vec<u8>, String>)>,
}
impl Transport for Fake {
    fn queued_input(&mut self) -> std::result::Result<Option<Vec<u8>>, String> {
        Ok(self.state.borrow_mut().unsolicited.pop_front())
    }
    fn connected(&self) -> bool {
        !self.state.borrow().disconnected
    }
    fn write(&mut self, b: &[u8]) -> std::result::Result<usize, String> {
        let mut s = self.state.borrow_mut();
        if s.fail_send == Some(s.sent.len()) {
            return Err("disconnected write".into());
        }
        let index = s.sent.len();
        s.sent.push(b.to_vec());
        if b[2] == 0xd9
            && b[3] == 0
            && let Some(stale) = s.unsolicited_after_lock.take()
        {
            s.unsolicited.push_back(stale);
        }
        if s.cancel_on_send == Some(index)
            && let Some(flag) = &s.cancel_flag
        {
            flag.store(true, std::sync::atomic::Ordering::Release);
        }
        if s.short_send == Some(index) {
            Ok(64)
        } else {
            Ok(b.len())
        }
    }
    fn read(&mut self, b: &mut [u8], timeout: i32) -> std::result::Result<usize, String> {
        match self.incoming.pop_front() {
            Some((delay, r)) => {
                if delay > timeout as u64 {
                    self.state.borrow_mut().time += timeout as u64;
                    self.state.borrow().check_cancel();
                    self.incoming.push_front((delay - timeout as u64, r));
                    return Ok(0);
                }
                self.state.borrow_mut().time += delay;
                self.state.borrow().check_cancel();
                let r = match r {
                    Ok(r) => r,
                    Err(e) => {
                        self.state.borrow_mut().disconnected = true;
                        return Err(e);
                    }
                };
                b[..r.len()].copy_from_slice(&r);
                Ok(r.len())
            }
            None => {
                self.state.borrow_mut().time += timeout as u64;
                self.state.borrow().check_cancel();
                Ok(0)
            }
        }
    }
    fn now_ms(&self) -> u64 {
        self.state.borrow().time
    }
    fn sleep_ms(&mut self, n: u64) {
        self.state.borrow_mut().time += n;
    }
}
fn ident() -> DeviceIdentity {
    DeviceIdentity {
        vendor_id: 0x0483,
        product_id: 0x502a,
        usage_page: 0x8c,
        usage: 1,
    }
}
fn profile() -> Profile {
    let mut p = Profile::demo();
    p.firmware = "66EC(XRGB)BLe;V1.0.48;V1.0;".into();
    p.identity = ident();
    p
}
fn version() -> Vec<u8> {
    let mut r = command(0xf9);
    let s = b"66EC(XRGB)BLe;V1.0.48;V1.0;";
    r[2..2 + s.len()].copy_from_slice(s);
    r.to_vec()
}
fn stream(p: &Profile) -> Vec<Vec<u8>> {
    let mut vr = command(0xf9);
    vr[2..2 + p.firmware.len()].copy_from_slice(p.firmware.as_bytes());
    let mut r = vec![vr.to_vec()];
    r.extend(p.reports().iter().map(|r| r.to_vec()));
    r.push(vec![0xf6; 64]);
    r
}
fn session(replies: Vec<Vec<u8>>) -> (Session, Rc<RefCell<State>>) {
    let state = Rc::new(RefCell::new(State::default()));
    let f = Fake {
        state: state.clone(),
        incoming: replies.into_iter().map(|r| (0, Ok(r))).collect(),
    };
    (Session::from_transport(Box::new(f), ident()), state)
}
#[test]
fn read_preserves_leading_zero_and_frames_65_bytes() {
    let p = profile();
    let (mut s, state) = session(stream(&p));
    let actual = s.read_profile().unwrap();
    assert_eq!(actual.reports(), p.reports());
    let sent = &state.borrow().sent;
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[0].len(), 65);
    assert_eq!(&sent[0][1..], &command(0xf9));
    assert_eq!(&sent[1][1..], &command(0xf2));
}
#[test]
fn active_calibration_rejects_raw_getter_without_any_f9_or_dc() {
    let p = live_fixture();
    let (mut session, state) = session(calibration_replies(&p));
    let baseline = session.read_baseline().unwrap();
    let directory = tempfile::tempdir().unwrap();
    session
        .begin_calibration(&baseline, directory.path())
        .unwrap();
    let sent_before = state.borrow().sent.clone();
    let error = session.read_raw_thresholds().unwrap_err();
    assert!(error.message.contains("calibration"));
    assert_eq!(state.borrow().sent, sent_before);
}
#[test]
fn short_read_poisoned_and_raw_retained() {
    let (mut s, state) = session(vec![vec![0, 0xf9, 1]]);
    let e = s.version().unwrap_err();
    assert_eq!(e.raw_reports, vec![vec![0, 0xf9, 1]]);
    assert!(s.version().is_err());
    assert_eq!(state.borrow().sent.len(), 1);
}
#[test]
fn missing_end_poisoned() {
    let mut r = stream(&profile());
    r.pop();
    let (mut s, _) = session(r);
    let e = s.read_profile().unwrap_err();
    assert_eq!(e.raw_reports.len(), 198);
    assert!(s.read_profile().is_err());
}
#[test]
fn wrong_version_response_rejected() {
    for r in [vec![1; 64], command(0xf9).to_vec()] {
        let (mut s, _) = session(vec![r]);
        assert!(s.version().is_err());
    }
}
#[test]
fn unsupported_model_rejected() {
    let mut r = command(0xf9);
    r[2..7].copy_from_slice(b"68ECX");
    let (mut s, _) = session(vec![r.to_vec()]);
    assert!(s.read_profile().is_err());
}
#[test]
fn unknown_terminator_preserves_diagnostics() {
    let (mut s, _) = session(vec![version(), command(0xf6).to_vec()]);
    let e = s.read_profile().unwrap_err();
    assert_eq!(e.raw_reports, vec![command(0xf6).to_vec()]);
}
#[test]
fn durable_backup_roundtrips_without_overwrite() {
    let d = tempfile::tempdir().unwrap();
    let p = profile();
    let a = save_backup(&p, d.path()).unwrap();
    let b = save_backup(&p, d.path()).unwrap();
    assert_ne!(a, b);
    assert_eq!(load_profile(&a).unwrap().reports(), p.reports());
}
#[test]
fn normal_apply_unqualified_sends_no_write() {
    let p = profile();
    let (mut s, state) = session(stream(&p));
    let b = s.read_baseline().unwrap();
    let mut desired = p.clone();
    desired
        .set_definition(
            0,
            &niz_core::Definition {
                kind: 0,
                keys: vec![28],
                interval: 0,
                cycles: 1,
                custom_delay: false,
                delays: vec![],
            },
        )
        .unwrap();
    let e = s.apply(&b, &desired, Path::new("/unused")).unwrap_err();
    assert!(e.message.contains("unqualified"));
    assert!(!state.borrow().sent.iter().any(|r| r[2] == 0xf1));
}
#[test]
fn foreign_baseline_rejected() {
    let p = profile();
    let (mut a, _) = session(stream(&p));
    let b = a.read_baseline().unwrap();
    let (mut s, state) = session(stream(&p));
    s.read_baseline().unwrap();
    assert!(s.trial_single_key(&b, 0, 28, Path::new("/unused")).is_err());
    assert_eq!(state.borrow().sent.len(), 2);
}
fn desired(p: &Profile, code: u8) -> Profile {
    let mut d = p.clone();
    d.set_definition(
        0,
        &niz_core::Definition {
            kind: 0,
            keys: vec![code],
            interval: 0,
            cycles: 1,
            custom_delay: false,
            delays: vec![],
        },
    )
    .unwrap();
    d
}
fn repeated_stream(profiles: &[&Profile]) -> Vec<Vec<u8>> {
    let mut replies = profiles.iter().flat_map(|p| stream(p)).collect::<Vec<_>>();
    if profiles.len() > 1 {
        replies.extend(rgb_stream(&rgb()));
    }
    replies
}
fn readback(p: &Profile) -> Vec<Vec<u8>> {
    let mut r = stream(p);
    r.extend(rgb_stream(&rgb()));
    r
}

fn f1_count(state: &Rc<RefCell<State>>) -> usize {
    state.borrow().sent.iter().filter(|r| r[2] == 0xf1).count()
}
#[test]
fn check_apply_local_gate_emits_no_traffic() {
    let p = profile();
    let (mut s, state) = session(stream(&p));
    let b = s.read_baseline().unwrap();
    let sent = state.borrow().sent.len();
    assert!(
        s.check_apply(&b, &desired(&p, 28))
            .unwrap_err()
            .message
            .contains("unqualified")
    );
    assert_eq!(state.borrow().sent.len(), sent);
}
#[test]
fn old_baseline_after_new_capture_rejected() {
    let p = profile();
    let (mut s, state) = session(repeated_stream(&[&p, &p]));
    let old = s.read_baseline().unwrap();
    s.read_baseline().unwrap();
    assert!(
        s.trial_single_key(&old, 0, 28, Path::new("/unused"))
            .is_err()
    );
    assert_eq!(state.borrow().sent.len(), 4);
}
#[test]
fn external_configuration_change_blocks_f1() {
    let p = profile();
    let external = desired(&p, 29);
    let (mut s, state) = session(repeated_stream(&[&p, &external]));
    let b = s.read_baseline().unwrap();
    let d = tempfile::tempdir().unwrap();
    let e = s.trial_single_key(&b, 0, 28, d.path()).unwrap_err();
    assert!(e.message.contains("external"));
    assert_eq!(f1_count(&state), 0);
    assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 0);
    assert!(s.trial_single_key(&b, 0, 28, d.path()).is_err());
}
#[test]
fn firmware_change_blocks_f1() {
    let p = profile();
    let mut changed = stream(&p);
    let mut v = version();
    v[15] = b'9';
    changed[0] = v;
    let mut replies = stream(&p);
    replies.extend(changed);
    let (mut s, state) = session(replies);
    let b = s.read_baseline().unwrap();
    let d = tempfile::tempdir().unwrap();
    assert!(s.trial_single_key(&b, 0, 28, d.path()).is_err());
    assert_eq!(f1_count(&state), 0);
}
#[test]
fn successful_trial_framing_backup_sequence_and_readback() {
    let p = profile();
    let target = desired(&p, 28);
    let mut replies = repeated_stream(&[&p, &p]);
    replies.extend(readback(&target));
    let (mut s, state) = session(replies);
    let b = s.read_baseline().unwrap();
    let d = tempfile::tempdir().unwrap();
    let result = s.trial_single_key(&b, 0, 28, d.path()).unwrap();
    assert_eq!(result.profile.records, target.records);
    assert_eq!(
        load_profile(&result.backup_path).unwrap().records,
        p.records
    );
    let st = state.borrow();
    assert!(st.sent.iter().all(|r| r.len() == 65 && r[0] == 0));
    let start = st.sent.iter().position(|r| r[2] == 0xf1).unwrap();
    assert_eq!(
        st.sent[start],
        std::iter::once(0).chain(command(0xf1)).collect::<Vec<_>>()
    );
    for (i, r) in target.reports().iter().enumerate() {
        assert_eq!(&st.sent[start + 1 + i][1..], r);
    }
    assert_eq!(st.sent[start + 199][1], 0);
    assert!(st.sent[start + 199][2..].iter().all(|b| *b == 0xf6));
    assert_eq!(&st.sent[start + 200][1..], &command(0xf9));
    assert_eq!(&st.sent[start + 201][1..], &command(0xf2));
    assert_eq!(&st.sent[start + 202][1..], &command(0xe2));
    assert_eq!(st.time, 198 * 3 + 300);
}
#[test]
fn readback_unknown_byte_change_detected_no_retry() {
    let p = profile();
    let mut target = desired(&p, 28);
    target.records[9][0][63] ^= 1;
    let mut replies = repeated_stream(&[&p, &p]);
    replies.extend(readback(&target));
    let (mut s, state) = session(replies);
    let b = s.read_baseline().unwrap();
    let d = tempfile::tempdir().unwrap();
    let e = s.trial_single_key(&b, 0, 28, d.path()).unwrap_err();
    assert!(e.began_write);
    assert_eq!(
        e.raw_reports,
        target
            .reports()
            .iter()
            .map(|r| r.to_vec())
            .collect::<Vec<_>>()
    );
    assert!(e.backup_path.unwrap().exists());
    assert!(e.message.contains("opaque"));
    assert_eq!(f1_count(&state), 1);
    assert!(s.version().is_err());
}
#[test]
fn send_failure_after_f1_poisoned_no_retry() {
    let p = profile();
    let (mut s, state) = session(repeated_stream(&[&p, &p]));
    let b = s.read_baseline().unwrap();
    state.borrow_mut().fail_send = Some(6);
    let d = tempfile::tempdir().unwrap();
    let e = s.trial_single_key(&b, 0, 28, d.path()).unwrap_err();
    assert!(e.began_write);
    assert!(e.backup_path.is_some());
    assert_eq!(f1_count(&state), 1);
    assert!(s.read_profile().is_err());
}
#[test]
fn backup_io_failure_never_sends_f1() {
    let p = profile();
    let (mut s, state) = session(repeated_stream(&[&p, &p]));
    let b = s.read_baseline().unwrap();
    let d = tempfile::tempdir().unwrap();
    let file = d.path().join("file");
    std::fs::write(&file, b"not a directory").unwrap();
    assert!(s.trial_single_key(&b, 0, 28, &file).is_err());
    assert_eq!(f1_count(&state), 0);
}
#[test]
fn backup_reparse_failure_never_sends_f1() {
    let p = profile();
    let (mut s, state) = session(repeated_stream(&[&p, &p]));
    s.backup_for_test(|_, dir| Ok(dir.join("missing.json")));
    let b = s.read_baseline().unwrap();
    let d = tempfile::tempdir().unwrap();
    assert!(s.trial_single_key(&b, 0, 28, d.path()).is_err());
    assert_eq!(f1_count(&state), 0);
}
#[test]
fn trial_rejects_fn_multikey_macro_extra_group_and_reserved_code() {
    for case in 0..6 {
        let mut p = profile();
        let (index, code) = match case {
            0 => (54, 28),
            1 => (0, 156),
            2 => {
                p.set_definition(
                    0,
                    &niz_core::Definition {
                        kind: 0,
                        keys: vec![28, 29],
                        interval: 0,
                        cycles: 1,
                        custom_delay: false,
                        delays: vec![],
                    },
                )
                .unwrap();
                (0, 28)
            }
            3 => {
                p.set_definition(
                    0,
                    &niz_core::Definition {
                        kind: 2,
                        keys: vec![28],
                        interval: 0,
                        cycles: 1,
                        custom_delay: false,
                        delays: vec![],
                    },
                )
                .unwrap();
                (0, 28)
            }
            4 => (198, 28),
            _ => (0, 0),
        };
        let (mut s, state) = session(stream(&p));
        let b = s.read_baseline().unwrap();
        assert!(
            s.trial_single_key(&b, index, code, Path::new("/unused"))
                .is_err()
        );
        assert_eq!(state.borrow().sent.len(), 2);
    }
}
#[test]
fn noop_never_writes_or_requeries_device() {
    let p = profile();
    let (mut s, state) = session(stream(&p));
    let b = s.read_baseline().unwrap();
    let d = tempfile::tempdir().unwrap();
    let result = s.apply(&b, &p, d.path()).unwrap();
    assert!(result.backup_path.exists());
    assert_eq!(state.borrow().sent.len(), 2);
}
#[test]
fn single_qualification_rejects_changed_combo_and_macro() {
    use crate::session::{Action, Qualification};
    let p = profile();
    for kind in [0, 1, 2] {
        let (mut s, state) = session(stream(&p));
        let b = s.read_baseline().unwrap();
        let mut target = p.clone();
        target
            .set_definition(
                0,
                &niz_core::Definition {
                    kind,
                    keys: vec![28, 29],
                    interval: 0,
                    cycles: 1,
                    custom_delay: false,
                    delays: vec![],
                },
            )
            .unwrap();
        let q = [Qualification {
            firmware: "66EC(XRGB)BLe;V1.0.48;V1.0;",
            product: 0x502a,
            groups: 3,
            actions: &[Action::BasicSingle],
        }];
        assert!(
            s.apply_impl(&b, &target, Path::new("/unused"), &q, false)
                .is_err()
        );
        assert_eq!(state.borrow().sent.len(), 2);
    }
}
#[test]
fn restore_a_over_live_b_preserves_unchanged_macro() {
    use crate::session::{Action, Qualification};
    let mut a = profile();
    a.set_definition(
        4,
        &niz_core::Definition {
            kind: 2,
            keys: vec![28, 29],
            interval: 100,
            cycles: 2,
            custom_delay: false,
            delays: vec![],
        },
    )
    .unwrap();
    let b = desired(&a, 29);
    let mut replies = repeated_stream(&[&b, &b]);
    replies.extend(readback(&a));
    let (mut s, state) = session(replies);
    let baseline = s.read_baseline().unwrap();
    let d = tempfile::tempdir().unwrap();
    let q = [Qualification {
        firmware: "66EC(XRGB)BLe;V1.0.48;V1.0;",
        product: 0x502a,
        groups: 3,
        actions: &[Action::BasicSingle],
    }];
    let result = s.apply_impl(&baseline, &a, d.path(), &q, false).unwrap();
    assert_eq!(result.profile.records, a.records);
    assert_eq!(
        load_profile(&result.backup_path).unwrap().records,
        b.records
    );
    assert_eq!(f1_count(&state), 1);
}
#[test]
fn counter_chunks_little_endian_and_exact_length() {
    let mut v = command(0xf9);
    let s = b"66EC(S)BLe;V1.0.37;V1.0;";
    v[2..2 + s.len()].copy_from_slice(s);
    let mut replies = vec![v.to_vec()];
    let data = (0..66u32).flat_map(|n| n.to_le_bytes()).collect::<Vec<_>>();
    for part in data.chunks(61) {
        let mut r = command(0xe3);
        r[2] = part.len() as u8;
        r[3..3 + part.len()].copy_from_slice(part);
        replies.push(r.to_vec());
    }
    replies.push(command(0xe6).to_vec());
    let (mut s, state) = session(replies);
    assert_eq!(s.read_counters().unwrap(), (0..66).collect::<Vec<_>>());
    assert_eq!(state.borrow().sent.len(), 2);
}
#[test]
fn counter_overflow_wrong_header_short_end_and_missing_end() {
    for case in 0..4 {
        let mut r = command(0xe3);
        r[2] = match case {
            0 => 62,
            1 => 1,
            _ => 61,
        };
        if case == 1 {
            r[0] = 1;
        }
        let mut replies = vec![version(), r.to_vec()];
        if case == 2 {
            replies.push(command(0xe6).to_vec())
        }
        let (mut s, _) = session(replies);
        assert!(s.read_counters().is_err());
        assert!(s.version().is_err());
    }
}
#[test]
fn delayed_disconnected_and_send_failure_poison() {
    for case in 0..3 {
        let state = Rc::new(RefCell::new(State::default()));
        if case == 2 {
            state.borrow_mut().fail_send = Some(0);
        }
        let r = if case == 1 {
            Err("device disconnected".into())
        } else {
            Ok(version())
        };
        let f = Fake {
            state: state.clone(),
            incoming: vec![(if case == 0 { 1001 } else { 0 }, r)].into(),
        };
        let mut s = Session::from_transport(Box::new(f), ident());
        assert!(s.version().is_err());
        let n = state.borrow().sent.len();
        assert!(s.version().is_err());
        assert_eq!(state.borrow().sent.len(), n);
    }
}
#[test]
fn descriptor_requires_exact_configuration_report_width_and_id() {
    let valid = [
        0x06, 0x8c, 0x00, 0x09, 0x01, 0xa1, 0x01, 0x75, 0x08, 0x95, 0x40, 0x81, 0x02, 0x91, 0x02,
        0xc0,
    ];
    assert!(crate::transport::validate_descriptor(&valid).is_ok());
    for case in 0..4 {
        let mut bad = valid.to_vec();
        match case {
            0 => bad[10] = 63,
            1 => {
                bad.splice(7..7, [0x85, 1]);
            }
            2 => bad[1] = 1,
            _ => {
                bad.pop();
            }
        }
        assert!(crate::transport::validate_descriptor(&bad).is_err());
    }
}
#[test]
fn stream_deadline_and_report_limit_are_bounded() {
    for case in 0..2 {
        let p = profile();
        let state = Rc::new(RefCell::new(State::default()));
        let mut incoming = vec![(0, Ok(version()))];
        if case == 0 {
            incoming.extend(
                p.reports()
                    .into_iter()
                    .take(31)
                    .map(|r| (1000, Ok(r.to_vec()))),
            );
        } else {
            incoming.extend((0..=niz_core::MAX_REPORTS).map(|_| (0, Ok(p.records[0][0].to_vec()))));
        }
        let f = Fake {
            state: state.clone(),
            incoming: incoming.into(),
        };
        let mut s = Session::from_transport(Box::new(f), ident());
        let e = s.read_profile().unwrap_err();
        assert!(
            e.message
                .contains(if case == 0 { "deadline" } else { "limit" }),
            "{e}"
        );
        assert!(e.raw_reports.len() <= niz_core::MAX_REPORTS + 1);
        assert_eq!(f1_count(&state), 0);
        assert!(s.version().is_err());
    }
}
#[test]
fn incoming_65_byte_report_not_silently_stripped() {
    let mut r = vec![0];
    r.extend(version());
    let (mut s, _) = session(vec![r.clone()]);
    let e = s.version().unwrap_err();
    assert_eq!(e.raw_reports, vec![r]);
}
struct ShortSend {
    sent: Rc<RefCell<usize>>,
}
impl Transport for ShortSend {
    fn write(&mut self, _: &[u8]) -> std::result::Result<usize, String> {
        *self.sent.borrow_mut() += 1;
        Ok(64)
    }
    fn read(&mut self, _: &mut [u8], _: i32) -> std::result::Result<usize, String> {
        panic!("short send must stop before reading")
    }
    fn now_ms(&self) -> u64 {
        0
    }
    fn sleep_ms(&mut self, _: u64) {}
}
#[test]
fn short_send_stops_without_retry_or_read() {
    let sent = Rc::new(RefCell::new(0));
    let mut s = Session::from_transport(Box::new(ShortSend { sent: sent.clone() }), ident());
    assert!(s.version().unwrap_err().message.contains("short HID write"));
    assert!(s.version().is_err());
    assert_eq!(*sent.borrow(), 1);
}
#[test]
fn trial_rejects_rgb_and_unidentified_s_suffix() {
    for v in [
        "66EC(RGB)BLe;V1.5.1;V1.0;",
        "66EC(S);V1.0;",
        "66EC(S)BLeFAKE;V1.0;",
    ] {
        let mut p = profile();
        p.firmware = v.into();
        let mut replies = stream(&p);
        let mut r = command(0xf9);
        r[2..2 + v.len()].copy_from_slice(v.as_bytes());
        replies[0] = r.to_vec();
        let (mut s, state) = session(replies);
        let baseline = s.read_baseline().unwrap();
        assert!(
            s.trial_single_key(&baseline, 0, 28, Path::new("/unused"))
                .is_err()
        );
        assert_eq!(f1_count(&state), 0);
        assert_eq!(state.borrow().sent.len(), 2);
    }
}
#[test]
fn descriptor_rejects_id0_fields_outside_selected_collection() {
    let config = [
        0x06, 0x8c, 0x00, 0x09, 0x01, 0xa1, 0x01, 0x75, 0x08, 0x95, 0x40, 0x81, 0x02, 0x91, 0x02,
        0xc0,
    ];
    let mut descriptor = config.to_vec();
    descriptor.extend([
        0x05, 0x01, 0x09, 0x02, 0xa1, 0x01, 0x75, 0x08, 0x95, 0x01, 0x81, 0x02, 0x91, 0x02, 0xc0,
    ]);
    assert!(crate::transport::validate_descriptor(&descriptor).is_err());
}
#[test]
fn descriptor_rejects_ambiguous_numbered_and_unnumbered_reports() {
    let mut descriptor = vec![
        0x06, 0x8c, 0x00, 0x09, 0x01, 0xa1, 0x01, 0x75, 0x08, 0x95, 0x40, 0x81, 0x02, 0x91, 0x02,
        0xc0,
    ];
    descriptor.extend([
        0x05, 0x01, 0x09, 0x02, 0xa1, 0x01, 0x85, 0x02, 0x75, 0x08, 0x95, 0x01, 0x81, 0x02, 0x91,
        0x02, 0xc0,
    ]);
    assert!(crate::transport::validate_descriptor(&descriptor).is_err());
}
#[test]
fn descriptor_rejects_unbounded_global_and_collection_stacks() {
    let config = [
        0x06, 0x8c, 0x00, 0x09, 0x01, 0xa1, 0x01, 0x75, 0x08, 0x95, 0x40, 0x81, 0x02, 0x91, 0x02,
        0xc0,
    ];
    let mut globals = vec![0xa4; 65];
    globals.extend([0xb4; 65]);
    globals.extend(config);
    assert!(crate::transport::validate_descriptor(&globals).is_err());
    let mut nested = vec![];
    for _ in 0..65 {
        nested.extend([0xa1, 1]);
    }
    nested.extend([0xc0; 65]);
    nested.extend(config);
    assert!(crate::transport::validate_descriptor(&nested).is_err());
}
#[test]
fn readback_failure_retains_received_order_instead_of_canonical_order() {
    let p = profile();
    let mut target = desired(&p, 28);
    target.records[9][0][63] = 0xa5;
    let mut actual = target.reports();
    actual.swap(0, 9);
    let mut replies = repeated_stream(&[&p, &p]);
    replies.push(version());
    replies.extend(actual.iter().map(|r| r.to_vec()));
    replies.push(vec![0xf6; 64]);
    let (mut session, state) = session(replies);
    let baseline = session.read_baseline().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let error = session
        .trial_single_key(&baseline, 0, 28, dir.path())
        .unwrap_err();
    assert_eq!(
        error.raw_reports,
        actual.iter().map(|r| r.to_vec()).collect::<Vec<_>>()
    );
    assert!(error.began_write);
    assert_eq!(f1_count(&state), 1);
}
#[test]
fn descriptor_bounds_reject_empty_oversized_and_zero_width_items() {
    assert!(crate::transport::validate_descriptor(&[]).is_err());
    assert!(crate::transport::validate_descriptor(&vec![0; 4097]).is_err());
    let config = [
        0x06, 0x8c, 0x00, 0x09, 0x01, 0xa1, 0x01, 0x75, 0x08, 0x95, 0x40, 0x81, 0x02, 0x91, 0x02,
        0xc0,
    ];
    for zero_at in [8, 10] {
        let mut invalid = config;
        invalid[zero_at] = 0;
        assert!(crate::transport::validate_descriptor(&invalid).is_err());
    }
}
fn live_fixture() -> Profile {
    Profile::from_json(include_str!(
        "../../../docs/validation/fixtures/atom66-xrgb-ble-v1.0.48-settings.json"
    ))
    .unwrap()
}
fn rgb() -> Vec<u8> {
    (0..198).map(|n| (n * 7 % 256) as u8).collect()
}
fn rgb_stream(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut reports = vec![];
    for bytes in bytes.chunks(61) {
        let mut r = command(0xe0);
        r[2] = bytes.len() as u8;
        r[3..3 + bytes.len()].copy_from_slice(bytes);
        if bytes.len() < 61 {
            r[63] = 0xa5;
        }
        reports.push(r.to_vec())
    }
    reports.push(command(0xe6).to_vec());
    reports
}
#[test]
fn lighting_exact_capture_and_framing() {
    let (mut session, state) = session({
        let mut replies = vec![version()];
        replies.extend(rgb_stream(&rgb()));
        replies
    });
    assert_eq!(session.read_lighting().unwrap(), rgb());
    assert_eq!(&state.borrow().sent[1][1..], &command(0xe2));
}
#[test]
fn exact_target_rejects_old_s_trial_before_traffic() {
    let mut p = profile();
    p.firmware = "66EC(S)BLe;V1.0.37;V1.0;".into();
    let (mut session, state) = session(stream(&p));
    let b = session.read_baseline().unwrap();
    assert!(
        session
            .trial_single_key(&b, 0, 28, Path::new("/unused"))
            .is_err()
    );
    assert_eq!(state.borrow().sent.len(), 2);
}
#[test]
fn snapshot_manifest_binds_config_and_lighting() {
    let dir = tempfile::tempdir().unwrap();
    let p = live_fixture();
    let paths = crate::backup::save_snapshot(&p, &rgb(), dir.path(), save_backup).unwrap();
    crate::backup::verify_snapshot(&paths, &p, &rgb()).unwrap();
    std::fs::write(&paths.lighting_path, vec![0; 198]).unwrap();
    assert!(crate::backup::verify_snapshot(&paths, &p, &rgb()).is_err());
}
fn notification(op: u8) -> Vec<u8> {
    let mut r = command(op);
    r[2] = op;
    r[63] = 0xaa;
    r.to_vec()
}
fn calibration_replies(p: &Profile) -> Vec<Vec<u8>> {
    let mut r = repeated_stream(&[p, p]);
    r.push(notification(0xda));
    r.push(notification(0xde));
    r.extend(stream(p));
    r.extend(rgb_stream(&rgb()));
    r
}
#[test]
fn calibration_owned_stages_and_unchanged_postflight() {
    let p = live_fixture();
    let (mut s, state) = session(calibration_replies(&p));
    let b = s.read_baseline().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let released = s.begin_calibration(&b, dir.path()).unwrap();
    assert_eq!(released.phase, CalibrationPhase::AwaitingHeldKeys);
    assert!(s.version().is_err());
    assert!(s.read_lighting().is_err());
    let held = s.calibrate_held_keys().unwrap();
    assert_eq!(held.phase, CalibrationPhase::AwaitingFinish);
    assert!(s.calibrate_held_keys().is_err());
    let result = s.finish_calibration().unwrap();
    assert_eq!(result.profile.records, p.records);
    assert_eq!(result.lighting, rgb());
    assert!(result.status.settings_verified && result.status.lighting_verified);
    assert_eq!(result.status.unlock, UnlockOutcome::Sent);
    assert!(s.trial_single_key(&b, 0, 28, dir.path()).is_err());
    let sent = &state.borrow().sent;
    assert!(
        sent.iter()
            .all(|r| matches!(r[2], 0xf9 | 0xf2 | 0xe2 | 0xd9 | 0xdb | 0xdd))
    );
    let changes = sent
        .iter()
        .filter(|r| [0xd9, 0xdb, 0xdd].contains(&r[2]))
        .collect::<Vec<_>>();
    assert_eq!(
        changes.iter().map(|r| (r[2], r[3])).collect::<Vec<_>>(),
        vec![(0xd9, 0), (0xdb, 0), (0xdd, 0), (0xd9, 1)]
    );
    assert!(changes.iter().all(|r| r[4..].iter().all(|b| *b == 0)));
}
#[test]
fn calibration_input_failure_unlocks_once_retains_error_and_trace() {
    let p = live_fixture();
    let mut replies = repeated_stream(&[&p, &p]);
    replies.push(notification(0xde));
    let (mut s, state) = session(replies);
    let b = s.read_baseline().unwrap();
    let d = tempfile::tempdir().unwrap();
    let error = s.begin_calibration(&b, d.path()).unwrap_err();
    let status = error.calibration.unwrap();
    assert_eq!(status.phase, CalibrationPhase::Failed);
    assert_eq!(status.unlock, UnlockOutcome::Sent);
    assert!(status.original_error.unwrap().contains("unexpected"));
    assert!(
        status
            .trace
            .iter()
            .any(|t| t.direction == TraceDirection::Input && t.bytes == notification(0xde))
    );
    let count = state.borrow().sent.len();
    assert!(s.cancel_calibration().is_err());
    assert_eq!(state.borrow().sent.len(), count);
}
#[test]
fn trial_swap_transmits_exact_opaque_fixture_and_preserves_rgb() {
    let p = live_fixture();
    let mut desired = p.clone();
    desired.swap_keys(0, 1).unwrap();
    let mut replies = repeated_stream(&[&p, &p]);
    replies.extend(readback(&desired));
    let (mut s, state) = session(replies);
    let b = s.read_baseline().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let result = s.trial_swap(&b, 0, 1, dir.path()).unwrap();
    assert_eq!(result.profile.records, desired.records);
    for i in [86, 87, 94, 152, 153, 160] {
        assert_eq!(result.profile.records[i], p.records[i]);
    }
    let sent = &state.borrow().sent;
    let start = sent.iter().position(|r| r[2] == 0xf1).unwrap();
    for (i, r) in desired.reports().iter().enumerate() {
        assert_eq!(&sent[start + i + 1][1..], r);
    }
    assert_eq!(sent.iter().filter(|r| r[2] == 0xe2).count(), 2);
    assert!(sent.iter().all(|r| !matches!(r[2], 0xe1 | 0xe0 | 0xe6)));
}
#[test]
fn trial_swap_rejects_fn_unknown_cross_layer_macro_and_bad_indices() {
    let p = live_fixture();
    for (from, to) in [(56, 1), (60, 1), (86, 87), (0, 66), (0, 198)] {
        let (mut s, state) = session(stream(&p));
        let baseline = s.read_baseline().unwrap();
        assert!(
            s.trial_swap(&baseline, from, to, Path::new("/unused"))
                .is_err()
        );
        assert_eq!(state.borrow().sent.len(), 2);
    }
}
#[test]
fn basic_qualification_does_not_authorize_fn_special_or_raw_padding() {
    use crate::session::{Action, Qualification};
    let p = live_fixture();
    for case in 0..3 {
        let mut target = p.clone();
        match case {
            0 => target
                .set_definition(
                    0,
                    &niz_core::Definition {
                        kind: 0,
                        keys: vec![156],
                        interval: 0,
                        cycles: 1,
                        custom_delay: false,
                        delays: vec![],
                    },
                )
                .unwrap(),
            1 => target
                .set_definition(
                    0,
                    &niz_core::Definition {
                        kind: 0,
                        keys: vec![88],
                        interval: 0,
                        cycles: 1,
                        custom_delay: false,
                        delays: vec![],
                    },
                )
                .unwrap(),
            _ => target.records[0][0][63] ^= 1,
        }
        let (mut s, state) = session(stream(&p));
        let b = s.read_baseline().unwrap();
        let q = [Qualification {
            firmware: crate::session::EXPERIMENTAL_FIRMWARE,
            product: 0x502a,
            groups: 3,
            actions: &[Action::BasicSingle],
        }];
        assert!(
            s.apply_impl(&b, &target, Path::new("/unused"), &q, false)
                .is_err()
        );
        assert_eq!(state.borrow().sent.len(), 2);
    }
}
#[test]
fn lighting_malformed_fault_matrix_blocks_changing_commands() {
    for case in 0..6 {
        let mut r = command(0xe0);
        r[2] = match case {
            0 => 0,
            1 => 62,
            _ => 61,
        };
        if case == 2 {
            r[0] = 1
        }
        let mut replies = vec![r.to_vec()];
        if case == 3 {
            replies.push(command(0xe6).to_vec());
        }
        if case == 4 {
            replies.extend((0..3).map(|_| r.to_vec()));
        }
        if case == 5 {
            replies = vec![vec![0, 0xe0, 1, 7]];
        }
        replies.insert(0, version());
        let (mut s, state) = session(replies);
        let error = s.read_lighting().unwrap_err();
        assert!(!error.raw_reports.is_empty());
        assert!(s.version().is_err());
        assert_eq!(state.borrow().sent.len(), 2);
    }
}
#[test]
fn calibration_every_changing_output_error_or_short_is_unknown_without_cleanup() {
    for fail_at in 5..=8 {
        for short in [false, true] {
            let p = live_fixture();
            let (mut s, state) = session(calibration_replies(&p));
            let b = s.read_baseline().unwrap();
            if short {
                state.borrow_mut().short_send = Some(fail_at)
            } else {
                state.borrow_mut().fail_send = Some(fail_at)
            }
            let d = tempfile::tempdir().unwrap();
            let error = match fail_at {
                5 | 6 => s.begin_calibration(&b, d.path()).unwrap_err(),
                7 => {
                    s.begin_calibration(&b, d.path()).unwrap();
                    s.calibrate_held_keys().unwrap_err()
                }
                _ => {
                    s.begin_calibration(&b, d.path()).unwrap();
                    s.calibrate_held_keys().unwrap();
                    s.finish_calibration().unwrap_err()
                }
            };
            let status = error.calibration.unwrap();
            assert_eq!(status.phase, CalibrationPhase::Failed);
            assert!(status.lock_may_have_been_sent);
            assert!(matches!(
                status.unlock,
                UnlockOutcome::NotAttempted | UnlockOutcome::Uncertain(_)
            ));
            let length = state.borrow().sent.len();
            assert!(s.cancel_calibration().is_err());
            assert!(s.version().is_err());
            assert_eq!(state.borrow().sent.len(), length);
            assert_eq!(
                state
                    .borrow()
                    .sent
                    .iter()
                    .filter(|r| r[2] == 0xd9 && r[3] == 1)
                    .count(),
                usize::from(short && fail_at == 8)
            );
        }
    }
}
#[test]
fn calibration_release_and_held_input_failure_matrix_cleanup_once() {
    for held in [false, true] {
        for reply in [
            None,
            Some(notification(0xf9)),
            Some(vec![0, 0xda]),
            Some(notification(if held { 0xda } else { 0xde })),
        ] {
            let p = live_fixture();
            let mut replies = repeated_stream(&[&p, &p]);
            if held {
                replies.push(notification(0xda));
            }
            if let Some(r) = reply {
                replies.push(r);
            }
            let (mut s, state) = session(replies);
            let b = s.read_baseline().unwrap();
            let d = tempfile::tempdir().unwrap();
            let error = if held {
                s.begin_calibration(&b, d.path()).unwrap();
                s.calibrate_held_keys().unwrap_err()
            } else {
                s.begin_calibration(&b, d.path()).unwrap_err()
            };
            let status = error.calibration.unwrap();
            assert_eq!(status.unlock, UnlockOutcome::Sent);
            assert_eq!(
                state
                    .borrow()
                    .sent
                    .iter()
                    .filter(|r| r[2] == 0xd9 && r[3] == 1)
                    .count(),
                1
            );
            assert!(s.cancel_calibration().is_err());
            assert!(s.calibrate_held_keys().is_err());
        }
    }
}
#[test]
fn calibration_success_after_idle_slices_retains_nonzero_reply_tail() {
    let p = live_fixture();
    let state = Rc::new(RefCell::new(State::default()));
    let mut incoming = repeated_stream(&[&p, &p])
        .into_iter()
        .map(|r| (0, Ok(r)))
        .collect::<VecDeque<_>>();
    incoming.push_back((3500, Ok(notification(0xda))));
    let mut s = Session::from_transport(
        Box::new(Fake {
            state: state.clone(),
            incoming,
        }),
        ident(),
    );
    let b = s.read_baseline().unwrap();
    let d = tempfile::tempdir().unwrap();
    let status = s.begin_calibration(&b, d.path()).unwrap();
    assert_eq!(state.borrow().time, 3500);
    assert!(
        status
            .trace
            .iter()
            .any(|t| t.bytes == notification(0xda) && t.timestamp_ms == 3500)
    );
    assert_eq!(status.phase, CalibrationPhase::AwaitingHeldKeys);
}
#[test]
fn cancellation_before_lock_after_lock_and_mid_receive_stops_dispatch() {
    for case in 0..3 {
        let p = live_fixture();
        let replies = if case == 2 {
            repeated_stream(&[&p, &p])
        } else {
            calibration_replies(&p)
        };
        let (mut s, state) = session(replies);
        let b = s.read_baseline().unwrap();
        state.borrow_mut().cancel_flag = Some(s.cancellation_flag());
        if case == 0 {
            s.cancellation_flag()
                .store(true, std::sync::atomic::Ordering::Release)
        } else if case == 1 {
            state.borrow_mut().cancel_on_send = Some(5)
        } else {
            state.borrow_mut().cancel_at = Some(1500)
        }
        let d = tempfile::tempdir().unwrap();
        let e = s.begin_calibration(&b, d.path()).unwrap_err();
        let status = e.calibration.unwrap();
        assert_eq!(status.phase, CalibrationPhase::Aborted);
        assert!(status.cancellation_requested);
        let sent = &state.borrow().sent;
        assert!(!sent.iter().any(|r| r[2] == 0xdd));
        if case == 0 {
            assert!(!sent.iter().any(|r| r[2] == 0xd9));
        } else {
            assert_eq!(status.unlock, UnlockOutcome::Sent);
        }
        if case == 1 {
            assert!(!sent.iter().any(|r| r[2] == 0xdb));
        }
        if case == 2 {
            assert_eq!(state.borrow().time, 1500);
        }
    }
}
#[test]
fn calibration_late_response_is_not_replayed_and_cleanup_failure_is_retained() {
    for cleanup_fail in [false, true] {
        let p = live_fixture();
        let state = Rc::new(RefCell::new(State::default()));
        let mut incoming = repeated_stream(&[&p, &p])
            .into_iter()
            .map(|r| (0, Ok(r)))
            .collect::<VecDeque<_>>();
        incoming.push_back((10001, Ok(notification(0xda))));
        if cleanup_fail {
            state.borrow_mut().fail_send = Some(7)
        }
        let mut s = Session::from_transport(
            Box::new(Fake {
                state: state.clone(),
                incoming,
            }),
            ident(),
        );
        let b = s.read_baseline().unwrap();
        let d = tempfile::tempdir().unwrap();
        let error = s.begin_calibration(&b, d.path()).unwrap_err();
        let status = error.calibration.unwrap();
        assert!(status.original_error.unwrap().contains("deadline"));
        assert_eq!(state.borrow().time, 10000);
        assert!(matches!(
            status.unlock,
            UnlockOutcome::Sent | UnlockOutcome::Uncertain(_)
        ));
        assert_eq!(
            matches!(status.unlock, UnlockOutcome::Uncertain(_)),
            cleanup_fail
        );
        let sent = state.borrow().sent.len();
        assert!(s.begin_calibration(&b, d.path()).is_err());
        assert!(s.calibrate_held_keys().is_err());
        assert!(s.cancel_calibration().is_err());
        assert_eq!(state.borrow().sent.len(), sent);
    }
}
#[test]
fn calibration_disconnect_does_not_send_cleanup_to_uncertain_handle() {
    let p = live_fixture();
    let state = Rc::new(RefCell::new(State::default()));
    let mut incoming = repeated_stream(&[&p, &p])
        .into_iter()
        .map(|r| (0, Ok(r)))
        .collect::<VecDeque<_>>();
    incoming.push_back((0, Err("disconnected input".into())));
    let mut s = Session::from_transport(
        Box::new(Fake {
            state: state.clone(),
            incoming,
        }),
        ident(),
    );
    let b = s.read_baseline().unwrap();
    let d = tempfile::tempdir().unwrap();
    let e = s.begin_calibration(&b, d.path()).unwrap_err();
    let status = e.calibration.unwrap();
    assert_eq!(status.unlock, UnlockOutcome::NotAttempted);
    assert!(e.message.contains("typing may remain locked"));
    assert_eq!(
        state.borrow().sent.iter().filter(|r| r[2] == 0xd9).count(),
        1
    );
}
#[test]
fn calibration_prepare_rgb_and_backup_failures_never_lock() {
    for case in 0..3 {
        let p = live_fixture();
        let mut replies = repeated_stream(&[&p, &p]);
        if case == 0 {
            let n = replies.len() - 5;
            replies[n] = command(0xf0).to_vec();
        }
        let (mut s, state) = session(replies);
        let b = s.read_baseline().unwrap();
        let d = tempfile::tempdir().unwrap();
        let target = if case == 1 {
            let file = d.path().join("occupied");
            std::fs::write(&file, b"not directory").unwrap();
            file
        } else {
            d.path().to_owned()
        };
        if case == 2 {
            s.backup_for_test(|_, _| Err(Error::new("injected durable cfg failure")));
        }
        let e = s.begin_calibration(&b, &target).unwrap_err();
        assert!(!e.began_write);
        let status = e.calibration.unwrap();
        assert_eq!(status.unlock, UnlockOutcome::NotAttempted);
        assert!(
            state
                .borrow()
                .sent
                .iter()
                .all(|r| !matches!(r[2], 0xd9 | 0xdb | 0xdd))
        );
    }
}
#[test]
fn calibration_postflight_failure_and_rgb_mismatch_leave_unlock_sent_and_session_blocked() {
    for case in 0..4 {
        let p = live_fixture();
        let mut replies = calibration_replies(&p);
        let postflight_start = stream(&p).len() * 2 + 5 + 2;
        if case == 0 {
            replies.truncate(postflight_start);
        } else if case == 1 {
            let mut changed = version();
            changed[20] ^= 1;
            replies[postflight_start] = changed;
        } else if case == 2 {
            replies[postflight_start + 10][63] ^= 1;
        } else {
            let n = replies.len() - 5;
            replies[n][3] ^= 1;
        }
        let (mut s, state) = session(replies);
        let b = s.read_baseline().unwrap();
        let d = tempfile::tempdir().unwrap();
        s.begin_calibration(&b, d.path()).unwrap();
        s.calibrate_held_keys().unwrap();
        let error = s.finish_calibration().unwrap_err();
        let status = error.calibration.unwrap();
        assert_eq!(status.phase, CalibrationPhase::Failed);
        assert_eq!(status.unlock, UnlockOutcome::Sent);
        assert!(!status.settings_verified && !status.lighting_verified);
        assert!(s.read_baseline().is_err());
        assert_eq!(
            state
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
fn calibration_duplicate_da_during_held_is_not_completion_or_retried() {
    let p = live_fixture();
    let mut replies = repeated_stream(&[&p, &p]);
    replies.extend([notification(0xda), notification(0xda), notification(0xde)]);
    let (mut s, state) = session(replies);
    let b = s.read_baseline().unwrap();
    let d = tempfile::tempdir().unwrap();
    s.begin_calibration(&b, d.path()).unwrap();
    let error = s.calibrate_held_keys().unwrap_err();
    let status = error.calibration.unwrap();
    assert!(status.release_completed);
    assert!(!status.held_completed);
    assert_eq!(status.unlock, UnlockOutcome::Sent);
    assert_eq!(
        state.borrow().sent.iter().filter(|r| r[2] == 0xdd).count(),
        1
    );
    assert!(s.calibrate_held_keys().is_err());
}
#[test]
fn calibration_finish_before_held_and_repeated_cancel_emit_no_extra_commands() {
    let p = live_fixture();
    let (mut s, state) = session(calibration_replies(&p));
    let b = s.read_baseline().unwrap();
    let d = tempfile::tempdir().unwrap();
    assert!(s.calibrate_held_keys().is_err());
    s.begin_calibration(&b, d.path()).unwrap();
    let count = state.borrow().sent.len();
    assert!(s.finish_calibration().is_err());
    assert_eq!(state.borrow().sent.len(), count);
    let aborted = s.cancel_calibration().unwrap();
    assert_eq!(aborted.phase, CalibrationPhase::Aborted);
    assert_eq!(aborted.unlock, UnlockOutcome::Sent);
    assert!(s.cancel_calibration().is_err());
    assert!(s.read_baseline().is_err());
    assert_eq!(state.borrow().sent.len(), count + 1);
}
#[test]
fn trial_rgb_preflight_failure_poisons_and_never_f1() {
    let p = live_fixture();
    let mut replies = repeated_stream(&[&p, &p]);
    let n = replies.len() - 5;
    replies[n] = vec![0; 64];
    let (mut s, state) = session(replies);
    let b = s.read_baseline().unwrap();
    let d = tempfile::tempdir().unwrap();
    assert!(s.trial_single_key(&b, 0, 28, d.path()).is_err());
    assert!(s.read_profile().is_err());
    assert_eq!(f1_count(&state), 0);
}
#[test]
fn pending_output_and_user_wait_retain_owner_and_lock_until_cleanup_settles() {
    use std::sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    };
    struct Blocking {
        incoming: VecDeque<Vec<u8>>,
        sent: Arc<Mutex<Vec<Vec<u8>>>>,
        gate: Arc<(Mutex<bool>, Condvar)>,
        entered: mpsc::Sender<()>,
        block_lock: bool,
        _lock: std::fs::File,
    }
    impl Transport for Blocking {
        fn queued_input(&mut self) -> std::result::Result<Option<Vec<u8>>, String> {
            Ok(None)
        }
        fn write(&mut self, b: &[u8]) -> std::result::Result<usize, String> {
            self.sent.lock().unwrap().push(b.to_vec());
            if self.block_lock && b[2] == 0xd9 && b[3] == 0 {
                self.entered.send(()).unwrap();
                let (mut settled, changed) = (self.gate.0.lock().unwrap(), &self.gate.1);
                while !*settled {
                    settled = changed.wait(settled).unwrap();
                }
            }
            Ok(b.len())
        }
        fn read(&mut self, b: &mut [u8], _: i32) -> std::result::Result<usize, String> {
            let r = self
                .incoming
                .pop_front()
                .ok_or_else(|| "no injected response".to_string())?;
            b[..r.len()].copy_from_slice(&r);
            Ok(r.len())
        }
        fn now_ms(&self) -> u64 {
            0
        }
        fn sleep_ms(&mut self, _: u64) {}
    }
    for block_lock in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let lockdir = dir.path().join("locks");
        let backup = dir.path().join("backup");
        let p = live_fixture();
        let replies = calibration_replies(&p);
        let sent = Arc::new(Mutex::new(vec![]));
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let flag = Arc::new(AtomicBool::new(false));
        let (entered_tx, entered_rx) = mpsc::channel();
        let worker_lockdir = lockdir.clone();
        let worker_sent = sent.clone();
        let worker_gate = gate.clone();
        let worker_flag = flag.clone();
        let worker = std::thread::spawn(move || {
            let lock =
                crate::transport::acquire_lock(&worker_lockdir, "controlled-test-device").unwrap();
            let transport = Blocking {
                incoming: replies.into(),
                sent: worker_sent,
                gate: worker_gate.clone(),
                entered: entered_tx.clone(),
                block_lock,
                _lock: lock,
            };
            let mut session = Session::from_transport(Box::new(transport), ident());
            session.cancel_flag = worker_flag;
            let baseline = session.read_baseline().unwrap();
            let result = session.begin_calibration(&baseline, &backup);
            if block_lock {
                return result.unwrap_err().calibration.unwrap().phase;
            }
            assert_eq!(result.unwrap().phase, CalibrationPhase::AwaitingHeldKeys);
            entered_tx.send(()).unwrap();
            let (mut proceed, changed) = (worker_gate.0.lock().unwrap(), &worker_gate.1);
            while !*proceed {
                proceed = changed.wait(proceed).unwrap();
            }
            drop(proceed);
            session.cancel_calibration().unwrap().phase
        });
        entered_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert!(crate::transport::acquire_lock(&lockdir, "controlled-test-device").is_err());
        let before = sent.lock().unwrap().len();
        flag.store(true, Ordering::Release);
        assert_eq!(sent.lock().unwrap().len(), before);
        assert!(!sent.lock().unwrap().iter().any(|r| r[2] == 0xdd));
        if block_lock {
            assert!(!sent.lock().unwrap().iter().any(|r| r[2] == 0xdb));
        }
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();
        assert_eq!(worker.join().unwrap(), CalibrationPhase::Aborted);
        assert!(crate::transport::acquire_lock(&lockdir, "controlled-test-device").is_ok());
        let commands = sent.lock().unwrap();
        assert_eq!(
            commands
                .iter()
                .filter(|r| r[2] == 0xd9 && r[3] == 1)
                .count(),
            1
        );
    }
}
#[test]
fn snapshot_final_gate_reopens_every_artifact_and_rejects_tampering() {
    for artifact in 0..3 {
        let d = tempfile::tempdir().unwrap();
        let p = live_fixture();
        let snapshot = crate::backup::save_snapshot(&p, &rgb(), d.path(), save_backup).unwrap();
        let path = match artifact {
            0 => snapshot.config_path.clone(),
            1 => snapshot.lighting_path.clone(),
            _ => snapshot.manifest_path.clone(),
        };
        let mut bytes = std::fs::read(&path).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        std::fs::write(path, bytes).unwrap();
        assert!(crate::backup::verify_snapshot(&snapshot, &p, &rgb()).is_err());
    }
}
#[test]
fn calibration_wrong_target_and_foreign_stale_tokens_never_dispatch() {
    let p = live_fixture();
    let (mut a, _) = session(stream(&p));
    let foreign = a.read_baseline().unwrap();
    let (mut s, state) = session(repeated_stream(&[&p, &p]));
    let stale = s.read_baseline().unwrap();
    let current = s.read_baseline().unwrap();
    for b in [&foreign, &stale] {
        assert!(s.begin_calibration(b, Path::new("/unused")).is_err());
    }
    assert_eq!(state.borrow().sent.len(), 4);
    let mut wrong = p.clone();
    wrong.firmware = "66EC(XRGB)BLe;V1.0.49;V1.0;".into();
    let (mut other, state) = session(stream(&wrong));
    let b = other.read_baseline().unwrap();
    assert!(other.begin_calibration(&b, Path::new("/unused")).is_err());
    assert_eq!(state.borrow().sent.len(), 2);
    assert_eq!(current.profile().records, p.records);
}
#[test]
fn every_configuration_output_failure_retains_snapshot_and_never_retries() {
    let p = live_fixture();
    let d = tempfile::tempdir().unwrap();
    for fault in 5..=204 {
        let (mut s, state) = session(repeated_stream(&[&p, &p]));
        let baseline = s.read_baseline().unwrap();
        state.borrow_mut().fail_send = Some(fault);
        let error = s.trial_single_key(&baseline, 0, 28, d.path()).unwrap_err();
        assert!(error.began_write);
        assert!(error.backup_path.unwrap().exists());
        let length = state.borrow().sent.len();
        assert!(s.read_profile().is_err());
        assert_eq!(state.borrow().sent.len(), length);
        assert_eq!(f1_count(&state), usize::from(fault > 5));
    }
}
#[test]
fn trial_rgb_readback_mismatch_retains_actual_native_reports() {
    let p = live_fixture();
    let target = desired(&p, 28);
    let mut replies = repeated_stream(&[&p, &p]);
    replies.extend(stream(&target));
    let mut changed = rgb();
    changed[100] ^= 1;
    let native = rgb_stream(&changed);
    replies.extend(native.clone());
    let (mut s, state) = session(replies);
    let b = s.read_baseline().unwrap();
    let d = tempfile::tempdir().unwrap();
    let e = s.trial_single_key(&b, 0, 28, d.path()).unwrap_err();
    assert!(e.began_write);
    assert!(e.message.contains("lighting"));
    assert_eq!(e.raw_reports, native);
    assert!(s.version().is_err());
    assert_eq!(f1_count(&state), 1);
}
#[test]
fn fresh_baseline_required_after_success_and_new_calibration_requires_explicit_begin() {
    let p = live_fixture();
    let mut replies = calibration_replies(&p);
    replies.extend(stream(&p));
    let (mut s, state) = session(replies);
    let b = s.read_baseline().unwrap();
    let d = tempfile::tempdir().unwrap();
    s.begin_calibration(&b, d.path()).unwrap();
    s.calibrate_held_keys().unwrap();
    s.finish_calibration().unwrap();
    assert!(s.begin_calibration(&b, d.path()).is_err());
    assert!(s.trial_swap(&b, 0, 1, d.path()).is_err());
    let count = state.borrow().sent.len();
    assert!(s.calibrate_held_keys().is_err());
    assert!(s.finish_calibration().is_err());
    assert_eq!(state.borrow().sent.len(), count);
    let trace_before = serde_json::to_string(&s.calibration_status().unwrap().trace).unwrap();
    let fresh = s.read_baseline().unwrap();
    assert!(s.check_apply(&fresh, fresh.profile()).is_ok());
    assert_eq!(
        serde_json::to_string(&s.calibration_status().unwrap().trace).unwrap(),
        trace_before
    );
}
#[test]
fn calibration_completion_at_deadline_and_unknown_third_byte_are_preserved() {
    let p = live_fixture();
    let state = Rc::new(RefCell::new(State::default()));
    let mut incoming = repeated_stream(&[&p, &p])
        .into_iter()
        .map(|r| (0, Ok(r)))
        .collect::<VecDeque<_>>();
    let mut da = notification(0xda);
    da[2] = 0x71;
    incoming.push_back((10000, Ok(da.clone())));
    let mut s = Session::from_transport(
        Box::new(Fake {
            state: state.clone(),
            incoming,
        }),
        ident(),
    );
    let b = s.read_baseline().unwrap();
    let d = tempfile::tempdir().unwrap();
    let status = s.begin_calibration(&b, d.path()).unwrap();
    assert_eq!(status.phase, CalibrationPhase::AwaitingHeldKeys);
    assert_eq!(state.borrow().time, 10000);
    assert!(status.trace.iter().any(|p| p.bytes == da));
}
#[test]
fn held_cancellation_before_send_and_after_completed_send_is_aborted_once() {
    for during in [false, true] {
        let p = live_fixture();
        let replies = if during {
            let mut r = repeated_stream(&[&p, &p]);
            r.push(notification(0xda));
            r
        } else {
            calibration_replies(&p)
        };
        let (mut s, state) = session(replies);
        let b = s.read_baseline().unwrap();
        let d = tempfile::tempdir().unwrap();
        s.begin_calibration(&b, d.path()).unwrap();
        state.borrow_mut().cancel_flag = Some(s.cancellation_flag());
        if during {
            state.borrow_mut().cancel_at = Some(1500)
        } else {
            s.cancellation_flag()
                .store(true, std::sync::atomic::Ordering::Release)
        }
        let e = s.calibrate_held_keys().unwrap_err();
        let status = e.calibration.unwrap();
        assert_eq!(status.phase, CalibrationPhase::Aborted);
        assert_eq!(status.unlock, UnlockOutcome::Sent);
        assert!(status.release_completed);
        assert!(!status.held_completed);
        assert_eq!(status.held_sent, during);
        assert_eq!(
            state.borrow().sent.iter().filter(|r| r[2] == 0xdd).count(),
            usize::from(during)
        );
        let count = state.borrow().sent.len();
        assert!(s.cancel_calibration().is_err());
        assert_eq!(state.borrow().sent.len(), count);
    }
}
#[test]
fn queued_stale_calibration_notification_before_dd_aborts_without_held_output() {
    for stale in [notification(0xde), notification(0xda), vec![0x71; 17]] {
        let p = live_fixture();
        let (mut s, state) = session(calibration_replies(&p));
        let b = s.read_baseline().unwrap();
        let d = tempfile::tempdir().unwrap();
        s.begin_calibration(&b, d.path()).unwrap();
        state.borrow_mut().unsolicited.push_back(stale.clone());
        let e = s.calibrate_held_keys().unwrap_err();
        let status = e.calibration.unwrap();
        assert_eq!(status.phase, CalibrationPhase::Failed);
        assert!(!status.held_attempted && !status.held_completed);
        assert_eq!(status.unlock, UnlockOutcome::Sent);
        assert_eq!(e.raw_reports, vec![stale.clone()]);
        assert!(
            status
                .trace
                .iter()
                .any(|t| t.direction == TraceDirection::Input && t.bytes == stale)
        );
        assert!(!state.borrow().sent.iter().any(|r| r[2] == 0xdd));
    }
}
#[test]
fn queued_notification_before_lock_or_db_is_failure_and_no_release_dispatch() {
    for after_lock in [false, true] {
        let p = live_fixture();
        let (mut s, state) = session(calibration_replies(&p));
        let b = s.read_baseline().unwrap();
        if after_lock {
            state.borrow_mut().unsolicited_after_lock = Some(notification(0xda));
        } else {
            state.borrow_mut().unsolicited.push_back(notification(0xda));
        }
        let d = tempfile::tempdir().unwrap();
        let e = s.begin_calibration(&b, d.path()).unwrap_err();
        let status = e.calibration.unwrap();
        assert!(!status.release_attempted);
        assert!(!state.borrow().sent.iter().any(|r| r[2] == 0xdb));
        assert_eq!(status.lock_may_have_been_sent, after_lock);
        assert_eq!(
            status.unlock,
            if after_lock {
                UnlockOutcome::Sent
            } else {
                UnlockOutcome::NotAttempted
            }
        );
    }
}
#[test]
fn native_quiet_probe_algorithm_is_nonblocking_and_preserves_unnumbered_bytes() {
    struct Probe {
        next: Option<Vec<u8>>,
    }
    impl Transport for Probe {
        fn write(&mut self, _: &[u8]) -> std::result::Result<usize, String> {
            panic!("probe cannot send")
        }
        fn read(&mut self, b: &mut [u8], timeout: i32) -> std::result::Result<usize, String> {
            assert_eq!(timeout, 0);
            if let Some(bytes) = self.next.take() {
                b[..bytes.len()].copy_from_slice(&bytes);
                Ok(bytes.len())
            } else {
                Ok(0)
            }
        }
        fn now_ms(&self) -> u64 {
            0
        }
        fn sleep_ms(&mut self, _: u64) {
            panic!("probe cannot wait")
        }
    }
    for expected in [notification(0xde), vec![0, 0xda, 0x91]] {
        let mut probe = Probe {
            next: Some(expected.clone()),
        };
        assert_eq!(probe.queued_input().unwrap(), Some(expected));
        assert_eq!(probe.queued_input().unwrap(), None);
    }
}
#[test]
fn native_all_e6_terminator_completes_exact_rgb_capture() {
    let mut replies = vec![version()];
    let mut native = rgb_stream(&rgb());
    *native.last_mut().unwrap() = vec![0xe6; 64];
    replies.extend(native);
    let (mut s, state) = session(replies);
    assert_eq!(s.read_lighting().unwrap(), rgb());
    assert_eq!(
        state.borrow().sent.iter().map(|r| r[2]).collect::<Vec<_>>(),
        vec![0xf9, 0xe2]
    );
}
#[test]
fn native_all_e6_terminator_completes_exact_counter_capture() {
    let mut version = command(0xf9);
    let firmware = b"66EC(S)BLe;V1.0.37;V1.0;";
    version[2..2 + firmware.len()].copy_from_slice(firmware);
    let mut replies = vec![version.to_vec()];
    let bytes = (0..66u32).flat_map(|n| n.to_le_bytes()).collect::<Vec<_>>();
    for part in bytes.chunks(61) {
        let mut r = command(0xe3);
        r[2] = part.len() as u8;
        r[3..3 + part.len()].copy_from_slice(part);
        replies.push(r.to_vec());
    }
    replies.push(vec![0xe6; 64]);
    let (mut s, state) = session(replies);
    assert_eq!(s.read_counters().unwrap(), (0..66).collect::<Vec<_>>());
    assert_eq!(
        state.borrow().sent.iter().map(|r| r[2]).collect::<Vec<_>>(),
        vec![0xf9, 0xe3]
    );
}
#[test]
fn known_e6_forms_still_require_exact_byte_counts_and_unknown_forms_fail() {
    for counters in [false, true] {
        let expected = if counters { 264 } else { 198 };
        for delta in [-1i32, 0, 1] {
            for full in [false, true] {
                let mut version = command(0xf9);
                let firmware = if counters {
                    "66EC(S)BLe;V1.0.37;V1.0;"
                } else {
                    crate::session::EXPERIMENTAL_FIRMWARE
                };
                version[2..2 + firmware.len()].copy_from_slice(firmware.as_bytes());
                let mut replies = vec![version.to_vec()];
                let bytes = vec![0x37; (expected + delta) as usize];
                for part in bytes.chunks(61) {
                    let mut r = command(if counters { 0xe3 } else { 0xe0 });
                    r[2] = part.len() as u8;
                    r[3..3 + part.len()].copy_from_slice(part);
                    replies.push(r.to_vec());
                }
                replies.push(if full {
                    vec![0xe6; 64]
                } else {
                    let mut r = command(0xe6);
                    r[63] = 0xa5;
                    r.to_vec()
                });
                let (mut session, _) = session(replies);
                let success = if counters {
                    session.read_counters().is_ok()
                } else {
                    session.read_lighting().is_ok()
                };
                assert_eq!(
                    success,
                    delta == 0,
                    "counter={counters} delta={delta} full={full}"
                );
            }
        }
    }
    for corrupt in [0usize, 2, 63] {
        let mut native = rgb_stream(&rgb());
        let mut invalid = vec![0xe6; 64];
        invalid[corrupt] ^= 1;
        *native.last_mut().unwrap() = invalid;
        let mut replies = vec![version()];
        replies.extend(native);
        let (mut s, _) = session(replies);
        assert!(s.read_lighting().is_err());
    }
}
#[test]
fn native_hardware_rgb_capture_matches_independent_golden_bytes() {
    let capture: serde_json::Value = serde_json::from_str(include_str!(
        "../tests/fixtures/atom66-xrgb-ble-v1.0.48-rgb-capture.json"
    ))
    .unwrap();
    let mut incoming = vec![version()];
    let native = capture["raw_reports"]
        .as_array()
        .unwrap()
        .iter()
        .map(|report| {
            report
                .as_array()
                .unwrap()
                .iter()
                .map(|b| u8::try_from(b.as_u64().unwrap()).unwrap())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(native.len(), 5);
    assert_eq!(native.last().unwrap(), &vec![0xe6; 64]);
    incoming.extend(native);
    let (mut session, state) = session(incoming);
    let actual = session.read_lighting().unwrap();
    assert_eq!(
        actual.as_slice(),
        include_bytes!("../tests/fixtures/atom66-xrgb-ble-v1.0.48-rgb.bin")
    );
    assert_eq!(actual.len(), 198);
    assert_eq!(
        state.borrow().sent.iter().map(|r| r[2]).collect::<Vec<_>>(),
        vec![0xf9, 0xe2]
    );
}
