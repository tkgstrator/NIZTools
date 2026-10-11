use niz_device::CalibrationPhase;
use niz_gui::{
    controller::{Activation, Controller, ReadChoice},
    protocol::*,
    worker::{Backend, DemoBackend, Worker},
};
use std::sync::{Arc, atomic::AtomicBool};

#[derive(Default)]
struct CapturePort(Vec<Request>);
impl Port for CapturePort {
    fn send(&mut self, request: Request) -> Result<(), String> {
        self.0.push(request);
        Ok(())
    }
}
fn connected() -> Controller<CapturePort> {
    let mut c = Controller::new(CapturePort::default());
    c.read("demo://keyboard".into(), ReadChoice::Discard)
        .unwrap();
    let profile = niz_core::Profile::from_json(include_str!(
        "../../../docs/validation/fixtures/atom66-xrgb-ble-v1.0.48-settings.json"
    ))
    .unwrap();
    c.process(Event {
        ticket: c.pending.unwrap(),
        response: Response::Read {
            profile,
            cancel: Arc::new(AtomicBool::new(false)),
        },
    });
    c
}
// Catches a missing ownership classification/target, incorrect position, and non-pointer dispatch.
#[test]
fn held_controller_pins_target_and_rejects_keyboard_duplicate_and_close() {
    let mut c = connected();
    assert!(
        c.begin_held("backup".into(), 30, Activation::Keyboard)
            .is_err()
    );
    assert!(
        c.begin_held("backup".into(), 66, Activation::Pointer)
            .is_err()
    );
    assert_eq!(c.port.0.len(), 1);
    let target = c.attached_target.clone();
    c.begin_held("backup".into(), 30, Activation::Pointer)
        .unwrap();
    assert!(c.blocked() && c.calibration_active());
    assert_eq!(c.port.0.last().unwrap().target, target);
    assert!(matches!(
        c.port.0.last().unwrap().command,
        Command::BeginHeld { position: 30, .. }
    ));
    assert!(
        c.begin_held("backup".into(), 31, Activation::Pointer)
            .is_err()
    );
    assert!(
        c.read("demo://keyboard".into(), ReadChoice::Discard)
            .is_err()
    );
    assert!(c.read_raw_thresholds().is_err());
    c.close();
    assert!(!c.ready_close);
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
    let mut backend = DemoBackend::default();
    let target = Target::from(&backend.list().unwrap()[0]);
    let mut worker = Worker::new(backend);
    assert!(matches!(
        worker
            .route(request(
                1,
                Command::Read {
                    path: target.path.clone()
                },
                &target
            ))
            .response,
        Response::Read { .. }
    ));
    (worker, target)
}
// Catches routing held-only success back to AwaitingFinish, loss of comparison, or a second DD.
#[test]
fn held_worker_auto_finishes_and_preserves_complete_simulated_comparison() {
    let (mut w, target) = demo_worker();
    let Response::Calibration(prepared) = w
        .route(request(
            2,
            Command::BeginHeld {
                backup: "backup".into(),
                position: 30,
            },
            &target,
        ))
        .response
    else {
        panic!("missing preparation");
    };
    assert_eq!(prepared.phase, CalibrationPhase::AwaitingHeldKeys);
    assert!(!prepared.release_attempted && !prepared.release_sent);
    let held = prepared.held_key.unwrap();
    assert_eq!(held.position, 30);
    assert_eq!(held.before.as_ref().unwrap().len(), 66);
    assert!(held.after.is_none());
    let Response::Finished(finished) = w.route(request(3, Command::Held, &target)).response else {
        panic!("must automatically finish");
    };
    assert_eq!(finished.phase, CalibrationPhase::Complete);
    assert!(finished.held_completed && finished.settings_verified && finished.lighting_verified);
    let values = finished.held_key.unwrap();
    assert_eq!(values.before.as_ref().unwrap()[30], 30);
    assert_eq!(values.after.as_ref().unwrap()[30], 31);
    assert_eq!(values.after.as_ref().unwrap().len(), 66);
    assert!(matches!(
        w.route(request(3, Command::Held, &target)).response,
        Response::Error(_)
    ));
    assert!(matches!(
        w.route(request(4, Command::Held, &target)).response,
        Response::Error(_)
    ));
    assert!(matches!(
        w.route(request(5, Command::Finish, &target)).response,
        Response::Error(_)
    ));
}

#[test]
fn held_worker_rejects_foreign_target_and_position_without_consuming_baseline() {
    let (mut w, target) = demo_worker();
    let mut foreign = target.clone();
    foreign.path = "other".into();
    assert!(matches!(
        w.route(request(
            2,
            Command::BeginHeld {
                backup: "b".into(),
                position: 30
            },
            &foreign
        ))
        .response,
        Response::Error(_)
    ));
    assert!(matches!(
        w.route(request(
            3,
            Command::BeginHeld {
                backup: "b".into(),
                position: 66
            },
            &target
        ))
        .response,
        Response::Error(_)
    ));
    let mut stale = request(
        4,
        Command::BeginHeld {
            backup: "b".into(),
            position: 30,
        },
        &target,
    );
    stale.ticket.generation = 0;
    assert!(matches!(w.route(stale).response, Response::Error(_)));
    assert!(matches!(
        w.route(request(
            5,
            Command::BeginHeld {
                backup: "b".into(),
                position: 65
            },
            &target
        ))
        .response,
        Response::Calibration(_)
    ));
    assert!(matches!(
        w.route(request(6, Command::ReadRawThresholds, &target))
            .response,
        Response::Error(_)
    ));
    let Response::Calibration(cancelled) = w.route(request(7, Command::Cancel, &target)).response
    else {
        panic!("must cancel");
    };
    assert_eq!(cancelled.phase, CalibrationPhase::Aborted);
    assert!(cancelled.held_key.unwrap().after.is_none());
}
