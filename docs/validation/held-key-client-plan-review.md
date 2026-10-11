# Independent rereview: official held calibration with all66 normal-R comparison

Native host/workspace rechecked: `devonlys-MacBook-Pro.local`, `/Users/devonly/Developer/personal/NIZ`. No repository edits, hardware/device I/O, app launches, or Git mutations. No other reviewer conclusions inspected. Current source remains the pre-implementation version reviewed previously.

## Accepted scope and verdict

The coordinator reports the user's explicit revised acceptance: use the official held calibration while keeping stock firmware, accept that other keys can change, and compare before/after threshold values for all66 keys. This supersedes the earlier absolute unselected65 preservation requirement.

**The revised bounded plan is feasible and has no remaining plan-level blocker.** Reuse retained released baseline B; add a dedicated held-only entry that captures the already qualified DC mode1 normal-R table before any DD, locks output with D9, waits for the selected physical key to be held and a real mouse click, sends one DD and awaits DE, unlocks once, captures the normal-R table again through the same strictly owned session, compares all66, and separately verifies keymap/RGB consistency. Keep all observations precisely scoped to the captured normal-R snapshots.

The earlier requirements for new DC0/2 readers, shared-reference equality, B/T preservation and conservative temporary-mode rejection are **withdrawn for this accepted scope**. They would add unqualified commands and complexity without proving an invariant the user no longer requires. Possible all-R regeneration from a temporary scale is an observed result to display, not a reason to normalize or reject merely because other values might differ. No DB/D5/D8 or firmware/configuration/RGB write is part of this plan.

## Integration requirements to retain

### 1. Distinct entry and truthful release flags

`crates/niz-device/src/calibration.rs:329-345` and GUI `controller.rs:318-345` currently require DB/DA before held calibration. Add a distinct explicit held-only API and GUI command (`BeginHeldOnly` or equivalent) with frozen selected position; do not route it to existing full `begin_calibration`. Set `release_attempted`, `release_sent` and `release_completed` false throughout this mode. Its preparation result may enter `AwaitingHeldKeys` directly without manufacturing DA. Existing full-calibration CLI behavior and tests remain unchanged. New selected-position values must be range checked (0..65) in both controller and device API before changing commands.

Add the new command to every ownership, exact-target, ticket-generation, replay, cancellation and close classification. `Worker::execute` owns requests at `worker.rs:76-83`, target checks at approximately111-128, and sets `valid=false` before preparation at195-200. `Controller::calibration_active` at94-104 includes pending command kinds; omission of the new begin kind would weaken blocking during preparation.

### 2. Reuse the exact mode1 transaction without weakening public ownership

`session.rs:225` public reader calls `ready()`, so it cannot be called during calibration. Factor or expose a **private owner-aware helper** around the existing inner mode1 transaction. Maintain exact fresh F9 metadata pin, same native session/transport lock, 11 strict rows, physical permutation, quiet boundaries, absolute deadline, opaque-tail evidence, excess-data rejection and passive settling after possible DC delivery. The public reader must still reject an active calibration. No queue draining, blind retry, mode0 DF handling, or new analog mode is necessary.

The pre-read can run in the held-only run's Preparing phase to retain packet trace. Its complete before snapshot must be stored before D9/DD. A read/backup/profile/RGB failure prevents DD (and D9 if it has not yet been sent). The DD-changing send still invalidates the old write baseline; complete readback never makes that old keymap token valid again.

Finish order should be DE observed → user instructed to release selected key → one D9 01 → post mode1 capture and keymap/RGB consistency verification on the same owner. Do not call cleanup unlock a second time when post-read fails. Existing setter send errors/short writes retain physical-delivery uncertainty; they do not trigger a competing write or automatic DD repeat.

### 3. Cancellation, close and failure must preserve observation state

