# Priority B independent actual-source review

Reviewed on 2026-10-11 on `devonlys-MacBook-Pro.local`, workspace `/Users/devonly/Developer/personal/NIZ`. Scope: all current device and CLI Rust source, the relevant core differential/swap/edit APIs, Priority B plan/spec, implementation report, literal saved XRGB firmware V1.0.48 settings fixture, and calibration research including the native Mac PID502b caveat. No other reviewer's conclusions were read. No repository source was edited, no Git action was performed, and no real HID device or Simulator was accessed. Temporary independent probe files and this report are the only new review artifacts.

Preferred LocalGPT and Claude MCP reviewers were unavailable according to the coordinating assignment; this is the authorized independent Codex fallback review. Source hashes are saved in `priority-device-reviewed-source-sha256.txt` alongside this report.

## Findings

### P1: Calibration preparation silently discards queued notifications before its fail-closed gate

Primary location: `crates/niz-device/src/calibration.rs:294-310`. Supporting locations: `crates/niz-device/src/session.rs:164-167,210-212,402-404`; `crates/niz-device/src/transport.rs:64-76`.

`begin_calibration` installs its private calibration run and then calls `profile_inner` and `lighting_inner` before checking `require_quiet_calibration_input`. Each underlying F9/F2/E2 transaction calls `Native.prepare`, which reads and silently discards every already-queued packet. Therefore a DA/DE/short packet queued before begin, or arriving between preflight streams, never reaches the quiet-input gate or trace. Calibration still sends D9/0 and DB. This contradicts the required behavior that queued calibration input fails closed and is retained, and hides evidence of unexpected prior calibration activity before a new changing operation.

I reproduced both cases against the actual source modules with an independent event-driven fake that generates responses only when commands are written, uses one shared input queue for normal reads and quiet probes, and mirrors Native.prepare's bounded nonblocking drain:

1. Read a valid live baseline from the literal settings fixture; enqueue stale `00 DA ...`; call begin. The stale DA is silently drained, D9/0 and DB are sent, and begin returns AwaitingHeldKeys.
2. Read the baseline; enqueue a stale DA immediately behind the fresh preflight F2 terminator. The E2 preparation silently drains it, D9/0 and DB are sent, and begin again succeeds.

The existing regression at `crates/niz-device/src/tests.rs:1672-1696` misses this because the main Fake implements a separate unsolicited queue for `queued_input` but inherits the no-op `Transport.prepare` (tests.rs:31-96; transport.rs:46-48). The native quiet-probe unit test verifies only the probe itself, not its interaction with earlier drain calls.

Fix suggestion: introduce calibration-aware transaction preparation that fails on and traces any queued packet instead of invoking silent Native.prepare throughout calibration-owned preflight and postflight. A single check at entry is insufficient because packets can arrive between F9/F2/E2 phases. Retain the before-D9/DB/DD gates. Test using an event-driven single input queue with production-equivalent preparation; assert no D9/DB on the above preflight failures. This does not eliminate the separately documented no-transaction-ID race after a successful quiet check.

### P2: No-op trial-swap claims readback/RGB verification that never happened

Primary location: `crates/niz-cli/src/main.rs:397-399`. Supporting location: `crates/niz-device/src/session.rs:526-533`.

A valid same-index swap, or two raw-identical basic assignments, yields no changed records. The intended device behavior is to save the configuration backup and return without requerying or writing anything; its result has `snapshot: None`. The CLI nevertheless prints “Settings and RGB readback verified.” This asserts an RGB read and post-transaction verification that did not occur and can mislead a user recording experimental qualification evidence. The independent probe confirmed `trial_swap(&baseline, 0, 0, ...)` returns success, snapshot=None, and sends zero additional commands (including no E2).

Fix suggestion: report an explicit no-change result and configuration-backup path when the preview has zero differences or when a dedicated transaction outcome indicates no-op. Only print settings/RGB readback verification for a changing transaction that actually completed both checks. Add an output-format test for same-index/equivalent assignments. The analogous single-key success wording should also distinguish a no-op to an already-selected output.

## Verification performed

- Ran `cargo test -p niz-device -p niz-cli --offline` against the shared workspace: 73 device tests, 10 CLI integration tests, and 1 CLI diagnostic unit test passed; no failures. These were executed independently, not inferred from the implementation report.
- Created `/private/tmp/niz-implementation-20261010/priority-device-independent-probe/`, compiling actual source modules via absolute `#[path]` references without modifying them. Ran `cargo test --offline --manifest-path /private/tmp/niz-implementation-20261010/priority-device-independent-probe/Cargo.toml probe:: -- --nocapture`: 2 independent observational probes passed, confirming both findings. Log: `/private/tmp/niz-implementation-20261010/priority-device-independent-probe.log`. “Passed” here means assertions documenting the current problematic behavior succeeded, not that the safety requirement passed. The initial harness had a RefCell borrow-lifetime bug; that harness-only bug was corrected before the recorded successful reproduction.
- Inspected the actual filesystem backup publication/reopen tests and their execution: write_all/short writes, file sync, no-clobber publication, directory/ancestor sync, config reparse, manifest hashes, RGB length, and tampering of each snapshot artifact are covered. Existing shared-pipeline fault tests all passed.
- Inspected all command constructors/call sites: calibration only emits F9/F2/E2 reads and D9/DB/DD changing commands; RGB write commands and analog rollback are not present.

## Checks without additional findings

- Ordinary qualification remains empty; exact firmware/VID/PID/usage/groups gate bounds the two experimental APIs. Trial desired profiles are regenerated from the private live baseline, and one-/two-key basic restrictions and Fn-column exclusions precede changing traffic.
- Baseline-relative validation preserves unchanged opaque records; future private basic qualification rejects Fn/special changes and unsupported raw padding/reserved-byte deltas. Exact understood reciprocal record moves are distinct from arbitrary padding edits.
- Configuration trials recheck current settings, capture 198 RGB bytes, durably publish and independently reopen the compound snapshot before F1, transmit once, and compare all settings and RGB bytes afterward. The reviewed error paths do not retry or silently restore.
- Calibration owns state and the original transport, invalidates baselines, enforces DA/user action/DD/DE ordering, accepts opaque response tails without inventing units, and uses an absolute 10-second receive deadline with 500-ms slices. Errors distinguish returned uncertain output from eligible input failure; only eligible resolved-output input failure can attempt one cleanup unlock. Pending-write/user-wait ownership and OS-lock retention have a passing real-thread offline test.
- No automatic ordinary-write qualification follows simulated or readback success. The first-party native Mac binary's PID502b, short output behavior, and duplicated opcode response prefix do not qualify the user's PID502a/V1.0.48; the implementation conservatively retains exact 64-byte payloads and treats actual calibration behavior as experimental.

## Remaining limits

The fake and filesystem tests establish software behavior only. No target hardware calibration, native full response capture, analog persistence, typing recovery, actual RGB preservation, physical Intel execution, or vendor recovery procedure was tested by this review. The protocol still cannot distinguish a same-opcode stale response arriving after the last quiet check because requests have no transaction IDs. Those limitations are documented appropriately; the P1 finding above concerns avoidable silent draining before that check. GUI absence was explicitly out of review scope.

Assessment: resolve the P1 preflight drain gap before enabling calibration in the GUI or performing a real-device experiment; correct the P2 evidence wording before presenting trial output as verified.
