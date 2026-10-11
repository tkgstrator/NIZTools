# Exact-target read-only raw threshold API implementation

Verified host: `devonlys-MacBook-Pro.local`. Workspace: `/Users/devonly/Developer/personal/NIZ`. This worker changed only `crates/niz-core/**` and `crates/niz-device/**`. No hardware getter, GUI, vendor program, simulator, Git commit, reset, unlock, flashing, keymap write, or analog-changing command was executed by this worker.

## Public contract

```rust
pub fn supports_raw_thresholds(identity: &niz_core::DeviceIdentity, firmware: &str) -> bool;
pub struct RawThresholds {
    pub identity: niz_core::DeviceIdentity,
    pub firmware: String,
    // private: physical-slot values [u8; 66], original reports Vec<Report>
}
impl RawThresholds {
    pub fn from_reports(identity: DeviceIdentity, firmware: String, raw_reports: Vec<Report>) -> niz_device::Result<Self>;
    pub fn values(&self) -> &[u8; 66];
    pub fn value_for_position(&self, position: usize) -> Option<u8>;
    pub fn raw_reports(&self) -> &[niz_core::Report];
}
impl Session {
    pub fn read_raw_thresholds(&mut self) -> niz_device::Result<RawThresholds>;
}
```

`RawThresholds` derives `Clone`, `Debug`, `PartialEq`, and `Eq`. `value_for_position` is zero-based official physical-slot order, not wire matrix order. The offline/synthetic constructor proves packet shape and the exact metadata pin; it does not prove a live-device origin. GUI demo values must remain explicitly identified as simulated. Serialization is intentionally left to the consumer, allowing CLI diagnostics to include its path/product/timestamp/trace evidence without pretending the snapshot itself authenticates them.

## Qualification and transaction

The only accepted target is VID `0483`, PID `502a`, usage page `008c`, usage `1`, and fresh exact firmware `66EC(XRGB)BLe;V1.0.48;V1.0;`. Other supported transport PIDs do not inherit this capability. Identity preflight and `ready()` reject before any command. The existing `transport::open`, descriptor checks, per-path lock, framing and bounded queue drain are unchanged.

The transaction records one overall 30-second monotonic deadline before the fresh version query. Existing `version_inner` now shares an internal capture helper that preserves the exact F9 report. It keeps the existing strict 64-byte incoming parse and 1-second timeout. A fresh incompatible F9 or a mismatch against an existing retained baseline stops before DC. A missing baseline is allowed for the isolated diagnostic Session API; GUI/worker code must enforce its own live baseline/target/ticket/generation lifecycle.

After a strict zero-time queued-input check at the fresh-F9 boundary, the API sends exactly one zero-filled 64-byte payload `00 DC 01`, wrapped once through the existing native 65-byte HID send method. It receives exactly eleven ordered 64-byte mode-1 rows under the same overall deadline, with existing 1000-ms maximum per-read timeouts. It never retries F9/DC or sends mode 0/2, DF, setters, calibration or recovery commands. It makes one zero-time queued-input probe after the eleven rows; any unexpected residual frame or error fails closed. There is no invented terminator or quiet-window wait.

The root requested stricter ordering than the reviewed plan's permissible row-ID reordering: the shipped decoder requires rows 1 through 11 in their exact statically emitted order. Reorder, duplicate, missing, out-of-range and excess rows fail. An incremental parser error poisons that decoder instance too, so ignoring `push` errors cannot repair it into a later snapshot.

Every transaction I/O or parser failure poisons the Session through its existing mechanism: retained baseline is removed, generation advances, and later calls refuse without another outgoing command. A simple already-poisoned/calibration/identity preflight refusal is not a new transaction. `Error.began_write` remains false, accurately meaning no configuration/analog setter was begun; read request reports are still HID writes. Failures preserve fresh F9 followed by all received DC rows, malformed/truncated/oversize bytes and the optional unexpected queued frame. Success preserves only the eleven original DC rows, including bytes 10 through 63 without guessing a zero-tail predicate.

## Mapping and meaning

The centralized core decoder inverts the physical-number permutation at exact-image address `0x9dc6`, image SHA-256 `b82e11891dfdc138780a81ac635697a7edb0ca1d4e123d6ad38c81a7ba55bf17`. Tests independently use the forward matrix-to-physical-number table and prove all 66 values/unique cell coverage. Nonlinear checks include A/physical 31 -> flat 13, S/32 -> 14, Enter/42 -> 64, and Right/66 -> 59.

These are RAM R table bytes: raw firmware signal units from the normal scanner threshold path. Every byte 0 through 255 is accepted without sentinel-T interpretation. No millimeter/press-distance conversion, current ADC/live stream, active-all-scanner promise, persistence guarantee, or individual setter capability is added. The getter pauses/resets firmware scanner/output state and copies configured/default compensation into the active compensation byte, rather than restoring prior active compensation. It does not change B/T/R tables, the global factor, or flash. API documentation states these runtime effects.

## Test evidence

