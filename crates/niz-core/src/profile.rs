use crate::{
    Definition, Error, MAX_FILE_SIZE, MAX_REPORTS, Report, Result, WINDOWS_PROFILE_FIRMWARE,
    decode_definition, encode_definition,
    error::require,
    protocol::{EDITABLE_RECORDS, KEY_COUNT, record_report_count, validate_editable},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceIdentity {
    pub vendor_id: u16,
    pub product_id: u16,
    pub usage_page: u16,
    pub usage: u16,
}

#[derive(Clone, Debug)]
pub struct Profile {
    pub firmware: String,
    pub identity: DeviceIdentity,
    pub records: Vec<Vec<Report>>,
    pub counters: Vec<u32>,
    pub captured_at: String,
    pub(crate) legacy_root: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonProfile {
    format: String,
    schema: u8,
    version: String,
    identity: DeviceIdentity,
    reports: Vec<String>,
    counters: Vec<u32>,
    #[serde(default)]
    captured_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    legacy_root: Option<String>,
}

impl Profile {
    pub fn from_reports(
        firmware: String,
        identity: DeviceIdentity,
        reports: Vec<Report>,
    ) -> Result<Self> {
        require(
            (EDITABLE_RECORDS..=MAX_REPORTS).contains(&reports.len()),
            "expected complete 3 or 9 groups within report limit",
        )?;
        require(
            !firmware.is_empty() && firmware.len() <= 256,
            "firmware must be 1..256 bytes",
        )?;
        let groups = reports.iter().map(|r| r[2] as usize).max().unwrap_or(0);
        require([3, 9].contains(&groups), "profile must have 3 or 9 groups")?;
        let mut slots: Vec<Option<Vec<Report>>> = vec![None; groups * KEY_COUNT];
        let mut i = 0;
        while i < reports.len() {
            let r = &reports[i];
            let count = record_report_count(r)?;
            require(i + count <= reports.len(), "macro reports are incomplete")?;
            let index = (r[2] as usize - 1) * KEY_COUNT + r[3] as usize - 1;
            require(slots[index].is_none(), format!("duplicate record {index}"))?;
            let record = reports[i..i + count].to_vec();
            decode_definition(&record)?;
            slots[index] = Some(record);
            i += count;
        }
        let records = slots
            .into_iter()
            .enumerate()
            .map(|(i, record)| record.ok_or_else(|| Error::Invalid(format!("missing record {i}"))))
            .collect::<Result<_>>()?;
        Ok(Self {
            firmware,
            identity,
            records,
            counters: Vec::new(),
            captured_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .to_string(),
            legacy_root: None,
        })
    }

    pub fn from_json(text: &str) -> Result<Self> {
        require(text.len() <= MAX_FILE_SIZE, "profile file exceeds 4 MiB")?;
        let json: JsonProfile = serde_json::from_str(text)?;
        require(
            json.format == "atom66-macos" && json.schema == 1,
            "unsupported JSON format/schema",
        )?;
        require(json.reports.len() <= MAX_REPORTS, "too many reports")?;
        let reports = json
            .reports
            .iter()
            .map(|s| {
                require(
                    s.len() == 128 && s.is_ascii(),
                    "report hex must have 128 ASCII characters",
                )?;
                let mut r = [0; 64];
                for (i, byte) in r.iter_mut().enumerate() {
                    *byte = u8::from_str_radix(&s[2 * i..2 * i + 2], 16)
                        .map_err(|_| Error::Invalid("report contains invalid hex".into()))?;
                }
                Ok(r)
            })
            .collect::<Result<_>>()?;
        require(
            json.counters.is_empty() || json.counters.len() == KEY_COUNT,
            "counters must have 0 or 66 entries",
        )?;
        require(json.captured_at.len() <= 256, "capture timestamp too long")?;
        let mut profile = Self::from_reports(json.version, json.identity, reports)?;
        profile.counters = json.counters;
        profile.captured_at = json.captured_at;
        profile.legacy_root = json.legacy_root;
        profile.validate_legacy_metadata()?;
        Ok(profile)
    }

    pub fn to_json(&self) -> Result<String> {
        self.validate_structure()?;
        let json = JsonProfile {
            format: "atom66-macos".into(),
            schema: 1,
            version: self.firmware.clone(),
            identity: self.identity.clone(),
            reports: self
                .reports()
                .iter()
                .map(|r| r.iter().map(|b| format!("{b:02x}")).collect())
                .collect(),
            counters: self.counters.clone(),
            captured_at: self.captured_at.clone(),
            legacy_root: self.legacy_root.clone(),
        };
        let text = serde_json::to_string_pretty(&json)?;
        require(text.len() <= MAX_FILE_SIZE, "profile file exceeds 4 MiB")?;
        Ok(text)
    }

    pub fn demo() -> Self {
        let codes: [u8; KEY_COUNT] = [
            1, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 41, 14, 28, 29, 30, 31, 32, 33, 34,
            35, 36, 37, 38, 39, 40, 27, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56,
            57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 156, 67, 68, 69, 166, 70, 71, 73, 74, 88, 89,
            90,
        ];
        let reports = (0..EDITABLE_RECORDS)
            .flat_map(|i| {
                let code = if i < KEY_COUNT || [54, 58].contains(&(i % KEY_COUNT)) {
                    codes[i % KEY_COUNT]
                } else if (1..=12).contains(&(i % KEY_COUNT)) {
                    (i % KEY_COUNT + 1) as u8
                } else {
                    0
                };
                let def = Definition {
                    kind: 0,
                    keys: if code == 0 { vec![] } else { vec![code] },
                    interval: 0,
                    cycles: 1,
                    custom_delay: false,
                    delays: vec![],
                };
                encode_definition(i, &def).expect("fixed demo definition is valid")
            })
            .collect();
        Self::from_reports(
            "Offline demo (not a device capture)".into(),
            DeviceIdentity::default(),
            reports,
        )
        .expect("fixed demo is complete")
    }

    /// Recognized Windows source root, retained for variant compatibility checks.
    pub fn legacy_root(&self) -> Option<&str> {
        self.legacy_root.as_deref()
    }

    fn validate_legacy_metadata(&self) -> Result<()> {
        if let Some(root) = self.legacy_root() {
            require(
                crate::legacy::recognized_root(root),
                "unknown legacy source model",
            )?;
            require(
                self.firmware == WINDOWS_PROFILE_FIRMWARE
                    && self.identity == DeviceIdentity::default()
                    && self.group_count() == 3,
                "legacy source metadata requires offline Windows profile",
            )?;
        }
        Ok(())
    }

    pub fn group_count(&self) -> usize {
        self.records.len() / KEY_COUNT
    }
    pub fn reports(&self) -> Vec<Report> {
        self.records.iter().flatten().copied().collect()
    }
    pub fn definition(&self, index: usize) -> Result<Definition> {
        decode_definition(
            self.records
                .get(index)
                .ok_or_else(|| Error::Invalid("record index out of range".into()))?,
        )
    }

    pub fn set_definition(&mut self, index: usize, def: &Definition) -> Result<()> {
        require(
            index < EDITABLE_RECORDS && index < self.records.len(),
            "only first 3 groups are editable",
        )?;
        self.validate_structure()?;
        validate_editable(def)?;
        // Edits are atomic: linked Fn changes and all validation happen on a draft.
        let current = self.definition(index)?;
        validate_editable(&current)?;
        if current == *def {
            return Ok(());
        }
        let linked = is_fn(&self.definition(index % KEY_COUNT)?).is_some() || is_fn(def).is_some();
        let mut draft = self.clone();
        if linked {
            for layer in 0..3 {
                draft.patch(layer * KEY_COUNT + index % KEY_COUNT, def)?;
            }
        } else {
            draft.patch(index, def)?;
        }
        draft.validate_fn()?;
        *self = draft;
        Ok(())
    }

    /// Exchange understood raw records on one editable layer. A normal-layer Fn anchor
    /// links both physical columns across all three editable layers. Only report addresses change.
    pub fn swap_keys(&mut self, from: usize, to: usize) -> Result<()> {
        require(
            from < EDITABLE_RECORDS
                && to < EDITABLE_RECORDS
                && from < self.records.len()
                && to < self.records.len(),
            "swap indices must be editable",
        )?;
        require(
            from / KEY_COUNT == to / KEY_COUNT,
            "swap indices must be on the same layer",
        )?;
        self.validate_structure()?;
        self.validate_fn()?;
        let linked = is_fn(&self.definition(from % KEY_COUNT)?).is_some()
            || is_fn(&self.definition(to % KEY_COUNT)?).is_some();
        let pairs: Vec<_> = if linked {
            (0..3)
                .map(|layer| {
                    (
                        layer * KEY_COUNT + from % KEY_COUNT,
                        layer * KEY_COUNT + to % KEY_COUNT,
                    )
                })
                .collect()
        } else {
            vec![(from, to)]
        };
        // Validate every source before moving any record, including linked alternate layers.
        for &(a, b) in &pairs {
            validate_editable(&self.definition(a)?)?;
            validate_editable(&self.definition(b)?)?;
        }
        if from == to {
            return Ok(());
        }
        let mut draft = self.clone();
        for (a, b) in pairs {
            draft.records.swap(a, b);
            for index in [a, b] {
                for report in &mut draft.records[index] {
                    report[2] = (index / KEY_COUNT + 1) as u8;
                    report[3] = (index % KEY_COUNT + 1) as u8;
                }
            }
        }
        draft.validate_changes_against(self)?;
        *self = draft;
        Ok(())
    }

    /// Validate a proposed configuration relative to a complete baseline. Unknown semantic
    /// records are permitted only when raw-identical at the same position. This comparison
    /// does not authenticate a live device baseline or establish hardware qualification.
    pub fn validate_changes_against(&self, baseline: &Self) -> Result<()> {
        self.validate_structure()?;
        baseline.validate_structure()?;
        require(
            self.identity == baseline.identity && self.firmware == baseline.firmware,
            "baseline device/firmware mismatch",
        )?;
        require(
            self.records.len() == baseline.records.len(),
            "baseline group count mismatch",
        )?;
        self.validate_fn()?;
        baseline.validate_fn()?;
        for (index, (desired, original)) in self.records.iter().zip(&baseline.records).enumerate() {
            if desired == original {
                continue;
            }
            require(
                index < EDITABLE_RECORDS,
                "opaque extra groups must remain unchanged",
            )?;
            validate_editable(&baseline.definition(index)?)?;
            validate_editable(&self.definition(index)?)?;
        }
        Ok(())
    }

    fn patch(&mut self, index: usize, def: &Definition) -> Result<()> {
        let current = self.definition(index)?;
        validate_editable(&current)?;
        if current == *def {
            return Ok(());
        }
        let mut encoded = encode_definition(index, def)?;
        let mut body_size = if def.kind >= 2 {
            u16::from_be_bytes([encoded[0][9], encoded[0][10]]) as usize
        } else {
            0
        };
        if current.kind >= 2 && current.custom_delay && def.kind >= 2 && def.custom_delay {
            let old_size =
                u16::from_be_bytes([self.records[index][0][9], self.records[index][0][10]])
                    as usize;
            if old_size % 4 == 1 {
                // An existing public-writer record keeps its short trailing form.
                body_size -= 3;
                encoded.truncate(body_size.div_ceil(53));
                for report in &mut encoded {
                    report[9..11].copy_from_slice(&(body_size as u16).to_be_bytes());
                }
            }
        }
        let mut result: Vec<Report> = encoded
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let mut original = self.records[index].get(i).copied().unwrap_or([0; 64]);
                let active = match def.kind {
                    0 => 6 + def.keys.len(),
                    1 => 8 + def.keys.len(),
                    _ => 11 + (body_size - i * 53).min(53),
                };
                original[..active].copy_from_slice(&r[..active]);
                original
            })
            .collect();
        // The native decoder ignores the last secondary value. Preserve it on existing
        // native custom macros instead of assigning a newly generated zero over it.
        if current.kind >= 2 && current.custom_delay && def.kind >= 2 && def.custom_delay {
            let old_size =
                u16::from_be_bytes([self.records[index][0][9], self.records[index][0][10]])
                    as usize;
            if old_size.is_multiple_of(4) {
                for trailing in 0..2 {
                    let old_offset = old_size - 2 + trailing;
                    let new_offset = body_size - 2 + trailing;
                    result[new_offset / 53][11 + new_offset % 53] =
                        self.records[index][old_offset / 53][11 + old_offset % 53];
                }
            }
        }
        self.records[index] = result;
        Ok(())
    }

    pub fn differences(&self, other: &Self) -> Result<Vec<usize>> {
        self.validate_structure()?;
        other.validate_structure()?;
        require(
            self.firmware == other.firmware && self.identity == other.identity,
            "profile device/firmware mismatch",
        )?;
        require(
            self.records.len() == other.records.len(),
            "profile group count mismatch",
        )?;
        Ok(self
            .records
            .iter()
            .zip(&other.records)
            .enumerate()
            .filter_map(|(i, (a, b))| (a != b).then_some(i))
            .collect())
    }

    fn validate_structure(&self) -> Result<()> {
        self.validate_legacy_metadata()?;
        require(
            [EDITABLE_RECORDS, 9 * KEY_COUNT].contains(&self.records.len()),
            "record count must be 198 or 594",
        )?;
        require(
            self.counters.is_empty() || self.counters.len() == KEY_COUNT,
            "counters must have 0 or 66 entries",
        )?;
        require(self.captured_at.len() <= 256, "capture timestamp too long")?;
        let rebuilt =
            Self::from_reports(self.firmware.clone(), self.identity.clone(), self.reports())?;
        require(
            rebuilt.records == self.records,
            "records must be stored in canonical position order",
        )
    }

    fn validate_fn(&self) -> Result<()> {
        let mut present = [false; 2];
        // Alternate layers may have additional Fn actions at otherwise ordinary columns.
        // Only Fn assignments on the normal layer anchor a synchronized physical column.
        for key in 0..KEY_COUNT {
            if let Some(code) = is_fn(&self.definition(key)?) {
                for layer in 1..3 {
                    require(
                        is_fn(&self.definition(layer * KEY_COUNT + key)?) == Some(code),
                        "normal-layer Fn anchor differs between editable layers",
                    )?;
                }
                present[usize::from(code == 166)] = true;
            }
        }
        require(
            present.iter().all(|found| *found),
            "both Left Fn and Right Fn must remain assigned on the normal layer",
        )
    }

    /// Strict standalone semantic validation. Opaque captured actions require the separate
    /// baseline-relative comparison; they are never added to the editable action whitelist.
    pub fn validate_for_writing(&self) -> Result<()> {
        self.validate_structure()?;
        for index in 0..EDITABLE_RECORDS {
            validate_editable(&self.definition(index)?)?;
        }
        self.validate_fn()
    }

    /// Merge a portable import onto this current baseline. Real firmware versions must match exactly.
    /// A Windows offline import must retain a recognized source root matching the target firmware
    /// family. It has no numeric device binding: patch only supported semantic fields and preserve
    /// the baseline's opaque bytes, extra groups, identity, counters and capture timestamp.
    pub fn merge_import(&self, imported: &Self) -> Result<Self> {
        self.validate_structure()?;
        self.validate_fn()?;
        imported.validate_structure()?;
        imported.validate_fn()?;
        require(
            imported.group_count() == self.group_count()
                || (imported.group_count() == 3 && self.group_count() == 9),
            "incompatible import group count",
        )?;
        let legacy = imported.firmware == WINDOWS_PROFILE_FIRMWARE
            && imported.identity == DeviceIdentity::default()
            && imported.legacy_root().is_some();
        if legacy {
            imported.validate_for_writing()?;
            require(
                crate::legacy::root_matches_firmware(
                    imported.legacy_root().expect("checked source root"),
                    &self.firmware,
                ),
                "Windows source variant differs from target firmware",
            )?;
        }
        require(
            legacy || (self.firmware == imported.firmware && self.identity == imported.identity),
            "import firmware or device identity mismatch",
        )?;
        let mut merged = self.clone();
        if legacy {
            require(
                imported.group_count() == 3,
                "Windows import must have 3 groups",
            )?;
            for i in 0..EDITABLE_RECORDS {
                merged.patch(i, &imported.definition(i)?)?;
            }
        } else {
            merged.records[..imported.records.len()].clone_from_slice(&imported.records);
        }
        if legacy {
            merged.validate_for_writing()?;
        } else {
            // A three-group native import first becomes a complete target-sized draft.
            // The exact-group differential gate then retains baseline opaque extra groups.
            merged.validate_changes_against(self)?;
        }
        Ok(merged)
    }
}

fn is_fn(def: &Definition) -> Option<u8> {
    if def.kind == 0 && def.keys.len() == 1 && [156, 166].contains(&def.keys[0]) {
        Some(def.keys[0])
    } else {
        None
    }
}
