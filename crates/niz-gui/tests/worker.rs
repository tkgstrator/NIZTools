use niz_core::{DeviceIdentity, Profile};
use niz_device::{
    ApplyOutcome, CalibrationPhase, CalibrationStatus, DeviceDescriptor, UnlockOutcome,
};
use niz_gui::{
    editor::{Mode, Selection, regenerate},
    protocol::*,
    worker::{Backend, Worker},
};
use std::{
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};
struct Fake {
    profile: Profile,
    reads: usize,
    trial: Vec<Selection>,
    stage: Vec<&'static str>,
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
impl Backend for Fake {
    fn list(&mut self) -> Result<Vec<DeviceDescriptor>, Failure> {
        Ok(vec![])
    }
    fn read(&mut self, _: &str) -> Result<(Profile, Arc<AtomicBool>), Failure> {
        self.reads += 1;
        Ok((self.profile.clone(), Arc::new(AtomicBool::new(false))))
    }
    fn baseline(&self) -> Option<&Profile> {
        Some(&self.profile)
    }
    fn read_raw_thresholds(
        &mut self,
    ) -> Result<(niz_device::RawThresholds, RawThresholdSource), Failure> {
        Err(Failure::from(
            "No analog fixture in this legacy fake".to_string(),
        ))
    }
    fn trial(&mut self, s: Selection, _: &Path) -> Result<Response, Failure> {
        self.trial.push(s);
        Ok(Response::Applied {
            profile: regenerate(&self.profile, s).unwrap(),
            outcome: ApplyOutcome::SettingsAndLightingVerified,
            backup: "backup".into(),
            snapshot: None,
        })
    }
    fn begin(&mut self, _: &Path) -> Result<CalibrationStatus, Failure> {
        self.stage.push("begin");
        Ok(status(CalibrationPhase::AwaitingHeldKeys))
    }
    fn held(&mut self) -> Result<CalibrationStatus, Failure> {
        self.stage.push("held");
        Ok(status(CalibrationPhase::AwaitingFinish))
    }
    fn finish(&mut self) -> Result<CalibrationStatus, Failure> {
        self.stage.push("finish");
        Ok(status(CalibrationPhase::Complete))
    }
    fn cancel(&mut self) -> Result<CalibrationStatus, Failure> {
        self.stage.push("cancel");
        Ok(status(CalibrationPhase::Aborted))
    }
}
fn worker() -> Worker<Fake> {
    let mut p = Profile::demo();
    p.firmware = "66EC(XRGB)BLe;V1.0.48;V1.0;".into();
    p.identity = DeviceIdentity {
        vendor_id: 0x0483,
        product_id: 0x502a,
        usage_page: 0x8c,
        usage: 1,
    };
    Worker::new(Fake {
        profile: p,
        reads: 0,
        trial: vec![],
        stage: vec![],
    })
}
fn request(g: u64, o: u64, command: Command) -> Request {
    Request {
        ticket: Ticket {
            generation: g,
            operation: o,
        },
        command,
        target: Some(Target {
            path: "device".into(),
            product: "NIZ keyboard".into(),
            identity: worker().backend.profile.identity,
        }),
    }
}
fn read(w: &mut Worker<Fake>) {
    assert!(matches!(
        w.route(request(
            1,
            1,
            Command::Read {
                path: "device".into()
            }
        ))
        .response,
        Response::Read { .. }
    ));
}
#[test]
fn worker_rejects_arbitrary_draft_even_if_ui_gates_are_bypassed() {
    let mut w = worker();
    read(&mut w);
    let s = Selection {
        to_code: 43,
        ..Selection::default()
    };
    let mut draft = regenerate(&w.backend.profile, s).unwrap();
    draft.records[10][0][63] = 33;
    assert!(matches!(
        w.route(request(
            1,
            2,
            Command::Trial {
                draft,
                selection: s,
                backup: "b".into()
            }
        ))
        .response,
        Response::Error(_)
    ));
    assert!(w.backend.trial.is_empty());
}
#[test]
fn worker_routes_only_exact_basic_operands_from_native_baseline() {
    for mode in [Mode::Reassign, Mode::Swap] {
        let mut w = worker();
        read(&mut w);
        let s = Selection {
            to_code: 43,
            mode,
            ..Selection::default()
        };
        let draft = regenerate(&w.backend.profile, s).unwrap();
        assert!(matches!(
            w.route(request(
                1,
                2,
                Command::Trial {
                    draft,
                    selection: s,
                    backup: "b".into()
                }
            ))
            .response,
            Response::Applied { .. }
        ));
        assert_eq!(w.backend.trial, vec![s]);
    }
}
#[test]
fn stale_generation_and_replayed_operation_never_call_backend() {
    let mut w = worker();
    read(&mut w);
    let s = Selection {
        to_code: 43,
        ..Selection::default()
    };
    for (g, o) in [(0, 2), (1, 1)] {
        let draft = regenerate(&w.backend.profile, s).unwrap();
        assert!(matches!(
            w.route(request(
                g,
                o,
                Command::Trial {
                    draft,
                    selection: s,
                    backup: "b".into()
                }
            ))
            .response,
            Response::Error(_)
        ));
    }
    assert!(w.backend.trial.is_empty());
    assert_eq!(w.backend.reads, 1);
}
#[test]
fn worker_enforces_user_wait_blocking_and_stage_order_without_ui() {
    let mut w = worker();
    read(&mut w);
    assert!(matches!(
        w.route(request(1, 2, Command::Held)).response,
        Response::Error(_)
    ));
    assert!(w.backend.stage.is_empty());
    assert!(matches!(
        w.route(request(1, 3, Command::Begin { backup: "b".into() }))
            .response,
        Response::Calibration(_)
    ));
    assert!(matches!(
        w.route(request(
            2,
            4,
            Command::Read {
                path: "other".into()
            }
        ))
        .response,
        Response::Error(_)
    ));
    assert_eq!(w.backend.reads, 1);
    assert!(matches!(
        w.route(request(1, 5, Command::Finish)).response,
        Response::Error(_)
    ));
    assert!(matches!(
        w.route(request(1, 6, Command::Held)).response,
        Response::Calibration(_)
    ));
    assert!(matches!(
        w.route(request(1, 7, Command::Held)).response,
        Response::Error(_)
    ));
    assert!(matches!(
        w.route(request(1, 8, Command::Cancel)).response,
        Response::Calibration(_)
    ));
    assert_eq!(w.backend.stage, vec!["begin", "held", "cancel"]);
}
#[test]
fn aborted_or_failed_session_requires_reconnect() {
    let mut w = worker();
    read(&mut w);
    w.route(request(1, 2, Command::Begin { backup: "b".into() }));
    w.route(request(1, 3, Command::Cancel));
    let s = Selection::default();
    let draft = w.backend.profile.clone();
    assert!(matches!(
        w.route(request(
            1,
            4,
            Command::Trial {
                draft,
                selection: s,
                backup: "b".into()
            }
        ))
        .response,
        Response::Error(_)
    ));
    assert!(w.backend.trial.is_empty());
    assert!(matches!(
        w.route(request(
            2,
            5,
            Command::Read {
                path: "device".into()
            }
        ))
        .response,
        Response::Read { .. }
    ));
}
#[test]
fn successful_finish_also_invalidates_baseline_until_explicit_read() {
    let mut w = worker();
    read(&mut w);
    w.route(request(1, 2, Command::Begin { backup: "b".into() }));
    w.route(request(1, 3, Command::Held));
    assert!(matches!(
        w.route(request(1, 4, Command::Finish)).response,
        Response::Finished(_)
    ));
    let s = Selection::default();
    let draft = w.backend.profile.clone();
    assert!(matches!(
        w.route(request(
            1,
            5,
            Command::Trial {
                draft,
                selection: s,
                backup: "b".into()
            }
        ))
        .response,
        Response::Error(_)
    ));
    assert!(w.backend.trial.is_empty());
}

#[test]
fn demo_thread_close_during_stage_waits_then_completes_cleanup() {
    use niz_gui::{
        controller::{Activation, Controller, ReadChoice},
        worker::spawn,
    };
    let ctx = eframe::egui::Context::default();
    let mut c = Controller::new(spawn(true, ctx));
    c.list().unwrap();
    let e = c
        .port
        .receiver
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    c.process(e);
    assert_eq!(c.devices[0].path, "demo://keyboard");
    c.read("demo://keyboard".into(), ReadChoice::Discard)
        .unwrap();
    let e = c
        .port
        .receiver
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    c.process(e);
    assert!(c.live);
    c.begin("unused-demo-directory".into(), Activation::Pointer)
        .unwrap();
    c.close();
    assert!(!c.ready_close);
    let e = c
        .port
        .receiver
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    c.process(e);
    if !c.ready_close {
        let e = c
            .port
            .receiver
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        c.process(e);
    }
    assert!(c.ready_close);
    assert!(!c.live);
    assert_eq!(
        c.calibration.as_ref().unwrap().phase,
        CalibrationPhase::Aborted
    );
    // Dropping this port closes its channel and joins the exact owned thread.
    drop(c);
}

#[test]
fn changing_request_must_name_the_attached_path_and_descriptor() {
    for variation in 0..3 {
        let mut w = worker();
        read(&mut w);
        let s = Selection {
            to_code: 43,
            ..Selection::default()
        };
        let draft = regenerate(&w.backend.profile, s).unwrap();
        let mut r = request(
            1,
            2,
            Command::Trial {
                draft,
                selection: s,
                backup: "backup".into(),
            },
        );
        match variation {
            0 => r.target = None,
            1 => r.target.as_mut().unwrap().path = "other-device".into(),
            _ => r.target.as_mut().unwrap().identity.product_id = 0x512a,
        }
        assert!(matches!(w.route(r).response, Response::Error(_)));
        assert!(w.backend.trial.is_empty());
    }
}
#[test]
fn calibration_request_with_other_path_cannot_enter_backend() {
    let mut w = worker();
    read(&mut w);
    let mut r = request(
        1,
        2,
        Command::Begin {
            backup: "backup".into(),
        },
    );
    r.target.as_mut().unwrap().path = "other-device".into();
    assert!(matches!(w.route(r).response, Response::Error(_)));
    assert!(w.backend.stage.is_empty());
}