Test-first missing API runs are saved as `core-red.log` and `device-red.log`; these failed because the new exports/methods did not exist. Initial minimal implementation passed its targeted tests. A subsequent failing behavioral red run `core-poison-red.log` demonstrated that an ignored incremental parser error could previously be repaired; `core-poison-green.log` proves the fail-closed fix.

Fresh package validation commands and results:

- `cargo test -p niz-core -p niz-device`: current revised checkpoint 152 tests passed, zero failed (core codec 31, existing priority 16, raw decoder 6, device 99, plus zero doc-tests). Log: `core-device-review-fixes-test.log`. Original checkpoint log `core-device-test.log` recorded 147 passing tests.
- `cargo clippy -p niz-core -p niz-device --all-targets -- -D warnings`: passed. Current revised checkpoint log: `core-device-review-fixes-clippy.log`; original log also retained as `core-device-clippy.log`.
- `cargo fmt -p niz-core -p niz-device --check`: passed. Current revised checkpoint log: `core-device-review-fixes-fmt.log`; original log also retained as `core-device-fmt.log`.

New device coverage verifies actual 65-byte outgoing bytes (only F9 then DC01), all exact-identity and fresh firmware rejection paths, retained baseline mismatch, success preservation of baseline/generation, malformed partial rows with ordered evidence, short/oversize/missing/error input, prepare failures, short or uncertain output delivery, no retry after poison, stale queue drain, residual extra failure, fake-clock completion deadline, offline parser qualification, and active calibration refusal with no added commands. Core coverage verifies every byte value, opaque tails, full independent permutation, malformed row counts/order/header/modes, no partial table, and poisoned incremental recovery refusal.

The independent Codex pre-probe source review at `/private/tmp/niz-perkey-20261011/firmware/raw-device-preprobe-review.md` identified the three P2 findings documented below. Their revised source still requires a focused independent rereview before hardware. No LocalGPT/Claude Code MCP implementation review is claimed in this report. Preferred LocalGPT/Claude Code MCP tools were not callable in this worker's tool inventory. The coordinating Codex agent must independently read actual source and evidence before a hardware probe, then verify the native target/path/product and runtime outcome. Package checks do not replace that probe or whole-workspace GUI/CLI validation. Source hashes at this checkpoint are in `device-source-sha256.json`.

The optional concurrent `cargo test --workspace` run completed before the coordinator-requested stop could act. The attempted TERM signals found both verified PIDs already absent and stopped no process. Its concurrent `workspace-test.log` is retained but is not final frozen-workspace completion evidence because GUI/CLI source was still changing. Root will rerun the final frozen workspace. No source changed after the hashed checkpoint.


## Corrections after independent pre-probe review

The three P2 findings were checked against actual source and corrected without changing the public API or command allowlist.

1. Post-fresh-F9 input now triggers immediate refusal before DC instead of the previous second silent queue drain. F9 and the unexpected bytes are retained. A poisoned Session refuses later requests. Only the initial stale queue preparation before F9 remains.
2. A `dc_attempted` flag is set before `send`, so short output and uncertain delivery are covered alongside malformed/short/timeout/excess/I/O receive failures. Every error after that point holds the owned Session/transport/native path lock for a passive capture/settling phase: a 2000-ms budget in at most forty 50-ms steps, at most forty nonblocking polls, and at most sixteen appended reports. A failed probe stops further probing but holds the remaining bounded passive wait. Capture limits stop more reads without releasing ownership early. A frozen/broken clock cannot create an infinite loop because of the attempt cap. No output, retry, unlock, reset, or firmware recovery operation is sent. The original error is preserved and diagnostic text explicitly says scanner restoration is not confirmed; settling is an opportunity for the firmware to complete, not proof that it did. The transaction's 30-second receive deadline and this 2-second failure-settling budget are separate.
3. Session API documentation now states that DC pauses/resets scanner output and restores active compensation from the configured/default byte rather than its prior active value; B/T/R/global factor/flash remain unchanged. Normal-scanner scope and lack of mm/current-ADC semantics remain explicit in the snapshot docs.

Behavioral TDD red evidence: `device-review-fixes-red.log` records five failing new tests (11 existing raw tests passed), reproducing silent fresh-F9 extra acceptance and absent passive settling/capture/uncertainty. `device-review-fixes-green.log` records all sixteen targeted raw tests passing. A real filesystem path-lock fixture proves the lock remains contended during every settling wait, the transport is not dropped before return, and ownership releases only after the Session is dropped. Additional cases cover malformed/short/timeout/excess/I/O receive failures, post-DC capture failure preserving the original error, zero new outputs, full received-tail evidence, and finite capture/wait behavior with a clock that never advances.

Final scoped verification after these fixes: `cargo test -p niz-core -p niz-device` exited 0 with 152 passing tests; all-target Clippy with `-D warnings` passed; package fmt check and `git diff --check -- crates/niz-core crates/niz-device` passed. No new workspace-wide test was run while GUI/CLI work was in progress. The current `device-source-sha256.json` hashes the revised eight owned paths; the previous review checkpoint is preserved in `device-source-sha256-before-review-fixes.json`.
