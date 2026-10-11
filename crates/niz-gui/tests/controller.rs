use niz_core::{DeviceIdentity, Profile};
use niz_device::{CalibrationPhase, CalibrationStatus, UnlockOutcome};
use niz_gui::{
    controller::{Activation, Controller, ReadChoice},
    editor::{Editor, Mode, exact_trial},
    protocol::*,
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
#[derive(Default)]
struct FakePort {
    requests: Vec<Request>,
}
impl Port for FakePort {
    fn send(&mut self, r: Request) -> Result<(), String> {
        self.requests.push(r);
        Ok(())
    }
}
fn target() -> Profile {
    let mut p = Profile::demo();
    p.firmware = "66EC(XRGB)BLe;V1.0.48;V1.0;".into();
    p.identity = DeviceIdentity {
        vendor_id: 0x0483,
        product_id: 0x502a,
        usage_page: 0x8c,
        usage: 1,
    };
    p
}
fn status(phase: CalibrationPhase) -> CalibrationStatus {
    CalibrationStatus {
        phase,
        failure_phase: None,
        backup: None,
        lock_may_have_been_sent: true,
        release_attempted: true,
        release_sent: true,
        release_completed: true,
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
fn connected() -> Controller<FakePort> {
    let mut c = Controller::new(FakePort::default());
    c.read("target".into(), ReadChoice::Discard).unwrap();
    let ticket = c.pending.unwrap();
    c.process(Event {
        ticket,
        response: Response::Read {
            profile: target(),
            cancel: Arc::new(AtomicBool::new(false)),
        },
    });
    c
}
#[test]
fn read_response_attaches_only_current_operation_and_generation() {
    let mut c = Controller::new(FakePort::default());
    c.read("target".into(), ReadChoice::Discard).unwrap();
    let t = c.pending.unwrap();
    c.process(Event {
        ticket: Ticket {
            generation: t.generation + 1,
            ..t
        },
        response: Response::Read {
            profile: target(),
            cancel: Arc::new(AtomicBool::new(false)),
        },
    });
    assert!(c.editor.is_none());
    assert_eq!(c.pending, Some(t));
    c.process(Event {
        ticket: Ticket {
            operation: t.operation + 1,
            ..t
        },
        response: Response::Read {
            profile: target(),
            cancel: Arc::new(AtomicBool::new(false)),
        },
    });
    assert!(c.editor.is_none());
    c.process(Event {
        ticket: t,
        response: Response::Read {
            profile: target(),
            cancel: Arc::new(AtomicBool::new(false)),
        },
    });
    assert!(c.live);
}
#[test]
fn double_click_read_emits_one_request() {
    let mut c = Controller::new(FakePort::default());
    c.read("target".into(), ReadChoice::Discard).unwrap();
    assert!(c.read("target".into(), ReadChoice::Discard).is_err());
    assert_eq!(c.port.requests.len(), 1);
}
#[test]
fn exact_trial_rejects_multiedit_padding_and_foreign_metadata() {
    let p = target();
    let mut e = Editor::new(p.clone());
    e.selection.to_code = 43;
    e.edit().unwrap();
    assert!(exact_trial(&p, &e.draft, e.selection).is_ok());
    let saved = e.selection;
    e.selection.from = 2;
    e.selection.to_code = 44;
    e.edit().unwrap();
    assert!(exact_trial(&p, &e.draft, saved).is_err());
    let mut one = niz_gui::editor::regenerate(&p, saved).unwrap();
    one.records[8][0][63] = 21;
    assert!(exact_trial(&p, &one, saved).is_err());
    one = niz_gui::editor::regenerate(&p, saved).unwrap();
    one.identity.product_id = 0x512a;
    assert!(exact_trial(&p, &one, saved).is_err());
}
#[test]
fn exact_trial_rejects_fn_columns_and_wrong_firmware() {
    let p = target();
    let mut e = Editor::new(p.clone());
    e.selection.mode = Mode::Swap;
    e.selection.from = 54;
    e.selection.to_position = 2;
    e.edit().unwrap();
    assert!(exact_trial(&p, &e.draft, e.selection).is_err());
    let mut other = p;
    other.firmware = "66EC(S)BLe;V1.0.48;V1.0;".into();
    let e = Editor::new(other.clone());
    assert!(exact_trial(&other, &e.draft, e.selection).is_err());
}
#[test]
fn trial_sends_operands_and_exact_draft_once() {
    let mut c = connected();
    let e = c.editor.as_mut().unwrap();
    e.selection.to_code = 43;
    c.edit().unwrap();
    c.trial(PathBuf::from("backup")).unwrap();
    assert!(c.trial(PathBuf::from("backup")).is_err());
    assert!(
        matches!(&c.port.requests.last().unwrap().command,Command::Trial{selection,..} if selection.to_code==43)
    );
}
#[test]
fn file_profile_never_grants_live_baseline() {
    let mut c = Controller::new(FakePort::default());
    c.load(target(), true).unwrap();
    c.editor.as_mut().unwrap().selection.to_code = 43;
    c.edit().unwrap();
    assert!(c.trial("backup".into()).is_err());
    assert!(!c.live);
}
#[test]
fn disconnect_error_retains_draft_and_history() {
    let mut c = connected();
    c.editor.as_mut().unwrap().selection.to_code = 43;
    c.edit().unwrap();
    let draft = c.editor.as_ref().unwrap().draft.records.clone();
    c.read("target".into(), ReadChoice::Retain).unwrap();
    let t = c.pending.unwrap();
    c.process(Event {
        ticket: t,
        response: Response::Error("disconnected".to_string().into()),
    });
    assert_eq!(c.editor.as_ref().unwrap().draft.records, draft);
    assert!(!c.live);
    assert!(c.editor.as_mut().unwrap().undo());
}
#[test]
fn retain_reconnect_requires_same_source_and_unchanged_baseline() {
    let mut c = connected();
    c.editor.as_mut().unwrap().selection.to_code = 43;
    c.edit().unwrap();
    c.read("other-path".into(), ReadChoice::Retain).unwrap();
    let t = c.pending.unwrap();
    c.process(Event {
        ticket: t,
        response: Response::Read {
            profile: target(),
            cancel: Arc::new(AtomicBool::new(false)),
        },
    });
    assert!(!c.live);
    assert!(c.editor.as_ref().unwrap().dirty());
    c.read("target".into(), ReadChoice::Retain).unwrap();
    let t = c.pending.unwrap();
    c.process(Event {
        ticket: t,
        response: Response::Read {
            profile: target(),
            cancel: Arc::new(AtomicBool::new(false)),
        },
    });
    assert!(c.live);
    assert!(c.editor.as_ref().unwrap().dirty());
}
#[test]
fn changed_reread_retains_detached_draft_until_explicit_discard() {
    let mut c = connected();
    c.editor.as_mut().unwrap().selection.to_code = 43;
    c.edit().unwrap();
    let before = c.editor.as_ref().unwrap().draft.records.clone();
    c.read("target".into(), ReadChoice::Retain).unwrap();
    let t = c.pending.unwrap();
    let mut changed = target();
    changed.records[1][0][6] = 44;
    c.process(Event {
        ticket: t,
        response: Response::Read {
            profile: changed,
            cancel: Arc::new(AtomicBool::new(false)),
        },
    });
    assert!(!c.live);
    assert_eq!(c.editor.as_ref().unwrap().draft.records, before);
    c.read("target".into(), ReadChoice::Discard).unwrap();
    let t = c.pending.unwrap();
    c.process(Event {
        ticket: t,
        response: Response::Read {
            profile: target(),
            cancel: Arc::new(AtomicBool::new(false)),
        },
    });
    assert!(!c.editor.as_ref().unwrap().dirty());
    assert!(c.live);
}
#[test]
fn keyboard_only_calibration_activation_never_dispatches() {
    let mut c = connected();
    let n = c.port.requests.len();
    assert!(c.begin("backup".into(), Activation::Keyboard).is_err());
    assert_eq!(c.port.requests.len(), n);
    c.calibration = Some(status(CalibrationPhase::AwaitingHeldKeys));
    assert!(c.held(Activation::Keyboard).is_err());
    assert_eq!(c.port.requests.len(), n);
}
#[test]
fn stage_gates_and_unrelated_work_block_during_user_wait() {
    let mut c = connected();
    assert!(c.held(Activation::Pointer).is_err());
    c.begin("backup".into(), Activation::Pointer).unwrap();
    assert!(c.begin("backup".into(), Activation::Pointer).is_err());
    let t = c.pending.unwrap();
    c.process(Event {
        ticket: t,
        response: Response::Calibration(status(CalibrationPhase::AwaitingHeldKeys)),
    });
    assert!(c.read("other".into(), ReadChoice::Discard).is_err());
    assert!(c.load(target(), true).is_err());
    assert!(c.edit().is_err());
    assert!(c.trial("backup".into()).is_err());
    assert!(c.finish(Activation::Pointer).is_err());
    c.held(Activation::Pointer).unwrap();
    let t = c.pending.unwrap();
    c.process(Event {
        ticket: t,
        response: Response::Calibration(status(CalibrationPhase::AwaitingFinish)),
    });
    assert!(c.held(Activation::Pointer).is_err());
    c.finish(Activation::Pointer).unwrap();
}
#[test]
fn close_during_pending_output_sets_flag_and_waits_before_cleanup() {
    let mut c = connected();
    c.begin("backup".into(), Activation::Pointer).unwrap();
    let begin = c.pending.unwrap();
    let n = c.port.requests.len();
    c.close();
    assert!(c.cancel.load(Ordering::Acquire));
    assert!(!c.ready_close);
    assert_eq!(c.port.requests.len(), n);
    c.process(Event {
        ticket: begin,
        response: Response::Calibration(status(CalibrationPhase::AwaitingHeldKeys)),
    });
    assert!(matches!(
        c.port.requests.last().unwrap().command,
        Command::Cancel
    ));
    assert!(!c.ready_close);
    let cancel = c.pending.unwrap();
    let mut terminal = status(CalibrationPhase::Aborted);
    terminal.unlock = UnlockOutcome::Sent;
    c.process(Event {
        ticket: cancel,
        response: Response::Calibration(terminal),
    });
    assert!(c.ready_close);
    assert!(!c.live);
}
#[test]
fn failure_preserves_original_phase_and_uncertain_unlock_for_display() {
    let mut c = connected();
    c.begin("backup".into(), Activation::Pointer).unwrap();
    let t = c.pending.unwrap();
    let mut s = status(CalibrationPhase::Failed);
    s.failure_phase = Some(CalibrationPhase::ReleaseWaiting);
    s.original_error = Some("original timeout".into());
    s.unlock = UnlockOutcome::Uncertain("short cleanup".into());
    c.process(Event {
        ticket: t,
        response: Response::Error(Failure {
            message: "original timeout".into(),
            calibration: Some(Box::new(s)),
            backup: Some("backup.json".into()),
            began_write: true,
            raw_reports: vec![vec![1, 2, 3]],
        }),
    });
    let s = c.calibration.as_ref().unwrap();
    assert_eq!(s.failure_phase, Some(CalibrationPhase::ReleaseWaiting));
    assert_eq!(s.original_error.as_deref(), Some("original timeout"));
    assert_eq!(s.unlock, UnlockOutcome::Uncertain("short cleanup".into()));
    assert!(!c.live);
}
#[test]
fn successful_calibration_requires_explicit_fresh_read_before_trial() {
    let mut c = connected();
    c.calibration = Some(status(CalibrationPhase::AwaitingFinish));
    c.finish(Activation::Pointer).unwrap();
    let t = c.pending.unwrap();
    let mut s = status(CalibrationPhase::Complete);
    s.unlock = UnlockOutcome::Sent;
    s.settings_verified = true;
    s.lighting_verified = true;
    c.process(Event {
        ticket: t,
        response: Response::Finished(s),
    });
    assert!(!c.live);
    assert!(c.trial("backup".into()).is_err());
    c.read("target".into(), ReadChoice::Retain).unwrap();
}

#[test]
fn no_op_message_never_claims_a_write_or_rgb_verification() {
    let mut c = connected();
    c.trial("backup".into()).unwrap();
    let ticket = c.pending.unwrap();
    c.process(Event {
        ticket,
        response: Response::Applied {
            profile: target(),
            outcome: niz_device::ApplyOutcome::NoChange,
            backup: "backup.json".into(),
            snapshot: None,
        },
    });
    assert!(
        c.message
            .contains("No changes; no hardware write or postflight/RGB readback performed")
    );
    assert!(!c.message.contains("readback verified"));
}
#[test]
fn terminal_cancel_failure_does_not_dispatch_another_unlock() {
    let mut c = connected();
    c.begin("backup".into(), Activation::Pointer).unwrap();
    c.close();
    let ticket = c.pending.unwrap();
    let count = c.port.requests.len();
    let mut s = status(CalibrationPhase::Failed);
    s.failure_phase = Some(CalibrationPhase::Locking);
    s.unlock = UnlockOutcome::Uncertain("pending output returned short".into());
    c.process(Event {
        ticket,
        response: Response::Error(Failure {
            message: "short output".into(),
            calibration: Some(Box::new(s)),
            backup: None,
            began_write: true,
            raw_reports: vec![],
        }),
    });
    assert_eq!(c.port.requests.len(), count);
    assert!(c.ready_close);
    assert!(matches!(
        c.calibration.as_ref().unwrap().unlock,
        UnlockOutcome::Uncertain(_)
    ));
}
#[test]
fn closing_during_read_waits_for_owned_worker_before_returning() {
    let mut c = Controller::new(FakePort::default());
    c.read("target".into(), ReadChoice::Discard).unwrap();
    let ticket = c.pending.unwrap();
    c.close();
    assert!(!c.ready_close);
    c.process(Event {
        ticket,
        response: Response::Read {
            profile: target(),
            cancel: Arc::new(AtomicBool::new(false)),
        },
    });
    assert!(c.ready_close);
}
fn descriptor(path: &str) -> niz_device::DeviceDescriptor {
    niz_device::DeviceDescriptor {
        path: path.into(),
        product: "Identical keyboard".into(),
        identity: target().identity,
    }
}
fn enumerate(c: &mut Controller<FakePort>, devices: Vec<niz_device::DeviceDescriptor>) {
    c.list().unwrap();
    let ticket = c.pending.unwrap();
    c.process(Event {
        ticket,
        response: Response::Devices(devices),
    });
}
fn two_devices() -> Controller<FakePort> {
    let mut c = Controller::new(FakePort::default());
    enumerate(&mut c, vec![descriptor("A"), descriptor("B")]);
    c.select_device("A".into()).unwrap();
    c.read("A".into(), ReadChoice::Discard).unwrap();
    let ticket = c.pending.unwrap();
    c.process(Event {
        ticket,
        response: Response::Read {
            profile: target(),
            cancel: Arc::new(AtomicBool::new(false)),
        },
    });
    c.editor.as_mut().unwrap().selection.to_code = 43;
    c.edit().unwrap();
    c
}
#[test]
fn selection_change_detaches_write_and_calibration_preserving_history() {
    let mut c = two_devices();
    let before = c.editor.as_ref().unwrap().draft.records.clone();
    c.select_device("B".into()).unwrap();
    assert!(!c.live);
    let n = c.port.requests.len();
    assert!(c.trial("backup".into()).is_err());
    assert!(c.begin("backup".into(), Activation::Pointer).is_err());
    assert_eq!(c.port.requests.len(), n);
    assert_eq!(c.editor.as_ref().unwrap().draft.records, before);
    assert!(c.editor.as_mut().unwrap().undo());
}
#[test]
fn list_reorder_keeps_selected_and_attached_path() {
    let mut c = two_devices();
    enumerate(&mut c, vec![descriptor("B"), descriptor("A")]);
    assert_eq!(c.selected_path.as_deref(), Some("A"));
    assert_eq!(c.attached_target.as_ref().unwrap().path, "A");
    assert!(c.live);
    c.trial("backup".into()).unwrap();
    assert_eq!(
        c.port
            .requests
            .last()
            .unwrap()
            .target
            .as_ref()
            .unwrap()
            .path,
        "A"
    );
}
#[test]
fn removed_or_changed_descriptor_detaches_without_auto_switch() {
    let mut c = two_devices();
    enumerate(&mut c, vec![descriptor("B")]);
    assert!(c.selected_path.is_none());
    assert!(!c.live);
    assert!(c.editor.as_ref().unwrap().dirty());
    enumerate(&mut c, vec![descriptor("A"), descriptor("B")]);
    assert!(!c.live);
    assert!(c.trial("backup".into()).is_err());
}
#[test]
fn read_b_retain_keeps_a_draft_detached_until_explicit_replace() {
    let mut c = two_devices();
    c.select_device("B".into()).unwrap();
    c.read("B".into(), ReadChoice::Retain).unwrap();
    let ticket = c.pending.unwrap();
    c.process(Event {
        ticket,
        response: Response::Read {
            profile: target(),
            cancel: Arc::new(AtomicBool::new(false)),
        },
    });
    assert!(!c.live);
    assert!(c.editor.as_ref().unwrap().dirty());
    c.read("B".into(), ReadChoice::Discard).unwrap();
    let ticket = c.pending.unwrap();
    c.process(Event {
        ticket,
        response: Response::Read {
            profile: target(),
            cancel: Arc::new(AtomicBool::new(false)),
        },
    });
    assert!(c.live);
    assert_eq!(c.attached_target.as_ref().unwrap().path, "B");
}
#[test]
fn approval_is_pinned_to_target_and_cannot_follow_selection_change() {
    let mut c = two_devices();
    let approval = c.prepare_trial("backup".into()).unwrap();
    assert_eq!(approval.target.path, "A");
    c.select_device("B".into()).unwrap();
    let count = c.port.requests.len();
    assert!(c.approve_trial(approval).is_err());
    assert_eq!(c.port.requests.len(), count);
}

#[test]
fn switching_back_to_original_path_still_requires_explicit_read() {
    let mut c = two_devices();
    c.select_device("B".into()).unwrap();
    c.select_device("A".into()).unwrap();
    assert!(!c.live);
    assert!(c.trial("backup".into()).is_err());
    c.read("A".into(), ReadChoice::Retain).unwrap();
    let ticket = c.pending.unwrap();
    c.process(Event {
        ticket,
        response: Response::Read {
            profile: target(),
            cancel: Arc::new(AtomicBool::new(false)),
        },
    });
    assert!(c.live);
    assert!(c.editor.as_ref().unwrap().dirty());
}
#[test]
fn same_path_with_changed_identity_detaches_the_editor() {
    let mut c = two_devices();
    let mut changed = descriptor("A");
    changed.identity.product_id = 0x512a;
    enumerate(&mut c, vec![changed, descriptor("B")]);
    assert!(!c.live);
    assert_eq!(c.selected_path.as_deref(), Some("A"));
    assert!(c.trial("backup".into()).is_err());
    assert!(c.editor.as_ref().unwrap().dirty());
}
#[test]
fn approval_cannot_apply_a_draft_changed_after_confirmation() {
    let mut c = two_devices();
    let approval = c.prepare_trial("backup".into()).unwrap();
    c.editor.as_mut().unwrap().selection.to_code = 44;
    c.edit().unwrap();
    let n = c.port.requests.len();
    assert!(c.approve_trial(approval).is_err());
    assert_eq!(c.port.requests.len(), n);
}

#[test]
fn board_navigation_sends_no_request_and_cannot_reuse_an_earlier_trial_approval() {
    use niz_gui::keyboard::KeyboardState;
    let mut c = two_devices();
    let approval = c.prepare_trial("backup".into()).unwrap();
    let requests = c.port.requests.len();
    let raw = c.editor.as_ref().unwrap().draft.reports();
    let mut board = KeyboardState::default();
    assert!(board.click(c.editor.as_mut().unwrap(), 2, false, false, false));
    assert_eq!(c.port.requests.len(), requests);
    assert_eq!(c.editor.as_ref().unwrap().draft.reports(), raw);
    assert!(c.approve_trial(approval).is_err());
    assert_eq!(c.port.requests.len(), requests);
    assert!(c.editor.as_mut().unwrap().undo());
    assert!(
        !c.editor.as_mut().unwrap().undo(),
        "navigation created no extra history entry"
    );
}
#[test]
fn detached_draft_can_navigate_but_never_dispatches_a_trial_or_calibration() {
    use niz_gui::keyboard::KeyboardState;
    let mut c = two_devices();
    c.select_device("B".into()).unwrap();
    let requests = c.port.requests.len();
    let mut board = KeyboardState::default();
    assert!(board.click(c.editor.as_mut().unwrap(), 2, false, false, false));
    assert_eq!(c.editor.as_ref().unwrap().selection.from, 2);
    assert!(c.trial("backup".into()).is_err());
    assert!(c.begin("backup".into(), Activation::Pointer).is_err());
    assert_eq!(c.port.requests.len(), requests);
}
