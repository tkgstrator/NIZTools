# Device implementation handoff

Implemented and frozen for integration/review in verified local host `devonlys-MacBook-Pro.local`, workspace `/Users/devonly/Developer/personal/NIZ`. Owned edits are limited to `crates/niz-device`.

## Public interface

- `Session::adjust_sensitivity(&LiveBaseline, &Path, u8) -> Result<SensitivityResult>`.
- `SensitivityResult { pub status: SensitivityStatus }`.
- Exported `SensitivityStatus` derives `Clone`, `Debug`, `Serialize`, with the exact brief fields: `coefficient`, `before`, `after`, `attempted`, `sent`, `settings_verified`, `lighting_verified`, `cancellation_requested`, `backup`.
- Additional public evidence fields: `phase`, `failure_phase`, `original_error`, `trace`.
- `SensitivityPhase`: `Preparing`, `Before`, `Sending`, `After`, `Verifying`, `Complete`, `Aborted`, `Failed`.
- `SensitivityStatus::preparing(coefficient: u8) -> Self` is public for the GUI/demo worker.
- `Error.sensitivity: Option<Box<SensitivityStatus>>` carries failure context, including local gate failures.

Source references: `src/sensitivity.rs:6` phase; `:17` status; `:33` constructor; `:52` result; `:137` device API; `src/lib.rs:10` exports and `:23` error field.

## Transaction and ownership

Checks retained live source/session/generation, exact firmware/identity/groups, current transport identity, and coefficient 10..40 before I/O. Owns a separate sensitivity status before fresh profile/RGB capture. A successful durable compound snapshot is independently reopened/verified before complete pre-DC1 capture. Complete R vectors are in physical order and never constructed from partial rows.

`Session::ready` rejects an active sensitivity owner (`src/session.rs:91`), and `prepare_transaction` uses a quiet probe instead of native queue draining while that owner is active (`:190`). Existing calibration ownership remains unchanged. Common trace captures sensitivity input/output, while common send annotates unknown/short delivery. Raw threshold F9/source validation, queue boundaries, deadline and passive failed-DC settling reuse the existing owned helper.

After quiet/cancellation checks, the baseline is invalidated before D8 dispatch. D8 is native 65 output bytes containing zero-padded `[00,D8,c,...]`, once, without acknowledgement read. Only F9/F2/E2/DC1/D8 are emitted on this transaction. No calibration, saved thresholds, scanner lock/unlock, firmware, config or RGB setter, retry or rollback is introduced.

After successful D8 delivery, one complete fresh DC1 observation is attempted even if cancellation arrives at D8 completion; any in-flight reader finishes under its existing bound. Cancellation returns an aborted status before further verification commands. Unknown/short D8 delivery returns failure without further output. Later keymap/RGB failures retain both complete threshold observations and individual verification flags. Failure retains the original error, input evidence and trace; the session is poisoned. Success leaves the old baseline invalid, requiring an explicit new read.

No guarantee of changed R is made: equal before/after vectors are a valid success. DC1 scan/output/compensation side effects and its non-recalculation behavior are documented in the API.

## TDD and verification

Tests were written before the transaction implementation. Initial compilation identified missing API; then an explicit error-only API skeleton permitted a meaningful runtime RED: `exact_no_ack_d8_once_preserves_settings_and_complete_physical_observations` failed because `adjust_sensitivity` returned `sensitivity transaction not implemented`. Only then was the transaction implemented. Three additional ownership/preflight characterization checks were added after GREEN, with no production change.

Strict transport scripts make replies available only after the exact expected output, reject unexpected/output ordering, reject native prepare calls after the initial baseline, and return no D8 reply. Literals independently assert the 66 physical-order values, one changed physical position and exact RGB snapshot content. Scenarios cover range endpoints/rejection, foreign/stale source, changed settings/firmware/identity, durable backup failure, queued input, no-change observations, uncertain D8 delivery/no retry, malformed/truncated/late/excess/short post-DC evidence and settling, before/after DC cancellation, immediate post-D8 cancellation, retained after evidence on keymap/RGB failure, active owner read/mutation exclusions and preserved calibration ownership.

Verified commands:

- `cargo test -p niz-device sensitivity_tests --lib`: 13 passed, 0 failed.
- `cargo test -p niz-device`: 122 passed, 0 failed; 0 doctests; all existing full/held calibration and raw-threshold tests passed. Final run completed in 12.49 seconds.
- `cargo check -p niz-device`: success, no warnings.
- `cargo fmt -p niz-device -- --check`: success.

No real HID enumeration/open/write, native app, Simulator, Git command, delegation, or other crate edit occurred. Root retains full workspace testing, independent reviews and any actual native API qualification. Device-only success is not hardware qualification.

Changed files: `src/lib.rs`, `src/session.rs`, `src/calibration.rs`; new `src/sensitivity.rs`, `src/sensitivity_tests.rs`.
