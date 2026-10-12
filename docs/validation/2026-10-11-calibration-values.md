# Atom66 calibration T read and comparison

Date: 2026-10-11 JST. Validation ran on macOS against the connected Atom66 EC-BLE.

The exact `66EC(XRGB)BLe;V1.0.48;V1.0;` image emits eleven `DC` mode-2 reports with six RAM calibration T bytes per row. This is separate from the existing `DC` mode-1 normal-scan threshold R table. The new decoder requires all eleven ordered 64-byte mode-2 reports and maps all 66 cells into official physical-key order. The session requires the exact `0483:502a`, usage `008c:1` identity and a fresh F9 firmware response before sending DC2. An error does not publish a partial table; possible DC delivery is passively settled while the session retains its transport lock.

Official held-key calibration now captures complete R and T tables before and after DD. The GUI shows the selected key's values and counts changes in other keys' T values. A missing post-calibration T read remains explicitly unverified. A separate manual T read includes its own source, read time and connection generation; simulated data is marked DEMO. Neither the getter nor the GUI writes an arbitrary threshold.

## Live read

With the coordinator-launched GUI closed, a single read-only CLI invocation succeeded:

```sh
cargo run --locked -p niz-cli -- calibration-values --json
```

The complete JSON was retained locally at `/private/tmp/niz-t-live-20261011.json` with owner-only permissions (2,627 bytes; SHA-256 `2fe005b12c9e6aba67ebddd0c983e7a568415e8d3aec75f889b58f8b492d523c`). It reported 66 physical positions and exactly eleven headers `00 DC 01 02` through `00 DC 0B 02`; all reports had 64 bytes. A/#31 was 16 (`0x10`), S/#32 was 37 (`0x25`), and Esc/#01 was 29 (`0x1D`); the table ranged from 16 to 43. These three values matched the separately observed GUI read for the same keyboard. The GUI's separate R read displayed 9 for A at this time. They are observations of this keyboard, not factory defaults or a recommended setting.

The freshly built Universal2 `dist/niz` was then used for the same read. Its JSON was byte-identical to the debug CLI capture (same SHA-256), with 66 values and eleven reports. The release app and CLI both contain `arm64` and `x86_64` slices, passed strict ad hoc signature verification, and the ZIP passed an integrity test. Intel HID operation was not tested on Intel hardware.

This reads RAM T, not the on-chip flash page directly. The matching values across GUI and CLI prove the read path and physical mapping for these samples, not power-cycle persistence. The read pauses scanning and can restore active compensation from its configured default. No DD calibration, keymap write, updater handoff or flash command was sent during this validation. The keyboard's physical typing recovery was not instrumented.

## 2026-10-12 read-only follow-up

With no NIZ Utility process running, the same connected `0483:502a` / usage `008c:1` keyboard reported firmware `66EC(XRGB)BLe;V1.0.48;V1.0;`. Separate `calibration-values --json` and `raw-thresholds --json` invocations returned complete eleven-report, 66-position tables. The owner-only captures are `/private/tmp/niz-t-live-20261012.json` (SHA-256 `6c060604872d330c6a6d2ee91e5e99a2875cf5166e98f50306486f96f3ae0c55`) and `/private/tmp/niz-r-live-20261012.json` (SHA-256 `2bdeaf25df6c7d48630d41381898ba048d8c5fc8e3c9194b0b3f27d96bee5fd8`). All 66 T values matched the 2026-10-11 capture; A/#31 remains T=16, R=9. No cold restart was controlled or observed, so matching RAM snapshots across dates do not establish flash persistence. The read commands can pause scanning and restore active compensation to its configured default; no calibration, keymap or flash write was requested in this follow-up.

## Verification scope

Core tests cover mode-2 header/order/count validation and the physical permutation. Device tests cover exact target/firmware pinning, full/partial/error streams, retained diagnostic reports, passive settling, and held-key before/after T comparison including other-key changes and cancellation during both DC2 captures. CLI tests verify the fixed read command has no mode/value setter arguments. GUI tests cover owned target/generation, R/T coexistence, DEMO provenance, and missing post-read display. On the final source tree, `cargo test --locked --workspace --all-targets`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `git diff --check` all passed. Independent Codex reviews of the core/device, GUI, CLI, and feasibility documentation found no remaining material findings.

The numeric per-key persistent setter remains unimplemented. The [feasibility record](../reverse-engineering/2026-10-11-persistent-numeric-thresholds.md) separates storage capability from the missing host-chosen value ingress and the unverified bootloader route.
