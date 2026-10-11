use crate::editor::Selection;
use niz_core::Profile;
use niz_device::{
    ApplyOutcome, CalibrationStatus, DeviceDescriptor, RawThresholds, SensitivityStatus,
    SnapshotPaths,
};
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {
    pub generation: u64,
    pub operation: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub path: String,
    pub product: String,
    pub identity: niz_core::DeviceIdentity,
}
impl From<&DeviceDescriptor> for Target {
    fn from(d: &DeviceDescriptor) -> Self {
        Self {
            path: d.path.clone(),
            product: d.product.clone(),
            identity: d.identity.clone(),
        }
    }
}
#[derive(Clone, Debug)]
pub enum Command {
    List,
    Read {
        path: String,
    },
    ReadRawThresholds,
    AdjustSensitivity {
        backup: PathBuf,
        coefficient: u8,
    },
    Trial {
        draft: Profile,
        selection: Selection,
        backup: PathBuf,
    },
    Begin {
        backup: PathBuf,
    },
    BeginHeld {
        backup: PathBuf,
        position: usize,
    },
    Held,
    Finish,
    Cancel,
    Shutdown,
}
#[derive(Clone, Debug)]
pub struct Request {
    pub ticket: Ticket,
    pub command: Command,
    pub target: Option<Target>,
}
#[derive(Debug)]
pub struct Failure {
    pub message: String,
    pub calibration: Option<Box<CalibrationStatus>>,
    pub sensitivity: Option<Box<SensitivityStatus>>,
    pub backup: Option<PathBuf>,
    pub began_write: bool,
    pub raw_reports: Vec<Vec<u8>>,
}
impl From<niz_device::Error> for Failure {
    fn from(e: niz_device::Error) -> Self {
        Self {
            message: e.message,
            calibration: e.calibration,
            sensitivity: e.sensitivity,
            backup: e.backup_path,
            began_write: e.began_write,
            raw_reports: e.raw_reports,
        }
    }
}
impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self {
            message,
            calibration: None,
            sensitivity: None,
            backup: None,
            began_write: false,
            raw_reports: vec![],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawThresholdSource {
    Device,
    Simulation,
}
#[derive(Debug)]
pub enum Response {
    Devices(Vec<DeviceDescriptor>),
    Read {
        profile: Profile,
        cancel: Arc<AtomicBool>,
    },
    RawThresholds {
        snapshot: RawThresholds,
        target: Target,
        source: RawThresholdSource,
    },
    Applied {
        profile: Profile,
        outcome: ApplyOutcome,
        backup: PathBuf,
        snapshot: Option<SnapshotPaths>,
    },
    Calibration(CalibrationStatus),
    Sensitivity(SensitivityStatus),
    Finished(CalibrationStatus),
    Error(Failure),
    Shutdown,
}
#[derive(Debug)]
pub struct Event {
    pub ticket: Ticket,
    pub response: Response,
}
pub trait Port {
    fn send(&mut self, request: Request) -> Result<(), String>;
}
