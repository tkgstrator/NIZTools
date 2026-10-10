//! Conservative importer for known Windows .pro KEY records. No Windows binaries are executed.
use crate::{
    Definition, DeviceIdentity, Error, MAX_FILE_SIZE, Profile, Result, encode_definition,
    error::require, parse_key,
};
use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};
use std::collections::BTreeMap;

pub const WINDOWS_PROFILE_FIRMWARE: &str = "Windows .pro (offline)";
const ROOTS: [&str; 5] = [
    "66EC(XRGB)Ble",
    "66EC(XRGB)BLe",
    "66EC(S)Ble",
    "66EC(S)BLe",
    "66EC(S)",
];
pub(crate) fn recognized_root(root: &str) -> bool {
    ROOTS.contains(&root)
}

pub(crate) fn root_matches_firmware(root: &str, firmware: &str) -> bool {
    // These are textual model families only. The static evidence does not map roots to PIDs.
    let source = root.to_ascii_uppercase();
    let target = firmware.to_ascii_uppercase();
    let target_root = ["66EC(XRGB)BLE", "66EC(S)BLE", "66EC(S)"]
        .into_iter()
        .find(|prefix| target.starts_with(prefix));
    target_root == Some(source.as_str())
        && !(source == "66EC(S)" && (target.contains("BLE") || target.contains("RGB")))
}

