use crate::{
    editor::{Mode, Selection, exact_trial, regenerate},
    protocol::*,
};
use niz_core::Profile;
use niz_device::{CalibrationPhase, CalibrationStatus, DeviceDescriptor, LiveBaseline, Session};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
};
pub trait Backend {
    fn list(&mut self) -> Result<Vec<DeviceDescriptor>, Failure>;
    fn read(&mut self, path: &str) -> Result<(Profile, Arc<AtomicBool>), Failure>;
    fn baseline(&self) -> Option<&Profile>;
    fn trial(&mut self, selection: Selection, backup: &Path) -> Result<Response, Failure>;
    fn begin(&mut self, backup: &Path) -> Result<CalibrationStatus, Failure>;
    fn held(&mut self) -> Result<CalibrationStatus, Failure>;
    fn finish(&mut self) -> Result<CalibrationStatus, Failure>;
    fn cancel(&mut self) -> Result<CalibrationStatus, Failure>;
}
pub struct Worker<B: Backend> {
    pub backend: B,
    generation: u64,
    last_operation: u64,
    valid: bool,
    attached_target: Option<Target>,
    calibration: Option<CalibrationPhase>,
}
impl<B: Backend> Worker<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            generation: 0,
            last_operation: 0,
            valid: false,
            attached_target: None,
            calibration: None,
        }
    }
    pub fn route(&mut self, r: Request) -> Event {
        let result = self.execute(&r);
        let response = match result {
            Ok(v) => v,
            Err(e) => Response::Error(e),
        };
        if let Response::Calibration(s) | Response::Finished(s) = &response {
            self.calibration = Some(s.phase);
        }
        if let Response::Error(Failure {
            calibration: Some(s),
            ..
        }) = &response
        {
            self.calibration = Some(s.phase);
        }
        Event {
            ticket: r.ticket,
            response,
        }
    }
    fn execute(&mut self, r: &Request) -> Result<Response, Failure> {
        let fail = |s: &str| Failure::from(s.to_string());
        if r.ticket.operation <= self.last_operation {
            return Err(fail("Stale or replayed operation"));
        }
        self.last_operation = r.ticket.operation;
        let active = self.calibration.is_some_and(|p| {
            !matches!(
                p,
                CalibrationPhase::Complete | CalibrationPhase::Aborted | CalibrationPhase::Failed
            )
        });
        if active && !matches!(r.command, Command::Held | Command::Finish | Command::Cancel) {
            return Err(fail("Calibration owns the device, including user wait"));
        }
        if let Command::Read { path } = &r.command {
            if r.ticket.generation <= self.generation {
                return Err(fail("Read connection generation must advance"));
            }
            self.generation = r.ticket.generation;
            self.valid = false;
            let (profile, cancel) = self.backend.read(path)?;
            if r.target
                .as_ref()
                .is_some_and(|t| t.path != *path || t.identity != profile.identity)
            {
                return Err(fail("Read descriptor differs from the actual device"));
            }
            self.attached_target = Some(Target {
                path: path.clone(),
                product: r
                    .target
                    .as_ref()
                    .map(|t| t.product.clone())
                    .unwrap_or_else(|| "NIZ keyboard".into()),
                identity: profile.identity.clone(),
            });
            self.valid = true;
            self.calibration = None;
            return Ok(Response::Read { profile, cancel });
        }
        if r.ticket.generation != self.generation {
            return Err(fail("Stale connection generation"));
        }
        if matches!(
            r.command,
            Command::Trial { .. }
                | Command::Begin { .. }
                | Command::Held
                | Command::Finish
                | Command::Cancel
        ) && (r.target.is_none() || r.target != self.attached_target)
        {
            return Err(fail(
                "Changing request does not match the attached concrete device",
            ));
        }
        match &r.command {
            Command::List => Ok(Response::Devices(self.backend.list()?)),
            Command::Trial {
                draft,
                selection,
                backup,
            } => {
                if !self.valid {
                    return Err(fail("A fresh native baseline is required"));
                }
                let baseline = self
                    .backend
                    .baseline()
                    .ok_or_else(|| fail("No native baseline"))?;
                exact_trial(baseline, draft, *selection).map_err(Failure::from)?;
                self.valid = false;
                self.backend.trial(*selection, backup)
            }
            Command::Begin { backup } => {
                if !self.valid || active {
                    return Err(fail("A fresh native baseline is required"));
                }
                self.valid = false;
                Ok(Response::Calibration(self.backend.begin(backup)?))
            }
            Command::Held => {
                if self.calibration != Some(CalibrationPhase::AwaitingHeldKeys) {
                    return Err(fail("Held stage is not ready"));
                }
                Ok(Response::Calibration(self.backend.held()?))
            }
            Command::Finish => {
                if self.calibration != Some(CalibrationPhase::AwaitingFinish) {
                    return Err(fail("Finish stage is not ready"));
                }
                Ok(Response::Finished(self.backend.finish()?))
            }
            Command::Cancel => {
                if !matches!(
                    self.calibration,
                    Some(CalibrationPhase::AwaitingHeldKeys | CalibrationPhase::AwaitingFinish)
                ) {
                    return Err(fail("Safe cleanup is unavailable or already terminal"));
                }
                Ok(Response::Calibration(self.backend.cancel()?))
            }
            Command::Shutdown => Ok(Response::Shutdown),
            Command::Read { .. } => unreachable!(),
        }
    }
}
#[derive(Default)]
struct NativeBackend {
    session: Option<Session>,
    baseline: Option<LiveBaseline>,
}
impl NativeBackend {
    fn session(&mut self) -> Result<&mut Session, Failure> {
        self.session
            .as_mut()
            .ok_or_else(|| "No native Session".to_string().into())
    }
}
impl Backend for NativeBackend {
    fn list(&mut self) -> Result<Vec<DeviceDescriptor>, Failure> {
        niz_device::list_devices().map_err(Into::into)
    }
    fn read(&mut self, path: &str) -> Result<(Profile, Arc<AtomicBool>), Failure> {
        // Native Session is constructed only here, on explicit Read, on the dedicated thread.
        self.baseline = None;
        self.session = None;
        let mut session = Session::open(path)?;
        let baseline = session.read_baseline()?;
        let profile = baseline.profile().clone();
        let flag = session.cancellation_flag();
        self.session = Some(session);
        self.baseline = Some(baseline);
        Ok((profile, flag))
    }
    fn baseline(&self) -> Option<&Profile> {
        self.baseline.as_ref().map(LiveBaseline::profile)
    }
    fn trial(&mut self, s: Selection, backup: &Path) -> Result<Response, Failure> {
        let b = self
            .baseline
            .as_ref()
            .ok_or_else(|| Failure::from("No current native baseline".to_string()))?;
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| Failure::from("No native Session".to_string()))?;
        let result = match s.mode {
            Mode::Reassign => session.trial_single_key(b, s.layer * 66 + s.from, s.to_code, backup),
            Mode::Swap => session.trial_swap(
                b,
                s.layer * 66 + s.from,
                s.layer * 66 + s.to_position,
                backup,
            ),
        }?;
        Ok(Response::Applied {
            profile: result.profile,
            outcome: result.outcome,
            backup: result.backup_path,
            snapshot: result.snapshot,
        })
    }
    fn begin(&mut self, backup: &Path) -> Result<CalibrationStatus, Failure> {
        let b = self
            .baseline
            .as_ref()
            .ok_or_else(|| Failure::from("No current native baseline".to_string()))?;
        Ok(self
            .session
            .as_mut()
            .ok_or_else(|| Failure::from("No native Session".to_string()))?
            .begin_calibration(b, backup)?)
    }
    fn held(&mut self) -> Result<CalibrationStatus, Failure> {
        Ok(self.session()?.calibrate_held_keys()?)
    }
    fn finish(&mut self) -> Result<CalibrationStatus, Failure> {
        Ok(self.session()?.finish_calibration()?.status)
    }
    fn cancel(&mut self) -> Result<CalibrationStatus, Failure> {
        Ok(self.session()?.cancel_calibration()?)
    }
}
#[derive(Default)]
pub struct DemoBackend {
    profile: Option<Profile>,
    cancel: Arc<AtomicBool>,
    status: Option<CalibrationStatus>,
}
impl DemoBackend {
    fn stage(&mut self, phase: CalibrationPhase) -> Result<CalibrationStatus, Failure> {
        std::thread::sleep(std::time::Duration::from_millis(450));
        let mut s = self.status.clone().unwrap_or(CalibrationStatus {
            phase,
            failure_phase: None,
            backup: None,
            lock_may_have_been_sent: false,
            release_attempted: false,
            release_sent: false,
            release_completed: false,
            held_attempted: false,
            held_sent: false,
            held_completed: false,
            cancellation_requested: false,
            unlock: niz_device::UnlockOutcome::NotAttempted,
            settings_verified: false,
            lighting_verified: false,
            original_error: None,
            trace: vec![],
        });
        s.phase = phase;
        if self.cancel.load(Ordering::Acquire) {
            s.phase = CalibrationPhase::Aborted;
            s.cancellation_requested = true;
            s.unlock = niz_device::UnlockOutcome::Sent;
        }
        self.status = Some(s.clone());
        Ok(s)
    }
}
impl Backend for DemoBackend {
    fn list(&mut self) -> Result<Vec<DeviceDescriptor>, Failure> {
        Ok(vec![DeviceDescriptor {
            path: "demo://keyboard".into(),
            product: "Simulated Atom66".into(),
            identity: niz_core::DeviceIdentity {
                vendor_id: 0x0483,
                product_id: 0x502a,
                usage_page: 0x8c,
                usage: 1,
            },
        }])
    }
    fn read(&mut self, _: &str) -> Result<(Profile, Arc<AtomicBool>), Failure> {
        if self.profile.is_none() {
            let mut p = Profile::demo();
            p.firmware = "66EC(XRGB)BLe;V1.0.48;V1.0;".into();
            p.identity = niz_core::DeviceIdentity {
                vendor_id: 0x0483,
                product_id: 0x502a,
                usage_page: 0x8c,
                usage: 1,
            };
            self.profile = Some(p);
        }
        self.cancel = Arc::new(AtomicBool::new(false));
        self.status = None;
        Ok((self.profile.as_ref().unwrap().clone(), self.cancel.clone()))
    }
    fn baseline(&self) -> Option<&Profile> {
        self.profile.as_ref()
    }
    fn trial(&mut self, s: Selection, backup: &Path) -> Result<Response, Failure> {
        let p = self
            .profile
            .as_ref()
            .ok_or_else(|| Failure::from("Read the demo first".to_string()))?;
        let desired = regenerate(p, s).map_err(Failure::from)?;
        let backup = niz_device::save_backup(p, backup)?;
        let outcome = if p.records == desired.records {
            niz_device::ApplyOutcome::NoChange
        } else {
            niz_device::ApplyOutcome::SettingsAndLightingVerified
        };
        self.profile = Some(desired.clone());
        Ok(Response::Applied {
            profile: desired,
            outcome,
            backup,
            snapshot: None,
        })
    }
    fn begin(&mut self, _: &Path) -> Result<CalibrationStatus, Failure> {
        let mut s = self.stage(CalibrationPhase::AwaitingHeldKeys)?;
        s.release_attempted = true;
        s.release_sent = true;
        s.release_completed = true;
        self.status = Some(s.clone());
        Ok(s)
    }
    fn held(&mut self) -> Result<CalibrationStatus, Failure> {
        let mut s = self.stage(CalibrationPhase::AwaitingFinish)?;
        s.held_attempted = true;
        s.held_sent = true;
        s.held_completed = true;
        self.status = Some(s.clone());
        Ok(s)
    }
    fn finish(&mut self) -> Result<CalibrationStatus, Failure> {
        let mut s = self.stage(CalibrationPhase::Complete)?;
        s.unlock = niz_device::UnlockOutcome::Sent;
        s.settings_verified = true;
        s.lighting_verified = true;
        self.status = Some(s.clone());
        Ok(s)
    }
    fn cancel(&mut self) -> Result<CalibrationStatus, Failure> {
        self.cancel.store(true, Ordering::Release);
        self.stage(CalibrationPhase::Aborted)
    }
}
pub struct ChannelPort {
    sender: Option<mpsc::Sender<Request>>,
    pub receiver: mpsc::Receiver<Event>,
    join: Option<JoinHandle<()>>,
}
impl Port for ChannelPort {
    fn send(&mut self, r: Request) -> Result<(), String> {
        self.sender
            .as_ref()
            .ok_or("Worker closed")?
            .send(r)
            .map_err(|_| "Device worker stopped".into())
    }
}
impl Drop for ChannelPort {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}
pub fn spawn(demo: bool, ctx: eframe::egui::Context) -> ChannelPort {
    let (sender, requests) = mpsc::channel();
    let (events, receiver) = mpsc::channel();
    let join = std::thread::spawn(move || {
        // Two separate branches prevent demo enumeration, startup or reads from constructing HID.
        if demo {
            run(Worker::new(DemoBackend::default()), requests, events, ctx);
        } else {
            run(Worker::new(NativeBackend::default()), requests, events, ctx);
        }
    });
    ChannelPort {
        sender: Some(sender),
        receiver,
        join: Some(join),
    }
}
fn run<B: Backend>(
    mut worker: Worker<B>,
    requests: mpsc::Receiver<Request>,
    events: mpsc::Sender<Event>,
    ctx: eframe::egui::Context,
) {
    while let Ok(request) = requests.recv() {
        let event = worker.route(request);
        let quit = matches!(event.response, Response::Shutdown);
        if events.send(event).is_err() {
            break;
        }
        ctx.request_repaint();
        if quit {
            break;
        }
    }
}
