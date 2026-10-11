use crate::{Report, Result, error::require};

// Inverse of the 66-byte physical-number permutation at 0x9dc6 in the exact
// 66EC(XRGB)BLe V1.0.48 image, SHA-256
// b82e11891dfdc138780a81ac635697a7edb0ca1d4e123d6ad38c81a7ba55bf17.
// Each 0-based physical slot selects one flat cell from the 11 x 6 DC table.
const PHYSICAL_TO_MATRIX: [usize; 66] = [
    0, 1, 2, 3, 4, 5, 29, 28, 27, 26, 25, 24, 60, 61, 62, 6, 7, 8, 9, 10, 11, 35, 34, 33, 32, 31,
    30, 63, 65, 12, 13, 14, 15, 16, 17, 41, 40, 39, 38, 37, 36, 64, 18, 19, 20, 21, 22, 23, 47, 46,
    45, 44, 43, 42, 58, 48, 49, 50, 51, 52, 53, 57, 56, 55, 54, 59,
];

/// Strict decoder for the pinned firmware's eleven DC mode-1 rows.
/// Values are unsigned firmware signal units, not ADC samples or millimeters.
#[derive(Debug)]
pub struct RawThresholdDecoder {
    matrix: [u8; 66],
    rows: usize,
    failed: bool,
}
impl Default for RawThresholdDecoder {
    fn default() -> Self {
        Self {
            matrix: [0; 66],
            rows: 0,
            failed: false,
        }
    }
}
impl RawThresholdDecoder {
    /// Accept one row in the exact emitted order, 1 through 11.
    /// Bytes 10 through 63 are opaque and are not used as a padding predicate.
    pub fn push(&mut self, report: &Report) -> Result<()> {
        require(
            !self.failed,
            "raw threshold decoder invalid after row failure",
        )?;
        self.failed = true;
        require(self.rows < 11, "excess raw threshold row")?;
        require(
            report[..2] == [0, 0xdc],
            "raw threshold prefix must be 00 DC",
        )?;
        require(report[3] == 1, "raw threshold response mode must be 1")?;
        require(
            report[2] as usize == self.rows + 1,
            "raw threshold rows must be exactly 1 through 11 in order",
        )?;
        self.matrix[self.rows * 6..self.rows * 6 + 6].copy_from_slice(&report[4..10]);
        self.rows += 1;
        self.failed = false;
        Ok(())
    }

    /// Publish a physical-slot-ordered table only after all eleven rows.
    pub fn finish(self) -> Result<[u8; 66]> {
        require(
            !self.failed,
            "raw threshold decoder invalid after row failure",
        )?;
        require(self.rows == 11, "incomplete raw threshold table")?;
        Ok(PHYSICAL_TO_MATRIX.map(|flat| self.matrix[flat]))
    }
}

/// Decode a complete DC mode-1 stream into the official 0-based physical slots.
/// Metadata qualification belongs to the device layer; this function sends no I/O.
pub fn decode_raw_thresholds(reports: &[Report]) -> Result<[u8; 66]> {
    let mut decoder = RawThresholdDecoder::default();
    for report in reports {
        decoder.push(report)?;
    }
    decoder.finish()
}
