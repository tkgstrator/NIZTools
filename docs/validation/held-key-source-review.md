# Independent actual implementation review

Host/workspace: local host, `/Users/devonly/Developer/personal/NIZ`.
Scope: held-key calibration specification and plan, supplied frozen `device-review.diff`, actual device implementation, decoder, transport ownership, and new held calibration tests. No code edits, Git operations, HID I/O, app launches, or redundant full test suite were performed. GUI integration was reviewed from the supplied frozen actual diff and formatted actual source.

## Device verdict

No material correctness findings. The frozen device implementation satisfies the held-only device requirements. This is a source review with deterministic test-log corroboration, not a physical hardware qualification.

- `crates/niz-device/src/calibration.rs:288-299` rejects invalid physical positions before preparation traffic. The held entry calls a shared implementation with an explicit held mode; full mode passes `None`.
- `calibration.rs:333-369` captures fresh settings and RGB, durably saves/verifies their backup, and publishes the complete before snapshot from qualified DC mode 1 before D9. Before/after publication is conditional on the production reader validating all eleven rows.
- `calibration.rs:382-410` enters held wait directly in held mode, bypassing DB/DA. Full calibration retains its released-stage path and exposes no held observation object.
- `calibration.rs:413-435` checks stage/cancellation/quiet input before one DD, then records dispatch and DE completion. Terminal phases prevent repeat DD or finish/cancel retries.
- `calibration.rs:437-458` performs the post DC mode 1 read only after successful DE and while the same session remains locked. `session.rs:235-300` operates on the existing owned `self.io`; it does not reopen the device or bypass metadata qualification. It publishes only complete physical-order values.
- `session.rs:293-355` passively settles any possibly dispatched failed DC transaction under ownership before `calibration.rs:453` invokes cleanup. `calibration.rs:153-170,172-214` bounds cleanup to one eligible D9 unlock and preserves the original failure, packet trace, write state and cleanup outcome.
- `calibration.rs:450-514` retains successful analog snapshots if settings or RGB verification later fails. Missing/partial snapshots remain absent rather than implying unchanged values.
- `session.rs:88-104,225-233` retains the public ordinary-operation guard while calibration owns the session. The internal reuse is crate-scoped, not a new public unguarded reader.

## Device test quality and limits

`held_calibration_tests.rs` uses the production Session, production reader and production decoder, with a strict deterministic Transport script that releases response packets only when the exact expected output is sent. It checks opcode allowlisting, physical-order mapping, lock state during reads and passive waits, single DD/unlock, incomplete/malformed/late/excess captures, short/unknown writes, cancellation at multiple boundaries, before/after retention and later settings/RGB failures. These are meaningful protocol/state tests, with synthetic I/O rather than live-device origin. The inspected `device-held-expanded.log` reports 10 targeted tests passed; the coordinator is responsible for independent full frozen-workspace tests.

The tests and source review do not prove physical key response improvement, flash persistence, equality of stored analog tables, or scanner behavior in all modes. No such provenance is claimed here.

## GUI/integration verdict

One P2 result-provenance finding remains. The held-only execution, ownership, cancellation, and physical-input guards otherwise satisfy the source-level requirements. Resolve the finding before claiming full UI correctness.

### P2: completed calibration values can be displayed against a different selected device

Reference: `crates/niz-gui/src/app.rs:364-367`; related switch path `crates/niz-gui/src/controller.rs:168-178`.

After completing held calibration on concrete device A, select another enumerated device B without reading it. `select_device` clears the raw snapshot and live token but retains the terminal calibration. The new main threshold fallback reads `calibration.held_key.after` with no selected-target check, so it still renders A's value as plain `しきい値` beneath the currently selected device B. The historical summary remains visible without a source qualifier; its original target is available only in collapsed details. This can attribute the normal-R observation to the wrong physical device, although changing-operation guards still prevent a write to B.

Tie fallback measurements to their captured concrete target, or suppress the main threshold value when the selected target differs. Preserve useful historical result evidence with an explicit original-source label. Add a rendered multi-device-switch regression: complete on A, select B, confirm the main measurement is unavailable or explicitly attributed to A, and confirm retained all-66 historical evidence keeps A's source.

### Reviewed integration requirements

- Controller dispatch classifies BeginHeld as an owned calibration and copies its concrete target into the request. It checks pointer activation, valid physical range, fresh attached selection, exact metadata and group count before dispatch. Worker checks monotonic operation, generation, concrete target, baseline validity, exact metadata and physical range before invoking the native entry.
- Worker Held invokes native held once, then calls finish immediately only for the explicit held observation mode in AwaitingFinish. Full calibration returns its separate AwaitingFinish stage. NativeBackend uses the existing Session and baseline; it creates a native Session only in explicit Read.
- BeginHeld is included in pending ownership. Physical selection freezes through both preparation and held/finish work. The actual changing preparation and Execute buttons require `clicked_by(PointerButton::Primary)` and pass the pointer activation gate. Focused Enter/Space and AccessKit click regression tests target real widget IDs, rather than merely testing the controller enum.
- Cancellation and close set the same cancellation flag while work is pending. They defer queued cleanup until the worker response; terminal status suppresses another unlock. Device exceptions convert into Failure without dropping the calibration object, before/after observations, raw bytes, or cleanup evidence. The controller retains the exception's full status.
- UI comparisons require exactly 66 before and 66 after values, report literal selected values, count other changed positions, warn in the theme warning color when neighbors change, distinguish no observed change from unavailable comparison, and preserve complete observations on later settings failure. Collapsed details enumerate all 66 physical legends and literal pairs.
- DEMO routes to a separate backend without native HID construction. Completed and detailed comparisons identify simulated values and do not claim measured hardware origin. UI disclosures correctly limit the comparison to normal-scan observations and explicitly exclude analog backup restoration, stored-table equality, persistence, every mode, and physical improvement.

### GUI test quality and limits

The added rendered App tests exercise production UI and controller behavior for physical freeze, primary-pointer execution, real focused keyboard and AccessKit rejection, result wording, partial-result truthfulness, neighbor attention, popup/modal/close/pending guards, and scrolling to all 66 literal rows. Worker/controller integration tests exercise production routing using the DEMO backend; they are simulation evidence. The inspected `gui-held-green.log` records the relevant ten-test filtered group passing. The supplied new worker/controller tests cover successful automatic finish, replay/terminal repetition, invalid physical positions, foreign targets and stale generations. They do not cover the multi-device result-provenance case above.

Source-reviewed cancellation passes through the shared flag and complete Failure conversion. Existing tests cover controller pending cleanup and worker close; device tests cover cancellation during the actual reader. A combined asynchronous held-only auto-finish post-read cancellation integration test would strengthen the seam, but no source defect was identified there and this is not a separate blocking finding.

No new tests were run during this read-only review; the coordinator owns independent full-workspace verification and native rendered verification. No physical calibration or response improvement was performed by this reviewer.
