use clap::{Parser, Subcommand};
use niz_core::{Definition, Profile, key_name, parse_key};
use niz_device::{Error, Result, Session, list_devices, load_profile, save_backup};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
#[derive(Parser)]
#[command(
    name = "niz",
    version,
    about = "ATOM66 macOS configuration utility. Ordinary hardware writes are UNQUALIFIED."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    Devices {
        #[arg(long)]
        json: bool,
    },
    Info {
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        capture: Option<PathBuf>,
    },
    /// Read the exact Atom66 firmware's raw normal-scan threshold snapshot.
    RawThresholds {
        #[arg(long)]
        device: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        capture: Option<PathBuf>,
    },
    Backup {
        #[arg(long)]
        device: Option<String>,
        #[arg(long, default_value = "backups")]
        directory: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        capture: Option<PathBuf>,
    },
    Inspect {
        input: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Diff {
        baseline: PathBuf,
        desired: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Set {
        input: PathBuf,
        #[arg(long)]
        index: usize,
        #[arg(long)]
        key: Vec<String>,
        #[arg(long, default_value_t = 0)]
        kind: u8,
        #[arg(long, default_value_t = 0)]
        interval: u16,
        #[arg(long, default_value_t = 1)]
        cycles: u8,
        #[arg(long)]
        delay: Vec<u16>,
        #[arg(long)]
        output: PathBuf,
    },
    Swap {
        input: PathBuf,
        #[arg(long)]
        from_index: usize,
        #[arg(long)]
        to_index: usize,
        #[arg(long)]
        output: PathBuf,
    },
    TrialSwap {
        #[arg(long)]
        device: String,
        #[arg(long)]
        from_index: usize,
        #[arg(long)]
        to_index: usize,
        #[arg(long)]
        confirm_trial: bool,
        #[arg(long, default_value = "backups")]
        backup_directory: PathBuf,
        #[arg(long)]
        capture: Option<PathBuf>,
    },
    Import {
        input: PathBuf,
        #[arg(long)]
        baseline: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Apply {
        input: PathBuf,
        #[arg(long)]
        device: Option<String>,
        #[arg(long, default_value = "backups")]
        backup_directory: PathBuf,
        #[arg(long)]
        capture: Option<PathBuf>,
    },
    Restore {
        input: PathBuf,
        #[arg(long)]
        device: Option<String>,
        #[arg(long, default_value = "backups")]
        backup_directory: PathBuf,
        #[arg(long)]
        capture: Option<PathBuf>,
    },
    TrialSingleKey {
        #[arg(long)]
        device: String,
        #[arg(long)]
        index: usize,
        #[arg(long)]
        key: String,
        #[arg(long)]
        confirm_trial: bool,
        #[arg(long, default_value = "backups")]
        backup_directory: PathBuf,
        #[arg(long)]
        capture: Option<PathBuf>,
    },
}
fn err(message: impl Into<String>) -> Error {
    Error {
        message: message.into(),
        raw_reports: vec![],
        began_write: false,
        backup_path: None,
        calibration: None,
    }
}
fn json(v: &serde_json::Value) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(v).map_err(|e| err(e.to_string()))?
    );
    Ok(())
}
fn device_path(path: Option<String>) -> Result<String> {
    if let Some(p) = path {
        return Ok(p);
    }
    let devices = list_devices()?;
    match devices.as_slice() {
        [d] => Ok(d.path.clone()),
        [] => Err(err("no supported configuration device found")),
        _ => Err(err(
            "multiple devices found; specify --device PATH explicitly",
        )),
    }
}
fn capture_error<T>(result: Result<T>, path: Option<&Path>) -> Result<T> {
    result.map_err(|mut e|{if let Some(p)=path {let body=serde_json::json!({"format":"niz-raw-diagnostic","error":e.message,"reports":e.raw_reports.iter().map(|r|r.iter().map(|b|format!("{b:02x}")).collect::<String>()).collect::<Vec<_>>(),"began_write":e.began_write,"backup_path":e.backup_path,"calibration":e.calibration});if let Err(failure)=publish(p,serde_json::to_string_pretty(&body).unwrap().as_bytes()){e.message.push_str(&format!("; diagnostic capture failed: {failure}"));}}e})
}
// User-selected draft outputs are create-only to preserve existing captures.
fn publish(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
fn save_draft(path: &Path, p: &Profile) -> Result<()> {
    publish(path, p.to_json()?.as_bytes())
}
fn format_byte(byte: Option<u8>) -> String {
    byte.map(|b| format!("{b:02x}"))
        .unwrap_or_else(|| "absent".into())
}
fn describe_definition(d: &Definition) -> String {
    format!(
        "kind {} keys {:?} interval={} cycles={} custom_delay={} delays={:?}",
        d.kind,
        d.keys.iter().map(|k| key_name(*k)).collect::<Vec<_>>(),
        d.interval,
        d.cycles,
        d.custom_delay,
        d.delays
    )
}
fn describe_diff(a: &Profile, b: &Profile) -> Result<Vec<usize>> {
    let changes = a.differences(b)?;
    eprintln!(
        "Hardware write qualification: UNQUALIFIED. {} changed records:",
        changes.len()
    );
    for &i in &changes {
        let before = a.definition(i)?;
        let after = b.definition(i)?;
        eprintln!(
            "  index {i} (group {}, key {}): {} -> {}",
            i / 66 + 1,
            i % 66 + 1,
            describe_definition(&before),
            describe_definition(&after)
        );
        if before == after {
            eprintln!("    opaque-byte-only change; decoded action unchanged");
        }
        let old = &a.records[i];
        let new = &b.records[i];
        for report in 0..old.len().max(new.len()) {
            for byte in 0..64 {
                let from = old.get(report).map(|r| r[byte]);
                let to = new.get(report).map(|r| r[byte]);
                if from != to {
                    eprintln!(
                        "    report {report} byte {byte}: {} -> {}",
                        format_byte(from),
                        format_byte(to)
                    );
                }
            }
        }
    }
    Ok(changes)
}
fn apply_result_message(result: &niz_device::ApplyResult) -> String {
    let status = match result.outcome {
        niz_device::ApplyOutcome::NoChange => {
            "No changes; no hardware write or postflight/RGB readback was performed"
        }
        niz_device::ApplyOutcome::SettingsVerified => {
            "Settings readback verified; ordinary writes remain UNQUALIFIED"
        }
        niz_device::ApplyOutcome::SettingsAndLightingVerified => {
            "Settings and RGB readback verified; ordinary writes remain UNQUALIFIED"
        }
    };
    format!(
        "{status}. Configuration backup: {}",
        result.backup_path.display()
    )
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Devices { json: as_json } => {
            let devices = list_devices()?;
            if as_json {
                json(&serde_json::json!({"devices":devices,"write_qualification":"unqualified"}))?
            } else {
                for d in devices {
                    println!(
                        "{}\t{}\t{:04x}:{:04x} usage {:04x}/{}",
                        d.path,
                        d.product,
                        d.identity.vendor_id,
                        d.identity.product_id,
                        d.identity.usage_page,
                        d.identity.usage
                    );
                }
                println!("Hardware writes: UNQUALIFIED");
            }
        }
        Commands::Info {
            device,
            json: as_json,
            capture,
        } => {
            let path = device_path(device)?;
            let mut s = Session::open(&path)?;
            let version = capture_error(s.version(), capture.as_deref())?;
            if as_json {
                json(
                    &serde_json::json!({"device":path,"firmware":version,"write_qualification":"unqualified"}),
                )?
            } else {
                println!("Device: {path}\nFirmware: {version}\nHardware writes: UNQUALIFIED");
            }
        }
        Commands::RawThresholds {
            device,
            json: as_json,
            capture,
        } => {
            let path = device_path(device)?;
            let mut s = Session::open(&path)?;
            let snapshot = capture_error(s.read_raw_thresholds(), capture.as_deref())?;
            if as_json {
                let reports: Vec<String> = snapshot
                    .raw_reports()
                    .iter()
                    .map(|report| report.iter().map(|byte| format!("{byte:02x}")).collect())
                    .collect();
                json(&serde_json::json!({
                    "device": path,
                    "identity": snapshot.identity,
                    "firmware": snapshot.firmware,
                    "table": "normal_scan_threshold_R",
                    "unit": "raw_firmware_signal",
                    "is_snapshot": true,
                    "runtime_effects": {
                        "scanning_pauses_during_read": true,
                        "active_compensation_restored_from_configured_default": true,
                        "previous_active_compensation_is_not_preserved": true
                    },
                    "values_by_physical_position": snapshot.values().as_slice(),
                    "reports": reports,
                    "persistent_settings_written": false
                }))?;
            } else {
                println!(
                    "Device: {path}\nFirmware: {}\nRaw normal-scan threshold R snapshot; these are not millimeters.\nReading briefly pauses scanning and restores active compensation from its configured default, not its previous active state.\nNo persistent settings were written.",
                    snapshot.firmware
                );
                for (position, value) in snapshot.values().iter().enumerate() {
                    println!("#{:02}\t{value}\t0x{value:02X}", position + 1);
                }
            }
        }
        Commands::Backup {
            device,
            directory,
            output,
            capture,
        } => {
            let path = device_path(device)?;
            let mut s = Session::open(&path)?;
            let p = capture_error(s.read_profile(), capture.as_deref())?;
            let saved = save_backup(&p, &directory)?;
            if let Some(output) = output {
                save_draft(&output, &p)?;
            }
            println!("{}", saved.display());
        }
        Commands::Inspect {
            input,
            json: as_json,
        } => {
            let p = load_profile(&input)?;
            if as_json {
                json(
                    &serde_json::json!({"firmware":p.firmware,"identity":p.identity,"groups":p.group_count(),"records":p.records.len(),"reports":p.reports().len(),"counters":p.counters.len(),"legacy_root":p.legacy_root(),"write_qualification":"unqualified"}),
                )?
            } else {
                println!(
                    "Firmware: {}\nGroups: {}\nReports: {}\nCounters: {}\nHardware writes: UNQUALIFIED",
                    p.firmware,
                    p.group_count(),
                    p.reports().len(),
                    p.counters.len()
                );
            }
        }
        Commands::Diff {
            baseline,
            desired,
            json: as_json,
        } => {
            let a = load_profile(&baseline)?;
            let b = load_profile(&desired)?;
            let changed = describe_diff(&a, &b)?;
            if as_json {
                json(
                    &serde_json::json!({"changed_records":changed,"write_qualification":"unqualified"}),
                )?
            }
        }
        Commands::Set {
            input,
            index,
            key,
            kind,
            interval,
            cycles,
            delay,
            output,
        } => {
            let mut p = load_profile(&input)?;
            let keys = key
                .iter()
                .map(|k| parse_key(k))
                .collect::<niz_core::Result<Vec<_>>>()?;
            let d = Definition {
                kind,
                keys,
                interval,
                cycles,
                custom_delay: !delay.is_empty(),
                delays: delay,
            };
            p.set_definition(index, &d)?;
            save_draft(&output, &p)?;
            println!("Offline draft saved: {}", output.display());
        }
        Commands::Swap {
            input,
            from_index,
            to_index,
            output,
        } => {
            let before = load_profile(&input)?;
            let mut desired = before.clone();
            desired.swap_keys(from_index, to_index)?;
            describe_diff(&before, &desired)?;
            save_draft(&output, &desired)?;
            println!("Offline swap draft saved: {}", output.display());
        }
        Commands::TrialSwap {
            device,
            from_index,
            to_index,
            confirm_trial,
            backup_directory,
            capture,
        } => {
            if !confirm_trial {
                return Err(err(
                    "bounded swap requires --confirm-trial after approving exact target and inverse swap",
                ));
            }
            if from_index >= 198 || to_index >= 198 || from_index / 66 != to_index / 66 {
                return Err(err(
                    "trial swap requires editable indices on the same layer",
                ));
            }
            eprintln!(
                "EXPERIMENTAL UNQUALIFIED swap on {device}: {from_index} <-> {to_index}. Exact target 66EC(XRGB)BLe;V1.0.48;V1.0; / 0483:502a only. Configuration and RGB snapshot: {}. This does not qualify ordinary writes.",
                backup_directory.display()
            );
            let mut session = Session::open(&device)?;
            let baseline = capture_error(session.read_baseline(), capture.as_deref())?;
            let mut preview = baseline.profile().clone();
            preview.swap_keys(from_index, to_index)?;
            describe_diff(baseline.profile(), &preview)?;
            eprintln!(
                "After a verified trial, reconnect and inspect before explicitly reversing with: niz trial-swap --device '{}' --from-index {from_index} --to-index {to_index} --backup-directory '{}' --confirm-trial",
                device,
                backup_directory.display()
            );
            let result = capture_error(
                session.trial_swap(&baseline, from_index, to_index, &backup_directory),
                capture.as_deref(),
            )?;
            println!("{}", apply_result_message(&result));
        }
        Commands::Import {
            input,
            baseline,
            output,
        } => {
            let a = load_profile(&baseline)?;
            let b = load_profile(&input)?;
            let merged = a.merge_import(&b)?;
            describe_diff(&a, &merged)?;
            save_draft(&output, &merged)?;
            println!("Offline merged draft saved: {}", output.display());
        }
        Commands::Apply {
            input,
            device,
            backup_directory,
            capture,
        }
        | Commands::Restore {
            input,
            device,
            backup_directory,
            capture,
        } => {
            let desired = load_profile(&input)?;
            let path = device_path(device)?;
            let mut s = Session::open(&path)?;
            let baseline = capture_error(s.read_baseline(), capture.as_deref())?;
            let merged = baseline.profile().merge_import(&desired)?;
            describe_diff(baseline.profile(), &merged)?;
            let result = capture_error(
                s.apply(&baseline, &merged, &backup_directory),
                capture.as_deref(),
            )?;
            println!("{}", apply_result_message(&result));
        }
        Commands::TrialSingleKey {
            device,
            index,
            key,
            confirm_trial,
            backup_directory,
            capture,
        } => {
            if !confirm_trial {
                return Err(err(
                    "bounded hardware trial requires --confirm-trial after approving the specific device/key and restore procedure",
                ));
            }
            let code = parse_key(&key)?;
            if index >= 198 || !(1..=74).contains(&code) {
                return Err(err(
                    "trial requires editable index 0..197 and basic key code 1..74",
                ));
            }
            eprintln!(
                "UNQUALIFIED bounded trial: device {device}, record {index} -> {key} ({code}). Durable original backup goes to {}. Ordinary restore remains qualification-gated; exact inverse trial is shown after capture. Device '{}' backup directory '{}'. do not retry after partial failure; reconnect and inspect first.",
                backup_directory.display(),
                device,
                backup_directory.display()
            );
            let mut s = Session::open(&device)?;
            let baseline = capture_error(s.read_baseline(), capture.as_deref())?;
            let before = baseline.profile().definition(index)?;
            eprintln!(
                "Captured source action: kind {} {:?}; changing only record {index}.",
                before.kind, before.keys
            );
            if before.kind == 0 && before.keys.len() == 1 && (1..=74).contains(&before.keys[0]) {
                eprintln!(
                    "To reverse a verified single-key trial after reconnecting and inspecting: niz trial-single-key --device '{}' --index {index} --key '{}' --backup-directory '{}' --confirm-trial",
                    device,
                    key_name(before.keys[0]),
                    backup_directory.display()
                );
            }
            let result = capture_error(
                s.trial_single_key(&baseline, index, code, &backup_directory),
                capture.as_deref(),
            )?;
            println!("{}", apply_result_message(&result));
        }
    }
    Ok(())
}
fn main() {
    if let Err(e) = run(Cli::parse()) {
        eprintln!("error: {e}");
        if !e.raw_reports.is_empty() {
            eprintln!(
                "{} raw diagnostic reports retained; use --capture PATH to save",
                e.raw_reports.len()
            );
        }
        if let Some(path) = e.backup_path {
            eprintln!("Backup: {}", path.display());
        }
        std::process::exit(1);
    }
}
#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    #[test]
    fn raw_threshold_command_is_read_only_and_cannot_select_other_modes() {
        assert!(
            Cli::try_parse_from(["niz", "raw-thresholds", "--device", "device", "--json"]).is_ok()
        );
        for flag in ["--mode", "--set", "--key", "--threshold"] {
            assert!(Cli::try_parse_from(["niz", "raw-thresholds", flag, "1"]).is_err());
        }
    }
    #[test]
    fn capture_saves_exact_readback_bytes_and_partial_write_context() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("readback.json");
        let mut raw = vec![0u8; 64];
        raw[1] = 0xf0;
        raw[2] = 1;
        raw[3] = 10;
        raw[63] = 0xa5;
        let error = Error {
            message: "readback byte mismatch, including opaque/unmodified bytes".into(),
            raw_reports: vec![raw.clone()],
            began_write: true,
            backup_path: Some(dir.path().join("original.json")),
            calibration: None,
        };
        let result: Result<()> = capture_error(Err(error), Some(&path));
        assert!(result.is_err());
        let json: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let expected = raw.iter().map(|b| format!("{b:02x}")).collect::<String>();
        assert_eq!(json["reports"], serde_json::json!([expected]));
        assert_eq!(json["began_write"], true);
        assert!(
            json["error"]
                .as_str()
                .unwrap()
                .contains("readback byte mismatch")
        );
        assert_eq!(
            json["backup_path"],
            dir.path().join("original.json").to_str().unwrap()
        );
    }
}
#[cfg(test)]
mod trial_result_tests {
    use super::*;
    #[test]
    fn changed_result_message_distinguishes_actual_settings_and_rgb_checks() {
        for (outcome, rgb) in [
            (niz_device::ApplyOutcome::SettingsVerified, false),
            (niz_device::ApplyOutcome::SettingsAndLightingVerified, true),
        ] {
            let result = niz_device::ApplyResult {
                profile: Profile::demo(),
                outcome,
                backup_path: "capture.json".into(),
                snapshot: None,
            };
            let message = apply_result_message(&result);
            assert!(message.contains("readback verified"));
            assert_eq!(message.contains("RGB"), rgb);
            assert!(!message.contains("No changes"));
        }
    }
    #[test]
    fn transaction_outcome_serialization_is_explicit() {
        assert_eq!(
            serde_json::to_value(niz_device::ApplyOutcome::NoChange).unwrap(),
            "no_change"
        );
        assert_eq!(
            serde_json::to_value(niz_device::ApplyOutcome::SettingsAndLightingVerified).unwrap(),
            "settings_and_lighting_verified"
        );
    }
    #[test]
    fn noop_trial_message_reports_backup_without_verification_claim() {
        let result = niz_device::ApplyResult {
            profile: Profile::demo(),
            outcome: niz_device::ApplyOutcome::NoChange,
            backup_path: "original.json".into(),
            snapshot: None,
        };
        let message = apply_result_message(&result);
        assert!(message.contains("No changes"), "{message}");
        assert!(message.contains("no hardware write"), "{message}");
        assert!(message.contains("original.json"));
        assert!(!message.contains("readback verified"));
    }
}
