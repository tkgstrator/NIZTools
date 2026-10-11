use niz_gui::{
    controller::{Activation, Controller, ReadChoice},
    protocol::*,
    worker::{Backend, DemoBackend, Worker},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Default)]
struct CapturePort(Vec<Request>);
impl Port for CapturePort {
    fn send(&mut self, r: Request) -> Result<(), String> {
        self.0.push(r);
        Ok(())
    }
}
fn profile() -> niz_core::Profile {
    niz_core::Profile::from_json(include_str!(
        "../../../docs/validation/fixtures/atom66-xrgb-ble-v1.0.48-settings.json"
    ))
    .unwrap()
}
fn connected() -> Controller<CapturePort> {
    let mut c = Controller::new(CapturePort::default());
    c.read("demo://keyboard".into(), ReadChoice::Discard)
        .unwrap();
    c.process(Event {
        ticket: c.pending.unwrap(),
        response: Response::Read {
            profile: profile(),
            cancel: Arc::new(AtomicBool::new(false)),
        },
    });
    c
}
fn request(operation: u64, command: Command, target: &Target) -> Request {
    Request {
        ticket: Ticket {
            generation: 1,
            operation,
        },
        command,
        target: Some(target.clone()),
    }
}
fn demo_worker() -> (Worker<DemoBackend>, Target) {
    let mut b = DemoBackend::default();
    let t = Target::from(&b.list().unwrap()[0]);
    let mut w = Worker::new(b);
    assert!(matches!(
        w.route(request(
            1,
            Command::Read {
                path: t.path.clone()
            },
            &t
        ))
        .response,
        Response::Read { .. }
    ));
    (w, t)
}
fn status() -> niz_device::SensitivityStatus {
    let (mut w, t) = demo_worker();
    match w
        .route(request(
            2,
            Command::AdjustSensitivity {
                backup: "backup".into(),
                coefficient: 24,
            },
            &t,
        ))
        .response
    {
        Response::Sensitivity(s) => s,
        other => panic!("{other:?}"),
    }
}
// Catches bypassing primary pointer/range/exact baseline and incorrect ownership/target dispatch.
#[test]
fn controller_sensitivity_requires_pointer_exact_fresh_target_and_exclusive_ownership() {
    let mut c = connected();
    assert!(
        c.adjust_sensitivity("backup".into(), 24, Activation::Keyboard)
            .is_err()
    );
    for coefficient in [0, 9, 41, 255] {
        assert!(
            c.adjust_sensitivity("backup".into(), coefficient, Activation::Pointer)
                .is_err()
        );
    }
    assert_eq!(c.port.0.len(), 1);
    let target = c.attached_target.clone();
    c.adjust_sensitivity("backup".into(), 10, Activation::Pointer)
        .unwrap();
    assert!(c.blocked());
    assert!(!c.calibration_active());
    assert!(!c.live);
    assert_eq!(c.port.0.last().unwrap().target, target);
    assert!(matches!(
        c.port.0.last().unwrap().command,
        Command::AdjustSensitivity {
            coefficient: 10,
            ..
        }
    ));
    assert!(
        c.adjust_sensitivity("backup".into(), 40, Activation::Pointer)
            .is_err()
    );
    assert!(
        c.read("demo://keyboard".into(), ReadChoice::Discard)
            .is_err()
    );
    assert!(c.read_raw_thresholds().is_err());
    c.close();
    assert!(c.cancel.load(Ordering::Acquire));
    assert!(!c.ready_close);
    assert_eq!(
        c.port.0.len(),
        2,
        "closing must not queue a second device operation"
    );
    c.process(Event {
        ticket: c.pending.unwrap(),
        response: Response::Sensitivity(status()),
    });
    assert!(c.ready_close && c.sensitivity.is_some());
}
#[test]
fn controller_sensitivity_rejects_unsupported_or_detached_baseline() {
    for case in 0..5 {
        let mut c = connected();
        match case {
            0 => c.editor.as_mut().unwrap().baseline.firmware = "other".into(),
            1 => c.editor.as_mut().unwrap().baseline.identity.usage = 2,
            2 => c.selected_path = Some("other".into()),
            3 => c.live = false,
            _ => {
                c.begin_held("b".into(), 30, Activation::Pointer).unwrap();
            }
        }
        assert!(
            c.adjust_sensitivity("backup".into(), 24, Activation::Pointer)
                .is_err(),
            "case {case}"
        );
    }
}
// Catches losing partial observations, attribution to a later source, or accepting an unrelated reply.
#[test]
fn controller_retains_failure_observation_then_invalidates_it_on_read_and_switch() {
    let mut c = connected();
    c.adjust_sensitivity("b".into(), 24, Activation::Pointer)
        .unwrap();
    let mut s = status();
    s.after = None;
    s.settings_verified = false;
    let mut e = Failure::from("after snapshot incomplete".to_string());
    e.sensitivity = Some(Box::new(s));
    e.began_write = true;
    c.process(Event {
        ticket: c.pending.unwrap(),
        response: Response::Error(e),
    });
    assert_eq!(
        c.sensitivity
            .as_ref()
            .unwrap()
            .before
            .as_ref()
            .unwrap()
            .len(),
        66
    );
    assert!(c.sensitivity.as_ref().unwrap().after.is_none());
    assert!(c.failure.as_ref().unwrap().began_write && !c.live);
    c.read("demo://keyboard".into(), ReadChoice::Discard)
        .unwrap();
    assert!(c.sensitivity.is_none());
    c.process(Event {
        ticket: c.pending.unwrap(),
        response: Response::Read {
            profile: profile(),
            cancel: Arc::new(AtomicBool::new(false)),
        },
    });
    c.adjust_sensitivity("b".into(), 24, Activation::Pointer)
        .unwrap();
    c.process(Event {
        ticket: c.pending.unwrap(),
        response: Response::Sensitivity(status()),
    });
    let mut d = DemoBackend::default().list().unwrap().remove(0);
    d.path = "other".into();
    c.devices = vec![d];
    c.select_device("other".into()).unwrap();
    assert!(c.sensitivity.is_none());
}
#[test]
fn controller_unrelated_success_does_not_publish_sensitivity() {
    let mut c = connected();
    c.list().unwrap();
    c.process(Event {
        ticket: c.pending.unwrap(),
        response: Response::Sensitivity(status()),
    });
    assert!(c.sensitivity.is_none() && c.failure.is_some() && !c.live);
}
// Catches stale/replayed/foreign/range bypass, consuming a baseline on rejection, and repeat adjustment.
#[test]
fn worker_sensitivity_guards_requests_and_consumes_eligibility_exactly_once() {
    let (mut w, t) = demo_worker();
    for (operation, coefficient) in [(2, 9), (3, 41)] {
        assert!(matches!(
            w.route(request(
                operation,
                Command::AdjustSensitivity {
                    backup: "b".into(),
                    coefficient
                },
                &t
            ))
            .response,
            Response::Error(_)
        ));
    }
    let mut bad = request(
        4,
        Command::AdjustSensitivity {
            backup: "b".into(),
            coefficient: 24,
        },
        &t,
    );
    bad.target.as_mut().unwrap().path = "other".into();
    assert!(matches!(w.route(bad).response, Response::Error(_)));
    let mut stale = request(
        5,
        Command::AdjustSensitivity {
            backup: "b".into(),
            coefficient: 24,
        },
        &t,
    );
    stale.ticket.generation = 0;
    assert!(matches!(w.route(stale).response, Response::Error(_)));
    let Response::Sensitivity(s) = w
        .route(request(
            6,
            Command::AdjustSensitivity {
                backup: "b".into(),
                coefficient: 24,
            },
            &t,
        ))
        .response
    else {
        panic!("adjustment should succeed")
    };
    assert_eq!(s.coefficient, 24);
    assert_eq!(s.before.as_ref().unwrap().len(), 66);
    assert_eq!(s.after.as_ref().unwrap().len(), 66);
    assert!(s.attempted && s.sent && s.settings_verified && s.lighting_verified);
    for operation in [6, 7] {
        assert!(matches!(
            w.route(request(
                operation,
                Command::AdjustSensitivity {
                    backup: "b".into(),
                    coefficient: 24
                },
                &t
            ))
            .response,
            Response::Error(_)
        ));
    }
    assert!(matches!(
        w.route(request(8, Command::ReadRawThresholds, &t)).response,
        Response::Error(_)
    ));
}
#[test]
fn worker_calibration_wait_owns_device_against_sensitivity() {
    let (mut w, t) = demo_worker();
    assert!(matches!(
        w.route(request(
            2,
            Command::BeginHeld {
                backup: "b".into(),
                position: 30
            },
            &t
        ))
        .response,
        Response::Calibration(_)
    ));
    assert!(matches!(
        w.route(request(
            3,
            Command::AdjustSensitivity {
                backup: "b".into(),
                coefficient: 24
            },
            &t
        ))
        .response,
        Response::Error(_)
    ));
    assert!(matches!(
        w.route(request(4, Command::Cancel, &t)).response,
        Response::Calibration(_)
    ));
}