#[derive(Default)]
struct Node {
    name: String,
    attributes: BTreeMap<String, String>,
    children: Vec<Node>,
}
impl Node {
    fn child(&self, name: &str) -> Result<&Node> {
        let nodes: Vec<_> = self.children.iter().filter(|n| n.name == name).collect();
        require(nodes.len() == 1, format!("expected exactly one {name}"))?;
        Ok(nodes[0])
    }
    fn number(&self, name: &str, max: u16, default: Option<u16>) -> Result<u16> {
        let value = match self.attributes.get(name) {
            Some(value) => value,
            None => {
                return default.ok_or_else(|| Error::Invalid(format!("missing attribute {name}")));
            }
        };
        require(
            !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()),
            format!("invalid integer {name}"),
        )?;
        let number = value
            .parse::<u16>()
            .map_err(|_| Error::Invalid(format!("integer overflow {name}")))?;
        require(number <= max, format!("attribute {name} exceeds {max}"))?;
        Ok(number)
    }
}
fn start_node(event: &BytesStart<'_>, reader: &Reader<&[u8]>) -> Result<Node> {
    let name = String::from_utf8(event.name().as_ref().to_vec())
        .map_err(|_| Error::Invalid("XML element name is not UTF-8".into()))?;
    let mut node = Node {
        name,
        ..Node::default()
    };
    for attribute in event.attributes() {
        let attribute = attribute.map_err(|e| Error::Invalid(format!("XML attribute: {e}")))?;
        let name = String::from_utf8(attribute.key.as_ref().to_vec())
            .map_err(|_| Error::Invalid("XML attribute name is not UTF-8".into()))?;
        let value = attribute
            .decode_and_unescape_value(reader.decoder())?
            .into_owned();
        require(
            node.attributes.insert(name, value).is_none(),
            "duplicate XML attribute",
        )?;
    }
    Ok(node)
}
fn parse_xml(text: &str) -> Result<Node> {
    require(text.len() <= MAX_FILE_SIZE, "legacy file exceeds 4 MiB")?;
    let upper = text.to_ascii_uppercase();
    require(
        !upper.contains("<!DOCTYPE") && !upper.contains("<!ENTITY"),
        "DTD and entities are forbidden",
    )?;
    let mut reader = Reader::from_str(text);
    let mut stack: Vec<Node> = Vec::new();
    let mut root: Option<Node> = None;
    let mut count = 0usize;
    let mut events = 0usize;
    let mut declaration_seen = false;
    loop {
        match reader.read_event()? {
            Event::Start(e) => {
                count += 1;
                require(
                    count <= 20000 && stack.len() < 32,
                    "XML nesting or node limit exceeded",
                )?;
                stack.push(start_node(&e, &reader)?);
            }
            Event::Empty(e) => {
                count += 1;
                require(count <= 20000, "XML node limit exceeded")?;
                let node = start_node(&e, &reader)?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    require(root.is_none(), "multiple XML roots")?;
                    root = Some(node);
                }
            }
            Event::End(_) => {
                let node = stack
                    .pop()
                    .ok_or_else(|| Error::Invalid("unmatched XML end".into()))?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    require(root.is_none(), "multiple XML roots")?;
                    root = Some(node);
                }
            }
            Event::Text(t) => {
                require(
                    t.as_ref().iter().all(u8::is_ascii_whitespace),
                    "unexpected XML text",
                )?;
            }
            Event::CData(_) | Event::DocType(_) | Event::GeneralRef(_) => {
                return Err(Error::Invalid("unsupported XML content/entity".into()));
            }
            Event::Eof => break,
            Event::Decl(declaration) => {
                require(
                    events == 0 && !declaration_seen && stack.is_empty() && root.is_none(),
                    "XML declaration must be leading and unique",
                )?;
                require(
                    declaration.version()?.as_ref() == b"1.0",
                    "only XML 1.0 is supported",
                )?;
                let content = std::str::from_utf8(declaration.as_ref())
                    .map_err(|_| Error::Invalid("XML declaration is not UTF-8".into()))?;
                let attributes = BytesStart::from_content(content, 3);
                let mut last_order = 0;
                for (i, attribute) in attributes.attributes().enumerate() {
                    let attribute = attribute
                        .map_err(|e| Error::Invalid(format!("XML declaration attribute: {e}")))?;
                    let order = match attribute.key.as_ref() {
                        b"version" => {
                            require(
                                i == 0 && attribute.value.as_ref() == b"1.0",
                                "XML version must be first and 1.0",
                            )?;
                            1
                        }
                        b"encoding" => {
                            require(
                                attribute.value.eq_ignore_ascii_case(b"UTF-8"),
                                "only UTF-8 XML encoding is supported",
                            )?;
                            2
                        }
                        b"standalone" => {
                            require(
                                matches!(attribute.value.as_ref(), b"yes" | b"no"),
                                "invalid XML standalone value",
                            )?;
                            3
                        }
                        _ => {
                            return Err(Error::Invalid("unknown XML declaration attribute".into()));
                        }
                    };
                    require(
                        order > last_order,
                        "XML declaration attributes are out of order or duplicated",
                    )?;
                    last_order = order;
                }
                declaration_seen = true;
            }
            Event::Comment(_) => {}
            Event::PI(_) => {
                return Err(Error::Invalid(
                    "XML processing instructions are forbidden".into(),
                ));
            }
        }
        events += 1;
    }
    require(stack.is_empty(), "truncated XML")?;
    root.ok_or_else(|| Error::Invalid("empty XML".into()))
}
fn legacy_key(text: &str) -> Result<u8> {
    let text = text.trim();
    let lower = text.to_ascii_lowercase();
    let text = if lower.ends_with(" up") {
        &text[..text.len() - 3]
    } else if lower.ends_with(" down") {
        &text[..text.len() - 5]
    } else {
        text
    };
    parse_key(text)
}

