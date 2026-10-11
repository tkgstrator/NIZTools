use niz_core::{DeviceIdentity, Profile, Report};
use niz_device::{CalibrationStatus, DeviceDescriptor, RawThresholds};
use niz_gui::{
    editor::Selection,
    protocol::*,
    worker::{Backend, DemoBackend, Worker},
};
use std::{
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};
struct Fake {
    profile: Profile,
    raw_calls: usize,
    behavior: u8,
}
fn profile() -> Profile {
    let mut p = Profile::demo();
    p.identity = DeviceIdentity {
        vendor_id: 0x0483,
        product_id: 0x502a,
        usage_page: 0x8c,
        usage: 1,
    };
    p.firmware = "66EC(XRGB)BLe;V1.0.48;V1.0;".into();
    p
}
impl Backend for Fake {
    fn list(&mut self) -> Result<Vec<DeviceDescriptor>, Failure> {
        Ok(vec![])
    }
    fn read(&mut self, _: &str) -> Result<(Profile, Arc<AtomicBool>), Failure> {
        Ok((self.profile.clone(), Arc::new(AtomicBool::new(false))))
    }
    fn baseline(&self) -> Option<&Profile> {
        Some(&self.profile)
    }
    fn read_raw_thresholds(&mut self) -> Result<(RawThresholds, RawThresholdSource), Failure> {
        self.raw_calls += 1;
        if self.behavior == 1 {
            return Err("partial receive failed".to_string().into());
        }
        let p = profile();
        let rows: Vec<Report> = (0..11)
            .map(|row| {
                let mut r = [0; 64];
                r[1] = 0xdc;
                r[2] = row + 1;
                r[3] = 1;
                for col in 0..6 {
                    r[4 + col] = row * 6 + col as u8;
                }
                r
            })
            .collect();
        let mut raw = RawThresholds::from_reports(p.identity, p.firmware, rows).unwrap();
        if self.behavior == 2 {
            raw.firmware = "wrong".into();
        }
        if self.behavior == 3 {
            raw.identity.product_id = 0x512a;
        }
        Ok((raw, RawThresholdSource::Device))
    }
    fn adjust_sensitivity(
        &mut self,
        _: &Path,
        _: u8,
    ) -> Result<niz_device::SensitivityStatus, Failure> {
        panic!("legacy fake does not support sensitivity")
    }
    fn trial(&mut self, _: Selection, _: &Path) -> Result<Response, Failure> {
        panic!("no threshold write")
    }
    fn begin(&mut self, _: &Path) -> Result<CalibrationStatus, Failure> {
        panic!("no calibration")
    }
    fn begin_held(&mut self, _: &Path, _: usize) -> Result<CalibrationStatus, Failure> {
        panic!("legacy fake does not support held-only calibration")
    }
    fn held(&mut self) -> Result<CalibrationStatus, Failure> {
        panic!("no calibration")
    }
    fn finish(&mut self) -> Result<CalibrationStatus, Failure> {
        panic!("no calibration")
    }
    fn cancel(&mut self) -> Result<CalibrationStatus, Failure> {
        panic!("no speculative cleanup")
    }
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
            product: "NIZ".into(),
            identity: profile().identity,
        }),
    }
}
fn worker() -> Worker<Fake> {
    let mut w = Worker::new(Fake {
        profile: profile(),
        raw_calls: 0,
        behavior: 0,
    });
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
    w
}
#[test]
fn raw_worker_calls_one_reader_and_returns_the_owned_target() {
    let mut w = worker();
    let r = w.route(request(1, 2, Command::ReadRawThresholds));
    match r.response {
        Response::RawThresholds {
            snapshot,
            target,
            source,
        } => {
            assert_eq!(snapshot.value_for_position(30), Some(13));
            assert_eq!(snapshot.value_for_position(12), Some(60));
            assert_eq!(target.path, "device");
            assert_eq!(source, RawThresholdSource::Device)
        }
        other => panic!("{other:?}"),
    };
    assert_eq!(w.backend.raw_calls, 1);
    assert!(matches!(
        w.route(request(1, 2, Command::ReadRawThresholds)).response,
        Response::Error(_)
    ));
    assert_eq!(w.backend.raw_calls, 1);
}
#[test]
fn wrong_ticket_target_and_unsupported_baseline_never_enter_raw_backend() {
    for case in 0..7 {
        let mut w = worker();
        let mut r = request(1, 2, Command::ReadRawThresholds);
        match case {
            0 => r.ticket.generation = 0,
            1 => r.target = None,
            2 => r.target.as_mut().unwrap().path = "other".into(),
            3 => w.backend.profile.identity.product_id = 0x512a,
            4 => w.backend.profile.firmware = "wrong".into(),
            5 => r.target.as_mut().unwrap().product = "changed".into(),
            _ => r.ticket.operation = 1,
        }
        assert!(
            matches!(w.route(r).response, Response::Error(_)),
            "case {case}"
        );
        assert_eq!(w.backend.raw_calls, 0);
    }
}
#[test]
fn raw_io_or_metadata_failure_poison_worker_until_explicit_baseline_read() {
    for behavior in 1..=3 {
        let mut w = worker();
        w.backend.behavior = behavior;
        match w.route(request(1, 2, Command::ReadRawThresholds)).response {
            Response::Error(error) => {
                if behavior >= 2 {
                    assert_eq!(error.raw_reports.len(), 11);
                }
            }
            other => panic!("{other:?}"),
        };
        w.backend.behavior = 0;
        assert!(matches!(
            w.route(request(1, 3, Command::ReadRawThresholds)).response,
            Response::Error(_)
        ));
        assert_eq!(w.backend.raw_calls, 1);
        assert!(matches!(
            w.route(request(
                2,
                4,
                Command::Read {
                    path: "device".into()
                }
            ))
            .response,
            Response::Read { .. }
        ));
        assert!(matches!(
            w.route(request(2, 5, Command::ReadRawThresholds)).response,
            Response::RawThresholds { .. }
        ));
        assert_eq!(w.backend.raw_calls, 2);
    }
}
#[test]
fn demo_raw_reader_is_simulated_and_calibration_ownership_blocks_it() {
    let mut w = Worker::new(DemoBackend::default());
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
    match w.route(request(1, 2, Command::ReadRawThresholds)).response {
        Response::RawThresholds {
            snapshot, source, ..
        } => {
            assert_eq!(source, RawThresholdSource::Simulation);
            assert_eq!(snapshot.value_for_position(31), Some(14));
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        w.route(request(1, 3, Command::Begin { backup: "b".into() }))
            .response,
        Response::Calibration(_)
    ));
    assert!(matches!(
        w.route(request(1, 4, Command::ReadRawThresholds)).response,
        Response::Error(_)
    ));
    assert!(matches!(
        w.route(request(1, 5, Command::Cancel)).response,
        Response::Calibration(_)
    ));
}