Check cancellation before D9/DD dispatch and after preparation transactions. Existing raw reader is bounded but does not currently consult the cancel flag midstream; it is acceptable to await the owned transaction before the next cleanup decision, rather than issuing overlapping output. Document/test this behavior. If cancellation occurs before DD, no calibration was attempted. If cancellation/close occurs after DD, retain held-attempted/sent/DE flags and the before snapshot; if no complete post snapshot is obtained, explicitly show comparison unavailable, not unchanged. Optional best-effort post-read after safe unlock must stay bounded and use the same handle; disconnect/poisoned/unresolved I/O must never lead to reopen-and-read as if it belonged to this operation.

A finish-stage post-read error must retain raw packet evidence, before snapshot, any complete after snapshot already obtained, unlock outcome and original error. A successful normal-R capture followed by a settings/RGB mismatch must retain its all66 comparison even though overall verification failed. Do not discard useful completed observations because a later consistency check fails.

### 4. Outcomes are completion + change scope, never an invented rollback

Expose operation mode, frozen physical position and captured before/after normal-R values in results. A complete readback with only the selected cell changed can say so. If neighbors also change, report their physical legends/# and before→after values and show an attention state; it is not a protocol failure that can undo the calibration. If no numeric value changes, say no change was observed. DE plus successful consistent readback establishes completed sequence/observations, not improved reaction, storedT/B equality, every scan mode, or persistence after power off.

Keymap/RGB mismatch and incomplete I/O remain actual failures. Retained B validity is not proven by a keymap read, but under the user's official-flow acceptance this is disclosed context rather than a new all-key-calibration requirement. Never silently run DB to refresh B.

### 5. Pointer-only execution must use the actual pointer predicate

Egui0.36.2 `Response::clicked()` accepts focused Enter/Space and accessibility activation. Its `clicked_by(egui::PointerButton::Primary)` explicitly excludes both. Use the latter for the red changing-action button and pass `Activation::Pointer` only after that predicate; do not hard-code Pointer after `.clicked()`. Existing controller Pointer checks are insufficient if the caller labels a keyboard event Pointer.

Freeze selected physical position from preparation until terminal result/cancellation; use official physical legend/# rather than remapped output. Keep selected-key hold instruction concise and persistent, red execution and horizontal cancel, progress/DE/release instruction, all66 comparison with changed keys prominent and the rest available in details. Physical board remains complete and centered; Light/Dark, current disclosure alignment, no hover displacement and no default scrolling remain acceptance criteria. DEMO results must be labeled simulation and cannot masquerade as actual hardware observations.

## Required meaningful tests

1. Held-only exact command trace contains F9/F2/E2/DC mode1/D9/DD only; successful mutation sequence D9 00 → DD 00 once → D9 01 once. All release flags false, no DB/DA required, old full-calibration trace unchanged.
2. All66 physical-index/permutation cases; selected-only/no-change/multiple-neighbor normal-R changes; correct before→after values and no storedT/B/reference/mm/physical/persistence claims.
3. Pre-read and preparation failure/cancel prevent DD; partial/extra/late/wrong row and post-read failure preserve evidence and never produce false unchanged/success. Keymap/RGB mismatch retains complete analog comparison and distinct failure.
4. DD output error/short/late DE/disconnect, cancellation and close in each stage; no blind retries, overlapping unlock, second unlock, foreign handle or stale ticket. Public raw reads and all unrelated worker operations remain blocked during ownership.
5. New command integration in controller/worker/demo; immutable selected position, exact metadata/generation, out-of-range positions, double-click, actual Pointer versus keyboard/AX activation, modal/popup and pending guards, draft/undo preservation.
6. UI compact/default dimensions and Light/Dark; selected hold wording, red execute/cancel row, DE release instruction, result attention for neighbors, DEMO provenance and complete lower keyboard with existing disclosure-arrow rendering tests preserved.

Actual hardware calibration and all66 comparison must be reported separately from deterministic tests. This rereview inspected existing files and the revised plan; it did not execute tests or establish real post-DD evidence.
