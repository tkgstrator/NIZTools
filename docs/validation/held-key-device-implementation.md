# Owned held-key calibration device implementation

Host: `devonlys-MacBook-Pro.local`.
Workspace: `/Users/devonly/Developer/personal/NIZ`.
Scope: Task 1 only. Device-owned source files are frozen for independent review.

Changed files:
- `crates/niz-device/src/calibration.rs`
- `crates/niz-device/src/session.rs`
- `crates/niz-device/src/lib.rs`
- `crates/niz-device/src/tests.rs`
- new `crates/niz-device/src/held_calibration_tests.rs`

## Interface and behavior

`Session::begin_held_key_calibration(&LiveBaseline, &Path, usize)` rejects nonphysical positions before preparation traffic. `CalibrationStatus::held_key` contains exported `HeldKeyCalibration { position, before, after }`. Populated snapshots contain all 66 physical-order bytes. Full calibration retains its original entry and workflow and has `held_key: None`.

Held preparation preserves the existing baseline, exact firmware/identity, current settings, durable snapshot/reparse, lighting and quiet-input gates. It reads qualified DC1 before D9=0, retains the release calibration, and enters AwaitingHeldKeys without DB. DD/DE use the existing owned stage. Finishing captures qualified DC1 after DE while the same D9=0 remains held, then unlocks once and verifies settings/RGB. Complete observations survive later settings/RGB failures. Failed DC1 reads preserve passive capture/settling before any cleanup unlock. Public manual raw reads remain blocked throughout active ownership. Cancellation during a DC transaction completes its bounded read before subsequent actions. Existing DD delivery/cancellation semantics retain attempted/sent/completed flags without falsely claiming DE completion or analog rollback.

The held strict transport only accepts hand-specified F9/F2/E2/DC1/D9/DD outputs; DB, D5, D8, configuration/RGB setters, retries or ordering deviations fail the script. Replies exist only after their exact output; pre/post DC reads record scanner lock ownership.

## Test-first evidence

Read the test-driven-development skill and writing-good-tests reference before test edits. Wrote seven meaningful held workflow/transport tests before production edits. The initial test-local entry shim called the existing full calibration entry to exercise the missing held-only behavior against real Session logic without a missing-symbol compilation error. The strict script required pre-DC capture, so existing preparation failed with `expected f9/0, got d9/0`; the position test caught mutation traffic for an invalid position. This first run compiled and failed all seven tests (0 passed / 7 failed), then the shim was switched to the newly implemented held entry. No production shim remains.

Command: `cargo test -p niz-device held_calibration_tests -- --nocapture`
- `device-red.log`: complete initial red evidence (large packet trace dump).
- `device-red-summary.log`: concise failures/count summary.

Initial green: `cargo test -p niz-device held_calibration_tests`
- `device-green-initial.log`: 7 passed, 0 failed; no warnings.

Expanded composed-path characterization coverage:
- Explicit held-only DD error/short delivery.
- Late/malformed/timed-out DE.
- Cancellation before DD, after dispatch, while receiving DE, after completed DE.
- Public manual-read guard both after preparation and after DE.
- Literal all-66 physical mapping, target/neighbor changes and no change.
- Incomplete/malformed/late/excess-row/short-output post DC captures never publish partial values.
- Pre-capture failure does not lock or send DD.
- DC cancellation completes captures before cleanup.
- Complete observations survive settings or RGB mismatch.

Command: `cargo test -p niz-device held_calibration_tests`
- `device-held-expanded.log`: 10 passed, 0 failed; no warnings.

Final fresh device suite command: `cargo test -p niz-device`
- Native command session 8032 completed with exit code 0.
- 109 unit tests passed, 0 failed, 0 ignored; 0 doc tests.
- No warnings or unfinished failures.
- Earlier `device-suite.log` records 106 passing tests before the last three characterization tests were added; final count is 109.

Formatting: `cargo fmt -p niz-device -- --check` exited 0.
Whitespace: `git diff --check -- crates/niz-device` exited 0.

## Boundaries

No Git commits/push, real HID I/O, device listing/session opening, native application launch, firmware action or subdelegation occurred. All transport evidence is deterministic synthetic evidence. Physical held-key behavior, response improvement, stored analog equality, persistence and all scan modes are not established. Coordinator owns full workspace integration tests, review, clippy, packaging and native GUI verification after source freeze.
