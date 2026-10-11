# Atom66 normal-scan raw threshold hardware read

Date: 2026-10-11 JST. Native coordinator host: `devonlys-MacBook-Pro.local`.
Workspace: `/Users/devonly/Developer/personal/NIZ`.

## Result and qualification

Two separate successful USB reads returned byte-identical complete snapshots: 66 physical-key values in eleven 64-byte DC mode-1 reports. A / physical #31 returned **23 (`0x17`)** and S / #32 returned **25 (`0x19`)**. Enter / #42 returned 28 and Right / #66 returned 30. Values in this snapshot ranged from 16 to 30. This is an observation of this connected keyboard, not a factory-default table or a calibration recommendation.

| Field | Actual target |
|---|---|
| Product | 66EC-XRGB |
| Native path at capture | DevSrvsID:4295010726 |
| VID / PID | 0483 / 502a |
| Usage page / usage | 008c / 1 |
| Fresh firmware | 66EC(XRGB)BLe;V1.0.48;V1.0; |
| Transport | USB HID on Apple Silicon macOS |
| Table | normal-scan threshold R |
| Units | raw firmware signal |

The first read ran at approximately 15:34:28 JST and completed in 0.480 seconds. The second completed in 0.477 seconds. Both invocations exited 0, with empty stderr. The read command was:

```sh
target/debug/niz raw-thresholds --device 'DevSrvsID:4295010726' --json
```

The actual received data is preserved in [the JSON fixture](fixtures/atom66-xrgb-ble-v1.0.48-raw-thresholds.json), SHA-256 `d027723658425b3a42ca631c0c71783eb5b32cefe48dcde343dda67d9491128d`. The fixture contains target/version/units/runtime effects, all 66 values in official physical order, and the original eleven received reports. It does not include live ADC samples, keypress timing, travel measurements, or baseline/held-calibration tables.

## Evidence behind the read

Before probing, an independent Codex worker reviewed the exact firmware image and the actual Core/Device/CLI source. The [initial review](raw-device-preprobe-review.md) raised three issues: silently discarding excess post-F9 input, releasing ownership too early after uncertain DC delivery, and missing active-compensation semantics. All three were fixed with failing-then-passing tests. The [focused rereview](raw-device-preprobe-rereview.md) found no remaining source blocker and independently passed all sixteen raw Device tests. The eight reviewed Core/Device hashes matched at the coordinator's probe and subsequent documentation integration; [the manifest](raw-device-source-sha256.json) records them.

The production API validates the native descriptor and path lock, checks exact identity, asks for a fresh F9 version, and then requests DC mode 1 once. It rejects unsupported firmware, unexpected queued input, malformed/short/oversize reports, unordered/duplicate/missing rows and excess input. Only complete snapshots are published. Failed transactions invalidate the Session and preserve diagnostic bytes. Possible DC delivery retains the owned transport/path lock during bounded passive capture/settling; no retry or recovery command is sent. That wait does not prove hardware restoration.

Every received row in these two captures had the expected `00 DC row 01` header, six value bytes, and 64-byte length. The observed unused tails were zero; the parser preserves opaque tails instead of assuming all firmware must zero them. All 66 physical mappings were independently checked against [the exact-image research CSV](physical-key-to-analog-cell.csv), rather than treating matrix order as keyboard order; [the capture check](raw-hardware-capture-check.json) records this comparison and the mapping hash. Examples A -> flat cell 13, S -> 14, Enter -> 64 and Right -> 59 exercise its non-linear mapping.

## Meaning and limits

The exact firmware's normal scanner compares derived signal values with the R table, with reference/base processing, compensation, confirmation and release hysteresis. These bytes therefore expose internal thresholds, but do not give the millimeters of travel at which a press occurs. They are also not the instantaneous sensor signal or necessarily the boundary used by another scanner mode. [The firmware evidence](per-key-firmware-evidence.md) explains those distinctions.

The DC getter does not persist configuration or alter the B/T/R tables, global factor or flash in the reviewed firmware. It does pause/reset scanner/output state, and returns active compensation to the configured/default value rather than preserving the previous active compensation. This read is consequently not stateless. The CLI, GUI helper, tooltip and snapshot details disclose the runtime effect. An automatic retry, unlock or reset would add an unqualified operation and is not performed.

No keymap setter, analog setter, calibration start, firmware update, global threshold toggle or flash command was sent during these reads. No physical damage was observed, but these two successful reads are not a general hardware-safety guarantee. Failure-path timing/lock behavior was exercised with transport fixtures, not a deliberately interrupted live keyboard. Post-read physical typing recovery was not instrumented as a timing measurement. Bluetooth HID, Intel-native HID and other firmware versions remain unverified.

**Selected-key-only threshold writes remain unsupported.** The ordinary HID dispatcher in the exact image supplies no confirmed position-and-value setter; existing calibration can change multiple keys. Reading R does not establish a way to write one entry, restore it, or turn its byte into a travel slider. No such control is presented as functional.

## Review route and resources

LocalGPT and Claude Code MCP were not callable in the tool inventory. These are explicitly authorized independent Codex fallback reviews; neither preferred service is represented as having reviewed the implementation. The coordinator verified actual source, complete output and captured data independently.

The old coordinator-launched GUI was closed before the separate CLI Session opened; its path lock was not bypassed. No vendor executable or firmware was run/flashed. No Simulator device, window or worker was created. The updated deliverable GUI is the only application intended to remain open for the user after final verification.
