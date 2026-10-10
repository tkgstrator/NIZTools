# Native macOS read-only hardware evidence

Host: devonlys-MacBook-Pro.local, Apple Silicon. Workspace: /Users/devonly/Developer/personal/NIZ. Captured on 2026-10-10 with the Rust CLI, hidapi2.6.7 and macos-shared-device.

The CLI listed one supported configuration collection: USB VID0483, PID502a, usage008c/1, product66EC-XRGB, path DevSrvsID:4295010726. Opening validated the actual descriptor. F9 returned exactly `66EC(XRGB)BLe;V1.0.48;V1.0;`. F2 returned three groups of66 key records (198 reports), parsed successfully. A settings backup was atomically published and reopened. [Saved settings fixture](fixtures/atom66-xrgb-ble-v1.0.48-settings.json) contains no device path or serial identifier.

Commands used: `niz devices --json`, `niz info --device DevSrvsID:4295010726 --json`, `niz backup --device DevSrvsID:4295010726`, and offline `niz inspect ... --json`. No F1/F0 configuration writes or calibration commands were sent. F9/F2 requests themselves use HID output reports; read-only here means no persistent setting changes.

The fixture reveals synchronized normal-layer Fn anchors at key57 (Left Fn166) and key62 (Right Fn156), plus extra Right Fn assignments on layers2/3 key61. The alternate layers also contain raw codes178,179,182 not yet assigned semantics by this port. Current strict validation initially rejected this data for unrelated editing/writing. Priority amendment review requires preserving it unchanged rather than inventing meanings or resetting Fn positions.

This settings fixture does not contain lighting or analog calibration values. Successful parsing and backup do not qualify configuration writes, physical key output, calibration, power-cycle persistence, Bluetooth output or Intel hardware behavior.


On 2026-10-11, a separate native read-only probe read E2 lighting data successfully: exactly198 bytes. The observed terminator is a64-byte report filled with E6, which differs from the zero-prefixed form used by the reference. The captured complete transaction is committed as a golden fixture in `crates/niz-device/tests/fixtures/atom66-xrgb-ble-v1.0.48-rgb-capture.json`; the198-byte payload is independently stored alongside it. Four regression cases cover the actual terminator, alternate framed terminator, counters, and mixed invalid tails. No E1/E0 lighting writes were sent.

The current captured configuration contains action149 at Right Fn/group2, key8 (physical7). Vendor static analysis and the variant manual identify149 as the trigger-point switch. This proves the existing shortcut mapping, not the selected trigger stage or the outcome of a physical shortcut press. See [trigger-point evidence](trigger-threshold-research.md).
