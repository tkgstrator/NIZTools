mod backup;
mod calibration;
pub use backup::SnapshotPaths;
pub use calibration::{
    CalibrationPhase, CalibrationResult, CalibrationStatus, PacketTrace, TraceDirection,
    UnlockOutcome,
};
mod session;
mod transport;
pub use backup::{load_profile, save_backup};
pub use session::{ApplyOutcome, ApplyResult, LiveBaseline, Session};
pub use transport::{DeviceDescriptor, list_devices};
#[derive(Debug)]
pub struct Error {
    pub message: String,
    pub raw_reports: Vec<Vec<u8>>,
    pub began_write: bool,
    pub calibration: Option<Box<CalibrationStatus>>,
    pub backup_path: Option<std::path::PathBuf>,
}
impl Error {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            raw_reports: vec![],
            began_write: false,
            backup_path: None,
            calibration: None,
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for Error {}
impl From<niz_core::Error> for Error {
    fn from(e: niz_core::Error) -> Self {
        Self::new(e.to_string())
    }
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::new(e.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;
#[cfg(test)]
mod event_tests;
#[cfg(test)]
mod tests;
