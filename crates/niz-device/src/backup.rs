use crate::{Error, Result};
use niz_core::{MAX_FILE_SIZE, Profile};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
pub fn load_profile(path: &Path) -> Result<Profile> {
    let file = File::open(path)?;
    if file.metadata()?.len() > MAX_FILE_SIZE as u64 {
        return Err(Error::new("profile exceeds 4 MiB"));
    }
    let mut bytes = vec![];
    file.take(MAX_FILE_SIZE as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_FILE_SIZE {
        return Err(Error::new("profile exceeds 4 MiB"));
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| Error::new("profile is not UTF-8"))?;
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("pro"))
        || text.trim_start().starts_with('<')
    {
        Ok(niz_core::import_windows_profile(text)?)
    } else {
        Ok(Profile::from_json(text)?)
    }
}
fn write_bytes(writer: &mut dyn Write, bytes: &[u8]) -> Result<()> {
    writer.write_all(bytes)?;
    Ok(())
}
trait BackupIo {
    fn write_all(&mut self, text: &[u8]) -> Result<()>;
    fn sync_file(&mut self) -> Result<()>;
    fn publish(&mut self) -> Result<PathBuf>;
    fn sync_directory(&mut self) -> Result<()>;
    fn reparse(&mut self, path: &Path) -> Result<Profile>;
}
struct NativeBackup {
    temp: Option<tempfile::NamedTempFile>,
    directory: PathBuf,
    prefix: String,
}
impl BackupIo for NativeBackup {
    fn write_all(&mut self, text: &[u8]) -> Result<()> {
        write_bytes(self.temp.as_mut().unwrap(), text)?;
        Ok(())
    }
    fn sync_file(&mut self) -> Result<()> {
        self.temp.as_mut().unwrap().as_file().sync_all()?;
        Ok(())
    }
    fn publish(&mut self) -> Result<PathBuf> {
        let temp = self.temp.take().unwrap();
        let name = temp.path().file_name().unwrap().to_string_lossy();
        let path = self.directory.join(format!(
            "{}-{}.json",
            self.prefix,
            name.trim_start_matches('.')
        ));
        temp.persist_noclobber(&path)
            .map_err(|e| Error::new(e.error.to_string()))?;
        Ok(path)
    }
    fn sync_directory(&mut self) -> Result<()> {
        let mut path = Some(self.directory.as_path());
        while let Some(directory) = path {
            if !directory.as_os_str().is_empty() {
                File::open(directory)?.sync_all()?;
            }
            path = directory.parent();
        }
        Ok(())
    }
    fn reparse(&mut self, path: &Path) -> Result<Profile> {
        load_profile(path)
    }
}
fn commit_bytes(bytes: &[u8], io: &mut dyn BackupIo) -> Result<PathBuf> {
    io.write_all(bytes)?;
    io.sync_file()?;
    let path = io.publish()?;
    io.sync_directory()?;
    Ok(path)
}
fn commit_backup(profile: &Profile, io: &mut dyn BackupIo) -> Result<PathBuf> {
    let text = profile.to_json()?;
    let path = commit_bytes(text.as_bytes(), io)?;
    let reparsed = io.reparse(&path)?;
    if reparsed.firmware != profile.firmware
        || reparsed.identity != profile.identity
        || reparsed.records != profile.records
        || reparsed.counters != profile.counters
        || reparsed.captured_at != profile.captured_at
    {
        return Err(Error::new("backup reparse differs from capture"));
    }
    Ok(path)
}
pub fn save_backup(profile: &Profile, directory: &Path) -> Result<PathBuf> {
    fs::create_dir_all(directory)?;
    let temp = tempfile::NamedTempFile::new_in(directory)?;
    let mut io = NativeBackup {
        temp: Some(temp),
        directory: directory.to_owned(),
        prefix: "niz-backup".into(),
    };
    commit_backup(profile, &mut io)
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Fault {
        stage: usize,
        calls: usize,
        p: Profile,
    }
    impl Fault {
        fn step(&mut self) -> Result<()> {
            self.calls += 1;
            if self.calls == self.stage {
                return Err(Error::new("injected boundary failure"));
            }
            Ok(())
        }
    }
    impl BackupIo for Fault {
        fn write_all(&mut self, _: &[u8]) -> Result<()> {
            self.step()
        }
        fn sync_file(&mut self) -> Result<()> {
            self.step()
        }
        fn publish(&mut self) -> Result<PathBuf> {
            self.step()?;
            Ok("backup.json".into())
        }
        fn sync_directory(&mut self) -> Result<()> {
            self.step()
        }
        fn reparse(&mut self, _: &Path) -> Result<Profile> {
            self.step()?;
            Ok(self.p.clone())
        }
    }
    #[test]
    fn every_durable_boundary_must_succeed_in_order() {
        let p = Profile::demo();
        for stage in 1..=5 {
            let mut io = Fault {
                stage,
                calls: 0,
                p: p.clone(),
            };
            assert!(commit_backup(&p, &mut io).is_err());
            assert_eq!(io.calls, stage);
        }
        let mut io = Fault {
            stage: 6,
            calls: 0,
            p: p.clone(),
        };
        assert!(commit_backup(&p, &mut io).is_ok());
        assert_eq!(io.calls, 5);
    }
    #[test]
    fn rgb_and_manifest_bytes_share_all_durable_boundaries() {
        for bytes in [&[0u8; 198][..], br#"{"snapshot":"independent"}"#.as_slice()] {
            for stage in 1..=4 {
                let mut io = Fault {
                    stage,
                    calls: 0,
                    p: Profile::demo(),
                };
                assert!(commit_bytes(bytes, &mut io).is_err());
                assert_eq!(io.calls, stage);
            }
            let mut io = Fault {
                stage: 5,
                calls: 0,
                p: Profile::demo(),
            };
            assert!(commit_bytes(bytes, &mut io).is_ok());
            assert_eq!(io.calls, 4);
        }
    }
    #[test]
    fn durable_reparse_detects_opaque_corruption() {
        let p = Profile::demo();
        let mut bad = p.clone();
        bad.records[0][0][63] ^= 1;
        let mut io = Fault {
            stage: 6,
            calls: 0,
            p: bad,
        };
        assert!(commit_backup(&p, &mut io).is_err());
    }
}
#[cfg(test)]
mod short_write_tests {
    use super::*;
    struct Partial {
        bytes: Vec<u8>,
        fail: bool,
        interrupted: bool,
    }
    impl Write for Partial {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.interrupted {
                self.interrupted = false;
                return Err(std::io::Error::from(std::io::ErrorKind::Interrupted));
            }
            if self.fail && self.bytes.len() >= 4 {
                return Err(std::io::Error::from(std::io::ErrorKind::WriteZero));
            }
            let n = bytes.len().min(2);
            self.bytes.extend_from_slice(&bytes[..n]);
            Ok(n)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn real_write_all_handles_short_writes_and_interruptions() {
        let bytes = b"independent fixture";
        let mut writer = Partial {
            bytes: vec![],
            fail: false,
            interrupted: true,
        };
        write_bytes(&mut writer, bytes).unwrap();
        assert_eq!(writer.bytes, bytes);
    }
    #[test]
    fn short_write_failure_is_error_with_partial_data() {
        let mut writer = Partial {
            bytes: vec![],
            fail: true,
            interrupted: false,
        };
        assert!(write_bytes(&mut writer, b"independent fixture").is_err());
        assert_eq!(writer.bytes, b"inde");
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SnapshotPaths {
    pub config_path: PathBuf,
    pub lighting_path: PathBuf,
    pub manifest_path: PathBuf,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotManifest {
    format: String,
    schema: u8,
    firmware: String,
    identity: niz_core::DeviceIdentity,
    captured_at: String,
    config_file: String,
    lighting_file: String,
    config_sha256: String,
    lighting_sha256: String,
}
fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn publish_companion(directory: &Path, prefix: &str, bytes: &[u8]) -> Result<PathBuf> {
    let temp = tempfile::NamedTempFile::new_in(directory)?;
    let mut io = NativeBackup {
        temp: Some(temp),
        directory: directory.to_owned(),
        prefix: prefix.into(),
    };
    commit_bytes(bytes, &mut io)
}
pub(crate) fn save_snapshot(
    profile: &Profile,
    lighting: &[u8],
    directory: &Path,
    backup: fn(&Profile, &Path) -> Result<PathBuf>,
) -> Result<SnapshotPaths> {
    if lighting.len() != 198 {
        return Err(Error::new("lighting snapshot must have 198 bytes"));
    }
    let config_path = backup(profile, directory)?;
    let config_bytes = fs::read(&config_path)?;
    let lighting_path = publish_companion(directory, "lighting", lighting)?;
    let manifest = SnapshotManifest {
        format: "atom66-config-rgb-snapshot".into(),
        schema: 1,
        firmware: profile.firmware.clone(),
        identity: profile.identity.clone(),
        captured_at: profile.captured_at.clone(),
        config_file: config_path
            .file_name()
            .ok_or_else(|| Error::new("config filename absent"))?
            .to_string_lossy()
            .into(),
        lighting_file: lighting_path.file_name().unwrap().to_string_lossy().into(),
        config_sha256: digest(&config_bytes),
        lighting_sha256: digest(lighting),
    };
    let manifest_path = publish_companion(
        directory,
        "snapshot-manifest",
        &serde_json::to_vec_pretty(&manifest).map_err(|e| Error::new(e.to_string()))?,
    )?;
    let paths = SnapshotPaths {
        config_path,
        lighting_path,
        manifest_path,
    };
    verify_snapshot(&paths, profile, lighting)?;
    Ok(paths)
}
pub(crate) fn verify_snapshot(
    paths: &SnapshotPaths,
    profile: &Profile,
    lighting: &[u8],
) -> Result<()> {
    let manifest: SnapshotManifest = serde_json::from_slice(&fs::read(&paths.manifest_path)?)
        .map_err(|e| Error::new(e.to_string()))?;
    let config = fs::read(&paths.config_path)?;
    let rgb = fs::read(&paths.lighting_path)?;
    let reparsed = load_profile(&paths.config_path)?;
    if manifest.format != "atom66-config-rgb-snapshot"
        || manifest.schema != 1
        || manifest.firmware != profile.firmware
        || manifest.identity != profile.identity
        || manifest.captured_at != profile.captured_at
        || manifest.config_file != paths.config_path.file_name().unwrap().to_string_lossy()
        || manifest.lighting_file != paths.lighting_path.file_name().unwrap().to_string_lossy()
        || manifest.config_sha256 != digest(&config)
        || manifest.lighting_sha256 != digest(&rgb)
        || rgb != lighting
        || rgb.len() != 198
        || reparsed.to_json()? != profile.to_json()?
    {
        return Err(Error::new(
            "compound settings/RGB snapshot verification failed",
        ));
    }
    Ok(())
}
