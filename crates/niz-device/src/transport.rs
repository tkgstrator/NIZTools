use crate::{Error, Result};
use niz_core::DeviceIdentity;
use serde::Serialize;
use std::time::Instant;
#[derive(Clone, Debug, Serialize)]
pub struct DeviceDescriptor {
    pub path: String,
    pub product: String,
    pub identity: DeviceIdentity,
}
pub(crate) fn supported(i: &DeviceIdentity) -> bool {
    i.vendor_id == 0x0483
        && [0x502a, 0x512a, 0x522a].contains(&i.product_id)
        && i.usage_page == 0x8c
        && i.usage == 1
}
fn identity(info: &hidapi::DeviceInfo) -> DeviceIdentity {
    DeviceIdentity {
        vendor_id: info.vendor_id(),
        product_id: info.product_id(),
        usage_page: info.usage_page(),
        usage: info.usage(),
    }
}
pub fn list_devices() -> Result<Vec<DeviceDescriptor>> {
    let api = hidapi::HidApi::new().map_err(|e| Error::new(e.to_string()))?;
    Ok(api
        .device_list()
        .filter(|d| supported(&identity(d)))
        .map(|d| DeviceDescriptor {
            path: d.path().to_string_lossy().into_owned(),
            product: d.product_string().unwrap_or("").into(),
            identity: identity(d),
        })
        .collect())
}
pub(crate) trait Transport {
    fn connected(&self) -> bool {
        true
    }
    fn queued_input(&mut self) -> std::result::Result<Option<Vec<u8>>, String> {
        let mut bytes = [0; 65];
        let n = self.read(&mut bytes, 0)?;
        Ok((n > 0).then(|| bytes[..n].to_vec()))
    }
    fn prepare(&mut self) -> std::result::Result<(), String> {
        Ok(())
    }
    fn write(&mut self, b: &[u8]) -> std::result::Result<usize, String>;
    fn read(&mut self, b: &mut [u8], timeout: i32) -> std::result::Result<usize, String>;
    fn now_ms(&self) -> u64;
    fn sleep_ms(&mut self, n: u64);
}
struct Native {
    device: hidapi::HidDevice,
    start: Instant,
    _lock: std::fs::File,
    connected: bool,
}
impl Transport for Native {
    fn connected(&self) -> bool {
        self.connected
    }
    fn prepare(&mut self) -> std::result::Result<(), String> {
        let mut report = [0; 65];
        for _ in 0..=niz_core::MAX_REPORTS {
            if self
                .device
                .read_timeout(&mut report, 0)
                .map_err(|e| e.to_string())?
                == 0
            {
                return Ok(());
            }
        }
        Err("stale HID queue exceeded bounded drain limit".into())
    }
    fn write(&mut self, b: &[u8]) -> std::result::Result<usize, String> {
        let result = self.device.write(b).map_err(|e| e.to_string());
        if result.is_err() {
            self.connected = false;
        }
        result
    }
    fn read(&mut self, b: &mut [u8], timeout: i32) -> std::result::Result<usize, String> {
        let result = self
            .device
            .read_timeout(b, timeout)
            .map_err(|e| e.to_string());
        if result.is_err() {
            self.connected = false;
        }
        result
    }
    fn now_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
    fn sleep_ms(&mut self, n: u64) {
        std::thread::sleep(std::time::Duration::from_millis(n));
    }
}
pub(crate) fn open(path: &str) -> Result<(Box<dyn Transport>, DeviceIdentity)> {
    let lock_directory = std::env::temp_dir().join("niz-configuration-locks");
    let lock = acquire_lock(&lock_directory, path)?;
    let api = hidapi::HidApi::new().map_err(|e| Error::new(e.to_string()))?;
    let info = api
        .device_list()
        .find(|d| d.path().to_bytes() == path.as_bytes())
        .ok_or_else(|| Error::new("device path is absent"))?;
    let i = identity(info);
    if !supported(&i) {
        return Err(Error::new("device descriptor identity is unsupported"));
    }
    let device = api
        .open_path(info.path())
        .map_err(|e| Error::new(e.to_string()))?;
    let actual = identity(
        &device
            .get_device_info()
            .map_err(|e| Error::new(e.to_string()))?,
    );
    if actual != i {
        return Err(Error::new("opened device identity changed"));
    }
    let mut descriptor = [0; 4096];
    let n = device
        .get_report_descriptor(&mut descriptor)
        .map_err(|e| Error::new(format!("descriptor unavailable: {e}")))?;
    validate_descriptor(&descriptor[..n])?;
    Ok((
        Box::new(Native {
            device,
            start: Instant::now(),
            _lock: lock,
            connected: true,
        }),
        i,
    ))
}
// Interpret HID short items, respecting global push/pop and report ID. The
// configuration collection must have exactly 512 input and output bits on ID 0.
pub(crate) fn validate_descriptor(bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() || bytes.len() > 4096 {
        return Err(Error::new("HID descriptor length outside 1..4096"));
    }
    let (mut total_input, mut total_output) = (0u32, 0u32);
    #[derive(Clone, Copy, Default)]
    struct Globals {
        size: u32,
        count: u32,
        id: u32,
        page: u32,
    }
    let (mut g, mut stack, mut pos, mut usage, mut collections, mut input, mut output) = (
        Globals::default(),
        Vec::new(),
        0usize,
        0u32,
        Vec::<bool>::new(),
        0u32,
        0u32,
    );
    while pos < bytes.len() {
        let prefix = bytes[pos];
        pos += 1;
        if prefix == 0xfe {
            return Err(Error::new("long HID items unsupported"));
        }
        let n = match prefix & 3 {
            3 => 4,
            x => x as usize,
        };
        if pos + n > bytes.len() {
            return Err(Error::new("truncated HID descriptor"));
        }
        let value = bytes[pos..pos + n]
            .iter()
            .enumerate()
            .fold(0u32, |v, (i, b)| v | ((*b as u32) << (8 * i)));
        pos += n;
        let tag = prefix >> 4;
        match (prefix >> 2 & 3, tag) {
            (1, 0) => g.page = value,
            (1, 7) => g.size = value,
            (1, 8) => {
                if value == 0 || value > 255 {
                    return Err(Error::new("invalid HID report ID"));
                }
                return Err(Error::new(
                    "numbered HID reports cannot be mixed with unnumbered configuration framing",
                ));
            }
            (1, 9) => g.count = value,
            (1, 10) => {
                if stack.len() >= 64 {
                    return Err(Error::new("HID global stack limit exceeded"));
                }
                stack.push(g)
            }
            (1, 11) => {
                g = stack
                    .pop()
                    .ok_or_else(|| Error::new("unbalanced HID global stack"))?
            }
            (2, 0) => usage = value,
            (0, 10) => {
                if collections.len() >= 64 {
                    return Err(Error::new("HID collection nesting limit exceeded"));
                }
                let inherited = collections.last().copied().unwrap_or(false);
                collections.push(inherited || (g.page == 0x8c && usage == 1));
                usage = 0;
            }
            (0, 12) => {
                collections
                    .pop()
                    .ok_or_else(|| Error::new("unbalanced HID collections"))?;
            }
            (0, 8 | 9) => {
                if collections.is_empty() || g.id != 0 || g.size == 0 || g.count == 0 {
                    return Err(Error::new("invalid HID input/output layout"));
                }
                let bits = g
                    .size
                    .checked_mul(g.count)
                    .ok_or_else(|| Error::new("HID report size overflow"))?;
                let total = if tag == 8 {
                    &mut total_input
                } else {
                    &mut total_output
                };
                *total = total
                    .checked_add(bits)
                    .ok_or_else(|| Error::new("HID report size overflow"))?;
                if *total > 512 {
                    return Err(Error::new(
                        "complete unnumbered HID report exceeds 64 bytes",
                    ));
                }
                if collections.last() == Some(&true) {
                    let selected = if tag == 8 { &mut input } else { &mut output };
                    *selected = selected
                        .checked_add(bits)
                        .ok_or_else(|| Error::new("HID report size overflow"))?;
                }
                usage = 0;
            }
            _ => {}
        }
    }
    if !collections.is_empty()
        || !stack.is_empty()
        || input != 512
        || output != 512
        || total_input != 512
        || total_output != 512
    {
        return Err(Error::new(
            "configuration descriptor requires 64-byte input/output Report ID 0",
        ));
    }
    Ok(())
}
pub(crate) fn acquire_lock(directory: &std::path::Path, path: &str) -> Result<std::fs::File> {
    use sha2::{Digest, Sha256};
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true).mode(0o700);
    builder.create(directory)?;
    if !std::fs::symlink_metadata(directory)?.file_type().is_dir() {
        return Err(Error::new("lock directory must be a real directory"));
    }
    let hash = Sha256::digest(path.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let lock_path = directory.join(format!("{hash}.lock"));
    if std::fs::symlink_metadata(&lock_path).is_ok_and(|m| !m.file_type().is_file()) {
        return Err(Error::new("lock path must be a regular file"));
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(lock_path)?;
    file.try_lock()
        .map_err(|e| Error::new(format!("device busy in another niz session: {e}")))?;
    // Keep this inode in place. Unlinking would let another opener lock a new inode.
    Ok(file)
}

#[cfg(test)]
mod lock_tests {
    use super::*;
    #[test]
    fn locks_contend_for_same_path_and_release_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let owned = acquire_lock(dir.path(), "device-path-a").unwrap();
        assert!(
            acquire_lock(dir.path(), "device-path-a")
                .unwrap_err()
                .message
                .contains("busy")
        );
        assert!(acquire_lock(dir.path(), "device-path-b").is_ok());
        drop(owned);
        assert!(acquire_lock(dir.path(), "device-path-a").is_ok());
    }
    #[test]
    fn lock_files_remain_after_release_with_private_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        drop(acquire_lock(dir.path(), "a").unwrap());
        let files = std::fs::read_dir(dir.path())
            .unwrap()
            .collect::<std::io::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(files.len(), 1);
        let meta = files[0].metadata().unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o600);
    }
}
