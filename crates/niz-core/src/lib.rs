//! Offline codec and lossless ATOM66 configuration profiles.
//! A valid profile does not establish hardware write qualification.
mod error;
mod keys;
mod legacy;
mod profile;
mod protocol;
mod raw_thresholds;

pub use error::{Error, Result};
pub use keys::{KEY_ROWS, KEY_WIDTHS, key_name, parse_key};
pub use legacy::{WINDOWS_PROFILE_FIRMWARE, import_windows_profile};
pub use profile::{DeviceIdentity, Profile};
pub use protocol::{
    Definition, MAX_FILE_SIZE, MAX_REPORTS, command, decode_definition, encode_definition,
};
pub use raw_thresholds::{
    CalibrationValueDecoder, RawThresholdDecoder, decode_calibration_values, decode_raw_thresholds,
};

/// Logical HID payload, without hidapi's outgoing report-ID prefix.
pub type Report = [u8; 64];
