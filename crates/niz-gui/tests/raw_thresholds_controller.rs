use niz_core::{DeviceIdentity, Profile, Report};
use niz_device::RawThresholds;
use niz_gui::{
    controller::{Activation, Controller, ReadChoice},
    protocol::*,
};
use std::sync::{Arc, atomic::AtomicBool};
#[derive(Default)]
struct Capture {
    requests: Vec<Request>,
}
impl Port for Capture {
    fn send(&mut self, r: Request) -> Result<(), String> {
        self.requests.push(r);
        Ok(())
    }
}
fn profile() -> Profile {
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
fn data() -> RawThresholds {
    let p = profile();
    let reports: Vec<Report> = (0..11)
        .map(|row| {
            let mut r = [0; 64];
            r[1] = 0xdc;
            r[2] = row + 1;
            r[3] = 1;
            for col in 0..6 {
                r[4 + col] = (row as usize * 6 + col) as u8;
            }
            r
        })
        .collect();
    RawThresholds::from_reports(p.identity, p.firmware, reports).unwrap()
}
fn connected() -> Controller<Capture> {
    let mut c = Controller::new(Capture::default());
    c.read("device".into(), ReadChoice::Discard).unwrap();
    c.process(Event {
        ticket: c.pending.unwrap(),
        response: Response::Read {
            profile: profile(),
            cancel: Arc::new(AtomicBool::new(false)),
        },
    });
    c
}
fn raw_response(c: &Controller<Capture>) -> Response {
    Response::RawThresholds {
        snapshot: data(),
        target: c.attached_target.clone().unwrap(),
        source: RawThresholdSource::Device,
    }
}
fn install(c: &mut Controller<Capture>) {
    c.read_raw_thresholds().unwrap();
    let response = raw_response(c);
    c.process(Event {
        ticket: c.pending.unwrap(),
        response,
    });
    assert!(c.raw_thresholds().is_some());
}
#[test]
fn manual_raw_read_binds_target_and_preserves_editor_history() {
    let mut c = connected();
    let e = c.editor.as_mut().unwrap();
    e.choose_output(44);
    e.edit().unwrap();
    let before = (
        e.draft.records.clone(),
        e.baseline.records.clone(),
        e.selection,
        e.can_undo(),
        e.can_redo(),
    );
    let generation = c.generation;
    let target = c.attached_target.clone();
    let count = c.port.requests.len();
    assert!(c.raw_thresholds().is_none());
    c.read_raw_thresholds().unwrap();
    assert!(matches!(
        c.port.requests.last().unwrap().command,
        Command::ReadRawThresholds
    ));
    assert_eq!(c.port.requests.last().unwrap().target, target);
    assert_eq!(c.port.requests.len(), count + 1);
    assert!(c.read_raw_thresholds().is_err());
    assert_eq!(c.port.requests.len(), count + 1);
    let response = raw_response(&c);
    c.process(Event {
        ticket: c.pending.unwrap(),
        response,
    });
    let raw = c.raw_thresholds().unwrap();
    assert_eq!(raw.snapshot.value_for_position(30), Some(13));
    assert_eq!(raw.snapshot.value_for_position(12), Some(60));
    assert_eq!(raw.source, RawThresholdSource::Device);
    assert_eq!(raw.generation, generation);
    assert_eq!(Some(raw.target.clone()), target);
    let e = c.editor.as_ref().unwrap();
    assert_eq!(
        (
            e.draft.records.clone(),
            e.baseline.records.clone(),
            e.selection,
            e.can_undo(),
            e.can_redo()
        ),
        before
    );
    assert!(c.live);
}
#[test]
fn stale_raw_tickets_and_duplicate_completion_cannot_replace_snapshot() {
    let mut c = connected();
    c.read_raw_thresholds().unwrap();
    let ticket = c.pending.unwrap();
    for stale in [
        Ticket {
            generation: ticket.generation + 1,
            ..ticket
        },
        Ticket {
            operation: ticket.operation + 1,
            ..ticket
        },
    ] {
        let response = raw_response(&c);
        c.process(Event {
            ticket: stale,
            response,
        });
        assert!(c.raw_thresholds().is_none());
        assert_eq!(c.pending, Some(ticket));
    }
    let response = raw_response(&c);
    c.process(Event { ticket, response });
    let at = c.raw_thresholds().unwrap().read_at;
    let response = raw_response(&c);
    c.process(Event { ticket, response });
    assert_eq!(c.raw_thresholds().unwrap().read_at, at);
}
#[test]
fn raw_reply_rejects_wrong_metadata_target_source_and_response_command() {
    for case in 0..7 {
        let mut c = connected();
        c.read_raw_thresholds().unwrap();
        let ticket = c.pending.unwrap();
        let mut snapshot = data();
        let mut target = c.attached_target.clone().unwrap();
        match case {
            0 => snapshot.firmware = "wrong".into(),
            1 => snapshot.identity.product_id = 0x512a,
            2 => target.path = "other".into(),
            3 => c.editor.as_mut().unwrap().baseline.records[4][0][63] = 1,
            4 => c.selected_path = Some("other".into()),
            5 => c.live = false,
            _ => {}
        }
        let response = if case == 6 {
            Response::Read {
                profile: profile(),
                cancel: Arc::new(AtomicBool::new(false)),
            }
        } else {
            Response::RawThresholds {
                snapshot,
                target,
                source: RawThresholdSource::Device,
            }
        };
        c.process(Event { ticket, response });
        assert!(c.raw_thresholds().is_none(), "case {case}");
        assert!(!c.live, "case {case}");
        assert!(c.pending.is_none());
        if case <= 1 {
            assert_eq!(c.failure.as_ref().unwrap().raw_reports.len(), 11);
        }
    }
    let mut c = connected();
    c.list().unwrap();
    let response = raw_response(&c);
    c.process(Event {
        ticket: c.pending.unwrap(),
        response,
    });
    assert!(c.raw_thresholds().is_none());
    assert!(!c.live);
    assert_eq!(c.failure.as_ref().unwrap().raw_reports.len(), 11);
}
#[test]
fn raw_snapshot_invalidates_on_connection_source_write_error_and_close() {
    for case in 0..7 {
        let mut c = connected();
        install(&mut c);
        match case {
            0 => {
                c.read("device".into(), ReadChoice::Retain).unwrap();
            }
            1 => {
                c.load(profile(), true).unwrap();
            }
            2 => {
                c.editor.as_mut().unwrap().choose_output(44);
                c.edit().unwrap();
                c.trial("backup".into()).unwrap();
            }
            3 => {
                c.begin("backup".into(), Activation::Pointer).unwrap();
            }
            4 => c.close(),
            5 => {
                c.read_raw_thresholds().unwrap();
                c.process(Event {
                    ticket: c.pending.unwrap(),
                    response: Response::Error("failed raw read".to_string().into()),
                });
                assert!(!c.live);
            }
            _ => {
                c.list().unwrap();
                c.process(Event {
                    ticket: c.pending.unwrap(),
                    response: Response::Devices(vec![]),
                });
            }
        }
        assert!(c.raw_thresholds().is_none(), "case {case}");
    }
}
#[test]
fn raw_snapshot_accessor_never_exposes_mutated_target_or_baseline() {
    for case in 0..5 {
        let mut c = connected();
        install(&mut c);
        match case {
            0 => c.generation += 1,
            1 => c.attached_target.as_mut().unwrap().path = "other".into(),
            2 => c.selected_path = Some("other".into()),
            3 => c.editor.as_mut().unwrap().baseline.firmware = "wrong".into(),
            _ => c.live = false,
        }
        assert!(c.raw_thresholds().is_none());
    }
}
#[test]
fn unsupported_offline_busy_and_close_states_never_dispatch_raw_read() {
    for case in 0..6 {
        let mut c = connected();
        match case {
            0 => c.live = false,
            1 => c.editor.as_mut().unwrap().baseline.identity.product_id = 0x512a,
            2 => c.editor.as_mut().unwrap().baseline.firmware = "wrong".into(),
            3 => c.selected_path = Some("other".into()),
            4 => {
                c.list().unwrap();
            }
            _ => c.close_requested = true,
        }
        let count = c.port.requests.len();
        assert!(c.read_raw_thresholds().is_err());
        assert_eq!(c.port.requests.len(), count);
    }
    let mut c = connected();
    c.begin("b".into(), Activation::Pointer).unwrap();
    let count = c.port.requests.len();
    assert!(c.read_raw_thresholds().is_err());
    assert_eq!(c.port.requests.len(), count);
}
