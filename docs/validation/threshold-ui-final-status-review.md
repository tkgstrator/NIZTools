# Final unsupported-status copy review

Independent read-only Codex fallback review on 2026-10-11 in `/Users/devonly/Developer/personal/NIZ`, verified host `devonlys-MacBook-Pro.local`. LocalGPT/Claude MCP availability remains unchanged and neither is claimed as the reviewer.

**Clean: no material findings.** Removing the analysis suffix accurately reflects the user's choice to keep stock firmware without implying that threshold-writing implementation remains in progress. The visible status remains `しきい値の変更は未対応です`.

Compared current files directly with the saved alignment after snapshots and `/private/tmp/niz-write-20261011/ui/disclosure-alignment/source-manifest.json`, without Git. The complete delta is exactly:

- `crates/niz-gui/src/app.rs:363`: one string replacement from `しきい値の変更は未対応です（解析中）` to `しきい値の変更は未対応です`.
- `crates/niz-gui/src/app_tests.rs:1386,1397,1731`: the same replacement in three expected visible strings.

An in-memory whole-file equality check against `before.replace(old, new)` passes for both files, so there are no additional edits. All other 15 frozen GUI source/test hashes match the prior alignment freeze. The header icon callback, layout calls, controls, reader ID and activation, source/target/operation/modal/popup/close guards, protocol, worker, device integration, and calibration paths are unchanged. The shorter warning still uses the same Body label and warning color.

Fresh independent headless tests against the final status:

- `cargo test -p niz-gui --lib raw_threshold_`: exit 0, 11 passed, 0 failed, 1.64 s. Covers the changed manual-read expectations plus disclosure alignment, details, keyboard/AccessKit activation, guards, manual snapshot behavior, error handling, viewport fit, and hover. Log: `final-status-raw-tests.log`.
- `cargo test -p niz-gui --lib per_key_threshold_capability_is_visible_without_legacy_write_controls`: exit 0, 1 passed, 0 failed, 1.00 s. Checks the changed capability expectation for Light/Dark and live/offline states, absent legacy write controls, and no pending operation. Log: `final-status-capability-test.log`.

Current hashes for all 17 GUI Rust source/test files are recorded in `final-status-source-sha256.json`; every current hash remained identical after the tests. Only app.rs and app_tests.rs differ from the prior alignment freeze. No source edits, native UI, HID, Git, normal executable build, distribution, or Simulator operations were performed. Native verification remains with the coordinator. This bounded review does not newly claim firmware setter support or qualify device behavior.