#[test]
fn controller_rejects_mismatched_coefficient_and_incomplete_success_but_retains_observations() {
    for case in 0..2 {
        let mut c = connected();
        c.adjust_sensitivity("b".into(), 24, Activation::Pointer)
            .unwrap();
        let mut s = status();
        if case == 0 {
            s.coefficient = 25;
        } else {
            s.after = Some(vec![12; 65]);
        }
        c.process(Event {
            ticket: c.pending.unwrap(),
            response: Response::Sensitivity(s),
        });
        assert!(
            c.failure.is_some(),
            "case {case}: mismatched or incomplete reply is not success"
        );
        assert!(c.sensitivity.is_some() && !c.live);
    }
}
#[test]
fn reconcile_changed_descriptor_discards_old_sensitivity_source() {
    let mut c = connected();
    c.adjust_sensitivity("b".into(), 24, Activation::Pointer)
        .unwrap();
    c.process(Event {
        ticket: c.pending.unwrap(),
        response: Response::Sensitivity(status()),
    });
    c.list().unwrap();
    let mut d = DemoBackend::default().list().unwrap().remove(0);
    d.identity.usage = 2;
    c.process(Event {
        ticket: c.pending.unwrap(),
        response: Response::Devices(vec![d]),
    });
    assert!(c.sensitivity.is_none());
}

