use crate::{Error, Report, Result, error::require};

pub const MAX_REPORTS: usize = 8192;
pub const MAX_FILE_SIZE: usize = 4 * 1024 * 1024;
pub(crate) const KEY_COUNT: usize = 66;
pub(crate) const EDITABLE_RECORDS: usize = 3 * KEY_COUNT;
const MAX_SEQUENCE: usize = 2048;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Definition {
    pub kind: u8,
    pub keys: Vec<u8>,
    pub interval: u16,
    pub cycles: u8,
    pub custom_delay: bool,
    /// Delays between keys. The native wire's final secondary value is opaque.
    pub delays: Vec<u16>,
}

pub fn command(op: u8) -> Report {
    let mut report = [0; 64];
    report[1] = op;
    report
}

pub(crate) fn validate_header(report: &Report) -> Result<()> {
    require(
        report[0] == 0 && report[1] == 0xf0,
        "record prefix must be 00 F0",
    )?;
    require((1..=9).contains(&report[2]), "group must be 1..9")?;
    require((1..=66).contains(&report[3]), "key must be 1..66")?;
    if report[4] > 4 {
        return Err(Error::Unsupported(format!("record kind {}", report[4])));
    }
    Ok(())
}

pub(crate) fn record_report_count(report: &Report) -> Result<usize> {
    validate_header(report)?;
    if report[4] < 2 {
        return Ok(1);
    }
    let size = u16::from_be_bytes([report[9], report[10]]) as usize;
    require(
        size > 0 && size <= MAX_SEQUENCE * 4,
        "macro body length outside 1..8192",
    )?;
    Ok(size.div_ceil(53))
}

pub(crate) fn known_code(code: u8) -> bool {
    // These are original EC table entries. Newer Mac-specific codes need separate evidence.
    (1..=177).contains(&code) || code == 199
}

pub(crate) fn validate_editable(def: &Definition) -> Result<()> {
    require(def.kind <= 4, "kind must be 0..4")?;
    require(
        def.keys.iter().all(|&code| known_code(code)),
        "unknown or reserved action code",
    )?;
    require(
        def.keys.len()
            <= match def.kind {
                0 => 58,
                1 => 56,
                _ => MAX_SEQUENCE,
            },
        "too many keys",
    )?;
    require(def.kind == 0 || !def.keys.is_empty(), "sequence is empty")?;
    if def.keys.iter().any(|code| [156, 166].contains(code)) {
        require(
            def.kind == 0 && def.keys.len() == 1,
            "Fn must be a single action",
        )?;
    }
    if def.kind < 2 {
        require(
            !def.custom_delay && def.delays.is_empty(),
            "delay mode requires macro",
        )?;
    } else {
        require(
            def.kind != 2 || def.cycles > 0,
            "counted macro cycles must be positive",
        )?;
        require(
            if def.custom_delay {
                def.delays.len() + 1 == def.keys.len()
            } else {
                def.delays.is_empty()
            },
            "delay count must equal key count minus one",
        )?;
    }
    Ok(())
}

pub fn encode_definition(index: usize, def: &Definition) -> Result<Vec<Report>> {
    require(index < 9 * KEY_COUNT, "record index outside 0..593")?;
    validate_editable(def)?;
    let mut header = command(0xf0);
    header[2] = (index / KEY_COUNT + 1) as u8;
    header[3] = (index % KEY_COUNT + 1) as u8;
    header[4] = def.kind;
    if def.kind < 2 {
        let offset = if def.kind == 0 {
            header[5] = def.keys.len() as u8;
            6
        } else {
            header[5..7].copy_from_slice(&def.interval.to_be_bytes());
            header[7] = def.keys.len() as u8;
            8
        };
        header[offset..offset + def.keys.len()].copy_from_slice(&def.keys);
        return Ok(vec![header]);
    }
    let mut payload = Vec::with_capacity(def.keys.len() * if def.custom_delay { 4 } else { 1 });
    for (i, &code) in def.keys.iter().enumerate() {
        payload.push(code);
        if def.custom_delay {
            // Match native DLL serializer, including a final C8/0000.
            payload.push(200);
            payload.extend_from_slice(&def.delays.get(i).copied().unwrap_or(0).to_be_bytes());
        }
    }
    header[5] = def.cycles;
    header[6] = u8::from(def.custom_delay);
    header[7..9].copy_from_slice(&def.interval.to_be_bytes());
    header[9..11].copy_from_slice(&(payload.len() as u16).to_be_bytes());
    Ok(payload
        .chunks(53)
        .map(|body| {
            let mut report = header;
            report[11..11 + body.len()].copy_from_slice(body);
            report
        })
        .collect())
}

pub fn decode_definition(reports: &[Report]) -> Result<Definition> {
    let b = reports
        .first()
        .ok_or_else(|| Error::Invalid("no record reports".into()))?;
    let expected = record_report_count(b)?;
    require(
        reports.len() == expected,
        "incomplete or excess record reports",
    )?;
    let mut def = Definition {
        kind: b[4],
        keys: Vec::new(),
        interval: 0,
        cycles: 1,
        custom_delay: false,
        delays: Vec::new(),
    };
    if def.kind < 2 {
        let (count, offset) = if def.kind == 0 {
            (b[5] as usize, 6)
        } else {
            def.interval = u16::from_be_bytes([b[5], b[6]]);
            (b[7] as usize, 8)
        };
        require(count <= 64 - offset, "key count exceeds report capacity")?;
        require(def.kind == 0 || count > 0, "repeat sequence is empty")?;
        def.keys.extend_from_slice(&b[offset..offset + count]);
        return Ok(def);
    }
    require(
        reports.iter().all(|r| r[..11] == b[..11]),
        "macro continuation headers differ",
    )?;
    require(b[6] <= 1, "unknown macro delay mode")?;
    require(
        def.kind != 2 || b[5] > 0,
        "counted macro cycles must be positive",
    )?;
    let size = u16::from_be_bytes([b[9], b[10]]) as usize;
    let body: Vec<u8> = reports
        .iter()
        .flat_map(|r| r[11..].iter().copied())
        .take(size)
        .collect();
    def.cycles = b[5];
    def.custom_delay = b[6] == 1;
    def.interval = u16::from_be_bytes([b[7], b[8]]);
    if !def.custom_delay {
        require(size <= MAX_SEQUENCE, "macro has too many keys")?;
        def.keys = body;
    } else {
        require(
            size.is_multiple_of(4) || size % 4 == 1,
            "custom macro must use 4n or 4n-3 body",
        )?;
        let n = size.div_ceil(4);
        require(n <= MAX_SEQUENCE, "macro has too many keys")?;
        for i in 0..n {
            let offset = i * 4;
            def.keys.push(body[offset]);
            if offset + 1 < size {
                require(body[offset + 1] == 200, "missing C8 delay marker")?;
                if i + 1 < n {
                    def.delays
                        .push(u16::from_be_bytes([body[offset + 2], body[offset + 3]]));
                }
            }
        }
    }
    Ok(def)
}