pub fn import_windows_profile(text: &str) -> Result<Profile> {
    let root = parse_xml(text)?;
    require(
        ROOTS.contains(&root.name.as_str()),
        "unknown Windows profile model/root",
    )?;
    let settings = root.child("CurrentSettings")?;
    require(
        settings.children.len() == 198 && settings.children.iter().all(|node| node.name == "KEY"),
        "expected exactly 198 KEY records",
    )?;
    let mut records = vec![None; 198];
    for node in &settings.children {
        let key = node.number("ID", 66, None)? as usize;
        require(key > 0, "legacy key ID must be 1..66")?;
        let level = node.number("Level", 2, None)? as usize;
        let mode = node.number("Mode", 4, None)?;
        let index = level * 66 + key - 1;
        require(records[index].is_none(), "duplicate legacy key/level")?;
        let mut def = Definition {
            kind: 0,
            keys: vec![],
            interval: 0,
            cycles: 1,
            custom_delay: false,
            delays: vec![],
        };
        match mode {
            0 => {
                require(
                    node.children.is_empty(),
                    "unassigned KEY has child definition",
                )?;
            }
            1 => {
                require(node.children.is_empty(), "single KEY has child definition")?;
                let code = node.number("HWCode", 255, None)? as u8;
                def.keys = if code == 199 {
                    vec![130, 130]
                } else {
                    vec![code]
                };
            }
            2..=4 => {
                let combo = node.child("ComboKey")?;
                require(
                    node.children.len() == 1 && combo.children.len() == 1,
                    "unexpected combo children",
                )?;
                let list = combo.child("List")?;
                require(
                    !list.children.is_empty()
                        && list.children.len() <= 4095
                        && list.children.iter().all(|n| n.name == "x"),
                    "invalid legacy sequence length/items",
                )?;
                let mut entries: Vec<Option<&str>> = vec![None; list.children.len()];
                for item in &list.children {
                    require(
                        item.children.is_empty()
                            || (item.children.len() == 1
                                && item.children[0].name == "List"
                                && item.children[0].children.is_empty()
                                && item.children[0].attributes.is_empty()),
                        "nested legacy sequence item",
                    )?;
                    let id = item.number("ID", 4094, None)? as usize;
                    require(
                        id < entries.len() && entries[id].is_none(),
                        "duplicate or out-of-range sequence ID",
                    )?;
                    entries[id] = Some(
                        item.attributes
                            .get("t1")
                            .ok_or_else(|| Error::Invalid("missing legacy action name".into()))?,
                    );
                }
                let custom = mode == 3 && list.number("IsAutoInterval", 1, Some(0))? == 0;
                require(
                    !custom || entries.len() % 2 == 1,
                    "custom sequence must end with a key",
                )?;
                for (i, entry) in entries.into_iter().enumerate() {
                    let entry =
                        entry.ok_or_else(|| Error::Invalid("missing sequence item".into()))?;
                    if custom && i % 2 == 1 {
                        let value = entry
                            .strip_prefix('<')
                            .and_then(|s| s.strip_suffix(" ms>"))
                            .ok_or_else(|| Error::Invalid("invalid legacy delay token".into()))?;
                        require(
                            !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()),
                            "invalid legacy delay",
                        )?;
                        def.delays.push(
                            value
                                .parse::<u16>()
                                .map_err(|_| Error::Invalid("legacy delay exceeds 65535".into()))?,
                        );
                    } else {
                        def.keys.push(legacy_key(entry)?);
                    }
                }
                if mode == 4 {
                    def.kind = 1;
                    def.interval = list.number("Delay", 65535, Some(30))?;
                }
                if mode == 3 {
                    let play = list.number("PlayMode", 2, Some(0))? as u8;
                    def.kind = play + 2;
                    def.cycles = if play == 0 {
                        list.number("Cycles", 255, Some(1))? as u8
                    } else {
                        0
                    };
                    def.custom_delay = custom;
                    def.interval = if custom {
                        0
                    } else {
                        list.number("Delay", 65535, Some(30))?
                    };
                }
            }
            _ => unreachable!("numeric bounds checked"),
        }
        records[index] = Some(encode_definition(index, &def)?);
    }
    let reports = records
        .into_iter()
        .map(|record| record.ok_or_else(|| Error::Invalid("missing legacy KEY".into())))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    let mut profile = Profile::from_reports(
        WINDOWS_PROFILE_FIRMWARE.into(),
        DeviceIdentity::default(),
        reports,
    )?;
    profile.legacy_root = Some(root.name);
    profile.validate_for_writing()?;
    Ok(profile)
}