#[test]
fn worker_failure_keeps_complete_before_and_requires_new_read_before_retry() {
    let mut b = DemoBackend::default();
    let t = Target::from(&b.list().unwrap()[0]);
    let mut w = Worker::new(b);
    let Response::Read { cancel, .. } = w
        .route(request(
            1,
            Command::Read {
                path: t.path.clone(),
            },
            &t,
        ))
        .response
    else {
        panic!("read")
    };
    cancel.store(true, Ordering::Release);
    let Response::Error(f) = w
        .route(request(
            2,
            Command::AdjustSensitivity {
                backup: "b".into(),
                coefficient: 24,
            },
            &t,
        ))
        .response
    else {
        panic!("cancelled adjustment")
    };
    let s = f.sensitivity.unwrap();
    assert_eq!(s.before.unwrap().len(), 66);
    assert!(s.after.is_none() && !s.attempted && !s.sent && s.cancellation_requested);
    cancel.store(false, Ordering::Release);
    assert!(matches!(
        w.route(request(
            3,
            Command::AdjustSensitivity {
                backup: "b".into(),
                coefficient: 24
            },
            &t
        ))
        .response,
        Response::Error(_)
    ));
    let mut r = request(
        4,
        Command::Read {
            path: t.path.clone(),
        },
        &t,
    );
    r.ticket.generation = 2;
    assert!(matches!(w.route(r).response, Response::Read { .. }));
    let mut r = request(
        5,
        Command::AdjustSensitivity {
            backup: "b".into(),
            coefficient: 40,
        },
        &t,
    );
    r.ticket.generation = 2;
    let Response::Sensitivity(s) = w.route(r).response else {
        panic!("fresh adjustment")
    };
    assert_eq!(s.coefficient, 40);
    assert_eq!(s.after.unwrap()[0], 23);
}
