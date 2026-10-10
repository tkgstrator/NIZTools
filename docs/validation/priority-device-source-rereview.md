# Priority B focused independent rereview

Date: 2026-10-11. Host/workspace: `devonlys-MacBook-Pro.local`, `/Users/devonly/Developer/personal/NIZ`.

Scope: independently rereview fixes to the two findings in `priority-device-review.md`. Read the fix report and actual changed device/CLI code, inspected new event-driven tests, reran the full device/CLI offline suite, and reran an independently maintained single-queue probe against actual source. No repository source edits, Git operations, real HID access, Simulator actions, or GUI operations. New artifacts are confined to the requested temporary review directory. Original review probes/logs were preserved; rereview probes are in a separate directory.

## Result

No new actionable findings in this scoped rereview. The earlier P1 and P2 are resolved by the reviewed source. This clears those two device/CLI findings for GUI integration; it is not hardware qualification or a review of the forthcoming GUI.

### P1 resolved: calibration-owned preparation no longer silently drains

`crates/niz-device/src/session.rs:174-184` introduces one private `prepare_transaction`. While a calibration run is in any non-Complete phase, it invokes `require_quiet_calibration_input` rather than ordinary `Transport.prepare`. F9, F2, counter, and lighting starts all route through it (lines 186, 231, 292, 423). A source search confirms the only remaining production `.prepare()` call is inside the ordinary branch at line 182. Both Preparing and Verifying are covered, while ordinary public operations still reject calibration ownership via `ready`.

`crates/niz-device/src/calibration.rs:208-221` retains the unexpected packet bytes in both the input trace and raw error reports. Existing before-D9/DB/DD checks remain intact. The failure paths preserve terminal status and do not retry changing commands.

Independent actual-source single-queue probe results:

- Queued DA before begin, after preflight F9, after preflight F2 terminator, and after preflight E2 terminator: begin fails, exact bytes appear in error/trace, no Native-equivalent drain occurs, and no D9/DB/DD is sent.
- Queued DA before postflight F9, behind postflight F9, or behind postflight F2 terminator: finish fails in Verifying, exact bytes appear in error/trace, no drain occurs, and the already-sent unlock is not repeated.
- Clean event-driven calibration completes using responses generated only after each command.
- Positive control verifies the same fake's ordinary read-only preparation really drains its input queue, so the test cannot hide the original bug behind a no-op preparation implementation.

The implementation's new event_tests.rs also covers truncated packets and nonzero-tail DA/DE at preflight boundaries, and those tests passed in this independent run.

### P2 resolved: no-op results and CLI messages distinguish actual verification

`crates/niz-device/src/session.rs:21-32` exposes `ApplyOutcome::{NoChange, SettingsVerified, SettingsAndLightingVerified}`. The no-op branch returns NoChange with snapshot=None and no added traffic (lines 547-553). Changing outcomes are assigned only after successful transaction verification (lines 636-647).

`crates/niz-cli/src/main.rs:236-253` formats the actual outcome. Trial swap, ordinary apply/restore, and trial single-key all invoke that helper (lines 415, 451, 497). NoChange explicitly states that no hardware write or postflight/RGB readback was performed. Settings-only and settings+lighting results have distinct messages. No unconditional trial success string remains on these paths.

The independent probe calls the actual source's same-index `trial_swap`, verifies NoChange/snapshot=None and zero additional output, then passes that result through a verbatim extraction of the current CLI helper. It verifies the no-change statement, backup path, and absence of the old readback-verified claim. The repository suite independently verifies same-index/equivalent-assignment swaps, unchanged remaps, helper variants, and enum serialization.

## Commands and evidence

1. `cargo test -p niz-device -p niz-cli --offline`
   - 78 device tests passed.
   - 10 CLI integration tests passed.
   - 4 CLI unit tests passed.
   - No failures; device/CLI doc tests contained zero tests.
   - Log: `/private/tmp/niz-implementation-20261010/priority-device-rereview-tests.log`.

2. `CARGO_TARGET_DIR=/private/tmp/niz-implementation-20261010/priority-device-independent-probe/target cargo test --offline --manifest-path /private/tmp/niz-implementation-20261010/priority-device-rereview-probe/Cargo.toml probe:: -- --nocapture`
   - 4 independent probe tests passed, covering four preflight injection boundaries, three postflight boundaries, a clean run/ordinary-drain positive control, and no-op outcome/reporting.
   - Probe crate references current production modules by absolute `#[path]`; repository source is not copied or changed. The small CLI message helper is extracted verbatim for the reporting assertion.
   - Log: `/private/tmp/niz-implementation-20261010/priority-device-rereview-probe.log`.
   - One expected unused test-helper warning appears because this separate harness does not invoke `backup_for_test`; no test failed.

Source hashes: `/private/tmp/niz-implementation-20261010/priority-device-rereview-source-sha256.txt`.

## Unchanged limits

This verification covers software and filesystem behavior only. Exact PID502a/V1.0.48 calibration, native device timing/full replies, typing recovery, physical RGB preservation, analog persistence, and vendor recovery remain unqualified. The protocol's absence of transaction IDs still permits an indistinguishable same-opcode stale response arriving after the last successful quiet check. The fix correctly addresses avoidable silent draining without claiming to solve that firmware-level limitation. No GUI conclusion is implied.
