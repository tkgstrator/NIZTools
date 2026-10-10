use std::{fs, process::Command};
fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_niz"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn malformed_inspect_is_nonzero() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("bad.json");
    fs::write(&p, "{}").unwrap();
    let r = run(&["inspect", p.to_str().unwrap(), "--json"]);
    assert!(!r.status.success());
    assert!(String::from_utf8_lossy(&r.stderr).contains("error"));
}
#[test]
fn inspect_and_set_preserve_unknown_bytes() {
    let d = tempfile::tempdir().unwrap();
    let input = d.path().join("a.json");
    let output = d.path().join("b.json");
    let mut p = niz_core::Profile::demo();
    p.records[0][0][63] = 0xa5;
    fs::write(&input, p.to_json().unwrap()).unwrap();
    let result = run(&["inspect", input.to_str().unwrap(), "--json"]);
    assert!(result.status.success());
    let info: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(info["groups"], 3);
    assert_eq!(info["write_qualification"], "unqualified");
    let result = run(&[
        "set",
        input.to_str().unwrap(),
        "--index",
        "0",
        "--key",
        "A",
        "--output",
        output.to_str().unwrap(),
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let actual = niz_core::Profile::from_json(&fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(actual.records[0][0][63], 0xa5);
    assert_eq!(
        actual.definition(0).unwrap().keys,
        vec![niz_core::parse_key("A").unwrap()]
    );
}
#[test]
fn invalid_set_does_not_publish_output() {
    let d = tempfile::tempdir().unwrap();
    let input = d.path().join("a.json");
    let output = d.path().join("b.json");
    fs::write(&input, niz_core::Profile::demo().to_json().unwrap()).unwrap();
    let r = run(&[
        "set",
        input.to_str().unwrap(),
        "--index",
        "999",
        "--key",
        "A",
        "--output",
        output.to_str().unwrap(),
    ]);
    assert!(!r.status.success());
    assert!(!output.exists());
}
#[test]
fn help_lists_commands_and_trial_confirmation() {
    let r = run(&["--help"]);
    let help = String::from_utf8(r.stdout).unwrap();
    for command in [
        "devices",
        "info",
        "backup",
        "inspect",
        "diff",
        "set",
        "import",
        "apply",
        "restore",
        "trial-single-key",
    ] {
        assert!(help.contains(command), "{command}");
    }
    let r = run(&["trial-single-key", "--help"]);
    assert!(String::from_utf8_lossy(&r.stdout).contains("--confirm-trial"));
}
#[test]
fn trial_without_confirmation_never_opens_device() {
    let r = run(&[
        "trial-single-key",
        "--device",
        "definitely-absent",
        "--index",
        "0",
        "--key",
        "A",
    ]);
    assert!(!r.status.success());
    assert!(String::from_utf8_lossy(&r.stderr).contains("confirm-trial"));
}
#[test]
fn diff_reports_exact_changed_record() {
    let d = tempfile::tempdir().unwrap();
    let a = d.path().join("a.json");
    let b = d.path().join("b.json");
    let p = niz_core::Profile::demo();
    fs::write(&a, p.to_json().unwrap()).unwrap();
    let mut q = p;
    q.records[1][0][63] ^= 1;
    fs::write(&b, q.to_json().unwrap()).unwrap();
    let r = run(&["diff", a.to_str().unwrap(), b.to_str().unwrap(), "--json"]);
    assert!(r.status.success());
    let value: serde_json::Value = serde_json::from_slice(&r.stdout).unwrap();
    assert_eq!(value["changed_records"], serde_json::json!([1]));
}
#[test]
fn timing_only_diff_shows_all_behavior_parameters() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.json");
    let b = dir.path().join("b.json");
    let mut p = niz_core::Profile::demo();
    let before = niz_core::Definition {
        kind: 2,
        keys: vec![28, 29],
        interval: 100,
        cycles: 2,
        custom_delay: false,
        delays: vec![],
    };
    p.set_definition(0, &before).unwrap();
    fs::write(&a, p.to_json().unwrap()).unwrap();
    let after = niz_core::Definition {
        interval: 200,
        cycles: 3,
        custom_delay: true,
        delays: vec![321],
        ..before
    };
    p.set_definition(0, &after).unwrap();
    fs::write(&b, p.to_json().unwrap()).unwrap();
    let output = run(&["diff", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert!(output.status.success());
    let preview = String::from_utf8(output.stderr).unwrap();
    for expected in [
        "interval=100",
        "interval=200",
        "cycles=2",
        "cycles=3",
        "custom_delay=false",
        "custom_delay=true",
        "delays=[]",
        "delays=[321]",
    ] {
        assert!(preview.contains(expected), "missing {expected}: {preview}");
    }
}
#[test]
fn opaque_only_diff_identifies_changed_report_byte_and_values() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.json");
    let b = dir.path().join("b.json");
    let mut p = niz_core::Profile::demo();
    fs::write(&a, p.to_json().unwrap()).unwrap();
    p.records[1][0][63] = 0xab;
    fs::write(&b, p.to_json().unwrap()).unwrap();
    let output = run(&["diff", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert!(output.status.success());
    let preview = String::from_utf8(output.stderr).unwrap();
    for expected in ["opaque-byte-only", "report 0 byte 63: 00 -> ab"] {
        assert!(preview.contains(expected), "missing {expected}: {preview}");
    }
}
#[test]
fn offline_swap_moves_raw_records_and_second_swap_restores_fixture() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.json");
    let b = dir.path().join("b.json");
    let c = dir.path().join("c.json");
    let original =
        include_str!("../../../docs/validation/fixtures/atom66-xrgb-ble-v1.0.48-settings.json");
    fs::write(&a, original).unwrap();
    for (input, output) in [(&a, &b), (&b, &c)] {
        let result = run(&[
            "swap",
            input.to_str().unwrap(),
            "--from-index",
            "0",
            "--to-index",
            "1",
            "--output",
            output.to_str().unwrap(),
        ]);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let expected = niz_core::Profile::from_json(original).unwrap();
    let actual = niz_core::Profile::from_json(&fs::read_to_string(c).unwrap()).unwrap();
    assert_eq!(actual.reports(), expected.reports());
}
#[test]
fn trial_swap_requires_explicit_confirmation_before_device_open() {
    let result = run(&[
        "trial-swap",
        "--device",
        "missing",
        "--from-index",
        "0",
        "--to-index",
        "1",
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("confirm-trial"));
}
