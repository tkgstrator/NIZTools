use crate::{Error, Result};
use niz_core::{DeviceIdentity, Report, decode_calibration_values, decode_raw_thresholds};

/// Eligibility for exact-image DC mode-1 and mode-2 table reads.
/// This does not qualify threshold setters, calibration writes, or other modes.
pub fn supports_raw_thresholds(identity: &DeviceIdentity, firmware: &str) -> bool {
    identity.vendor_id == 0x0483
        && identity.product_id == 0x502a
        && identity.usage_page == 0x008c
        && identity.usage == 1
        && firmware == crate::session::EXPERIMENTAL_FIRMWARE
}

/// A complete snapshot of the firmware's RAM R actuation threshold table.
/// Values are raw firmware signal units, not press travel, millimeters, live ADC
/// samples, or a guarantee of the active threshold in every scanning mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawThresholds {
    pub identity: DeviceIdentity,
    pub firmware: String,
    values: [u8; 66],
    raw_reports: Vec<Report>,
}
impl RawThresholds {
    /// Validate and decode offline or synthetic evidence. This pure constructor
    /// proves packet shape and the exact metadata pin, not a live device origin.
    pub fn from_reports(
        identity: DeviceIdentity,
        firmware: String,
        raw_reports: Vec<Report>,
    ) -> Result<Self> {
        let parsed = (|| {
            if !supports_raw_thresholds(&identity, &firmware) {
                return Err(Error::new(
                    "raw thresholds require exact 0483:502a 008c:1 firmware 66EC(XRGB)BLe;V1.0.48;V1.0;",
                ));
            }
            Ok(decode_raw_thresholds(&raw_reports)?)
        })();
        let values = parsed.map_err(|mut error: Error| {
            error.raw_reports = raw_reports.iter().map(|report| report.to_vec()).collect();
            error
        })?;
        Ok(Self {
            identity,
            firmware,
            values,
            raw_reports,
        })
    }

    /// Raw value for the official 0-based physical key position (0 through 65).
    pub fn value_for_position(&self, position: usize) -> Option<u8> {
        self.values.get(position).copied()
    }

    pub fn values(&self) -> &[u8; 66] {
        &self.values
    }

    /// Eleven original DC rows, including their opaque tail bytes.
    pub fn raw_reports(&self) -> &[Report] {
        &self.raw_reports
    }
}

/// Complete snapshot of the pinned firmware's RAM T per-key calibration table.
/// These values can seed scanner thresholds, but this read does not directly
/// inspect flash and cannot independently prove power-cycle persistence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalibrationValues {
    pub identity: DeviceIdentity,
    pub firmware: String,
    values: [u8; 66],
    raw_reports: Vec<Report>,
}
impl CalibrationValues {
    /// Validate exact target metadata and decode eleven DC mode-2 rows.
    pub fn from_reports(
        identity: DeviceIdentity,
        firmware: String,
        raw_reports: Vec<Report>,
    ) -> Result<Self> {
        let parsed = (|| {
            if !supports_raw_thresholds(&identity, &firmware) {
                return Err(Error::new(
                    "calibration values require exact 0483:502a 008c:1 firmware 66EC(XRGB)BLe;V1.0.48;V1.0;",
                ));
            }
            Ok(decode_calibration_values(&raw_reports)?)
        })();
        let values = parsed.map_err(|mut error: Error| {
            error.raw_reports = raw_reports.iter().map(|report| report.to_vec()).collect();
            error
        })?;
        Ok(Self {
            identity,
            firmware,
            values,
            raw_reports,
        })
    }

    /// Raw RAM T value for the official 0-based physical key position.
    pub fn value_for_position(&self, position: usize) -> Option<u8> {
        self.values.get(position).copied()
    }

    pub fn values(&self) -> &[u8; 66] {
        &self.values
    }

    /// Eleven original DC mode-2 rows, including opaque bytes.
    pub fn raw_reports(&self) -> &[Report] {
        &self.raw_reports
    }
}
