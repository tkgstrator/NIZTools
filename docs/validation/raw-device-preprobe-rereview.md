# Exact-target raw threshold API and CLI: focused pre-probe rereview

2026-10-11. Verified native host `devonlys-MacBook-Pro.local`, workspace `/Users/devonly/Developer/personal/NIZ`. Read-only review of the revised source and fixture tests, followed by an independent targeted fixture-test run. No HID, live Session open/enumeration, keyboard command, GUI, vendor binary, firmware execution, Git operation or repository source edit. Temporary report/test log saved only under `/private/tmp/niz-perkey-20261011/firmware/`.

## Verdict

**Clean: all three prior P2 findings are resolved; no remaining actionable finding blocks the planned exact-target DC mode-1 qualification read.** This is source clearance for the bounded read procedure, not hardware qualification, proof of scanner restoration, an all-mode threshold claim or setter authorization. The static firmware caveats still apply.

## Fixes verified against actual source

1. **Fresh post-F9 excess now stops before DC.** `crates/niz-device/src/session.rs:253..260` uses a nonblocking `queued_input` check after fresh exact F9 qualification, retains unexpected bytes, and returns an error. The second silent prepare/drain has been removed. The only prepare remains before fresh F9. `raw_threshold_tests.rs:406..419` tests a full DC row, short unexpected packet and 65-byte unexpected packet, retaining exact F9+extra evidence, sending only F9, poisoning the Session and performing no settling wait because DC was never attempted. The success fixture proves one initial prepare, two queue probes and only F9/DC01 output (`:173..190`).

2. **Possible DC dispatch retains owned transport/lock during bounded passive cleanup.** `session.rs:267` sets `dc_attempted` before send, so short/error writes with uncertain delivery receive the same cleanup as parse/receive/extra/deadline failures. `:291..297` preserves original evidence/error before invoking `settle_failed_raw_threshold_read`. The helper `:300..347` holds the existing Session/transport/native lock while making only nonblocking input probes and bounded waits. It requests at most 2000ms of waits in forty steps, with at most forty probes and sixteen appended reports; a probe error stops additional probes while the remaining bounded wait is retained. There is no output, retry, D9/unlock, calibration/reset or rescue command in this helper. The original error is preserved and the message explicitly says delivery may have occurred and scanner restoration is not confirmed. The Session is poisoned after cleanup through its unchanged outer `poison` path.

   The actual filesystem-lock fixture at `raw_threshold_tests.rs:423..465` acquires the existing lock implementation, attaches the lock handle to the Fake transport, probes contention during each passive wait, asserts the transport has not been dropped, and verifies release only after Session drop. It covers both short output and output error, captures all eleven pending DC reports, and proves exactly F9/DC01 were sent. Additional fixtures `:469..501`, `:505..530`, `:534..555` cover malformed/short/timeout/excess/I/O responses, no subsequent output after poison, a frozen clock with finite capture/wait caps, and a failed passive probe that preserves the original output error. The 2000ms budget is an opportunity for normal firmware completion; it makes no guarantee after a broken SysTick or device fault.

3. **CLI and API now disclose the exact compensation caveat.** `session.rs:218..224` says the getter restores active compensation from the configured/default byte rather than prior active compensation, leaves B/T/R/global-factor/flash unchanged, and cannot confirm restoration after failure settling. CLI `crates/niz-cli/src/main.rs:321..325` exposes this as structured JSON fields, including `previous_active_compensation_is_not_preserved=true`; text at `:331..333` states the same behavior. Existing `table=normal_scan_threshold_R`, raw-firmware-signal unit, snapshot flag and no-persistent-settings-write labels remain. Snapshot docs still exclude travel/mm/current ADC and all-scanner guarantees.

## Preserved safeguards

The reviewed fixes retain exact VID0483/PID502a/usage008c/1 and fresh exact version `66EC(XRGB)BLe;V1.0.48;V1.0;` gates before DC. Output remains exactly one fresh F9 then one zero-padded DC01; no other mode, terminator, setter or recovery request is introduced. The eleven-row decoder, canonical 64-byte receive/65-byte transmit framing, error poison, no retry, original opaque-tail preservation and exact inverse firmware physical map are unchanged. All66 mapping entries were independently checked in the prior review against the exact image at0x9dc6; unchanged core file hashes confirm that checked map remains the current one.

Native open still acquires the existing per-path process lock, checks the opened device identity, validates 64-byte unnumbered input/output descriptor semantics and retains that handle for the complete transaction/cleanup. No alternate lock or bypass is introduced. The initial bounded stale drain remains the existing transport behavior; it does not retain discarded packets, a known diagnostic limitation. A single nonblocking boundary probe cannot prove future absence of another client's delayed traffic. The planned root probe must continue to close the old GUI cleanly, acquire the lock normally, preserve evidence, and issue a second fresh read only after the first is fully valid and error-free.

## Independent validation

I ran `cargo test --offline -p niz-device raw_threshold_tests::` on the verified host/workspace. Exit0: **16 passed, 0 failed, 83 filtered**. These are reviewed Fake-transport fixtures plus temporary filesystem locks; they do not open a native HID device. Own log: `/private/tmp/niz-perkey-20261011/firmware/raw-device-rereview-tests.log`.

I also inspected the saved revised package log `core-device-review-fixes-test.log`: 31+16+6+99=152 passing tests, zero failures. Revised scoped Clippy log ends successfully and fmt-check log is empty. The saved CLI log records 5 unit tests plus10 integration tests passing; CLI Clippy ends successfully. I did not rerun whole-workspace GUI/CLI tests or claim a live probe result.

All eight current Core/Device files matched `/private/tmp/niz-raw-20261011/device-source-sha256.json`, checkpoint `after-independent-preprobe-review-fixes`, both at initial inspection and final report write. CLI source SHA-256 is `81dc1b90e3860d8fc9ccca4c2c7e65d68ba132148160b6c73c17c260df115662`.

| Reviewed file | SHA-256 |
|---|---|
| `crates/niz-core/src/lib.rs` | `8e1ddeb6ab9e98abccb9fdd6b72ce94967b7a8c48070fc3259a75578c2914a06` |
| `crates/niz-core/src/raw_thresholds.rs` | `9395a105bbcd4bcdefbb950290b80f0838de4aec0c36ec1e4d5be538536b8c15` |
| `crates/niz-core/tests/raw_thresholds.rs` | `4645f27f5096cb09d8ed4d0db00767dae8196c4c2a8fa5e7b9d9255e1147f09e` |
| `crates/niz-device/src/lib.rs` | `55b54224b1d4a74979be25caa153a9a3f207e1f7eee9346f16bd8d062601c468` |
| `crates/niz-device/src/raw_thresholds.rs` | `7016d0234aa7933a167c54b4acf55a0b2949768c1b32f965fdcc62bd35f2f701` |
| `crates/niz-device/src/raw_threshold_tests.rs` | `b5dede0b8e4891c46aab9963f21d2190244c67559d7c0060d36d2da8acc8ca36` |
| `crates/niz-device/src/session.rs` | `a33bd1a3701a2e092f39e497266efa638151b50900cb209b9c90ceba05a515e4` |
| `crates/niz-device/src/tests.rs` | `2fa11bcaa0d90d1fc4b30116b7cf9c2acf70d9a3fba673de3dec95d14e43332b` |

The static firmware source remains `/private/tmp/niz-ui-reference-20261011/target-v1.0.48-private-image.bin`, SHA-256 `b82e11891dfdc138780a81ac635697a7edb0ca1d4e123d6ad38c81a7ba55bf17`. Exact getter behavior and timing proof are in `/private/tmp/niz-perkey-20261011/firmware/raw-read-feasibility-review.md`. Earlier findings remain recorded in `raw-device-preprobe-review.md`; this rereview supersedes its request-changes disposition for the revised hashes above.
