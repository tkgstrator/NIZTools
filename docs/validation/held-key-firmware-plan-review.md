# Independent selected-key calibration feasibility review

2026-10-11. Native access verified on `devonlys-MacBook-Pro.local`, workspace `/Users/devonly/Developer/personal/NIZ`. Read-only review: no HID reports, lock, calibration, reset, update, native application, Git mutation, or repository edits. This report is the only new file. Other current reviewers' conclusions were not consulted.

## Evidence pin and verdict

The private 42,396-byte image `/private/tmp/niz-ui-reference-20261011/target-v1.0.48-private-image.bin` independently hashes to `b82e11891dfdc138780a81ac635697a7edb0ca1d4e123d6ad38c81a7ba55bf17`. Focused DD, DB, DC, scale and mode-regenerator instructions were reread, and measurement/division/scanner helpers were independently disassembled directly from this image using Capstone. Existing client code was inspected natively.

**The proposed stock-firmware DD-only flow can perform held-key calibration, and normally preserves unheld keys under specific measurement and scale conditions. It cannot enforce a host-selected one-key write mask. Complete pre/post DC mode1 R comparisons can establish the displayed normal-scanner threshold changes for all 66 keys, but cannot prevent or undo a violation.**

**Updated accepted scope from the coordinator:** the user explicitly permits implementing the official held-key flow with the possibility of other keys changing, requires complete before/after comparisons of all 66 thresholds, and retains stock firmware. Under that scope, the minimal existing-DC1 candidate is feasible: retained B, no DB, exact-image-only owned D9/DD/DE flow, complete R/config/RGB evidence before and after, factual target/neighbor change result, no analog rollback promise. B/T reads and persistence verification can remain separate deferred qualification. A UI-selected key is a guide, not a device target. The prior stricter enforced-mask blocker is resolved by this explicit scope choice, not by a newly discovered firmware mask.

## Concrete findings

### P1: DD independently chooses all qualifying measurement cells; the selected position does not constrain it

The normal dispatcher at `0x5e70` calls `0x3268`. DD does not accept or read an input key index/mask; the body constructs its own packet/buffer. At `0x327e..0x3280` it takes a fresh all-66 measurement through `0x5ab8`. It then traverses all eleven 6-cell scan rows:

1. `0x329c..0x32aa`: `d[i] = max(S[i] - B[i], 0)`.
2. `0x32ae..0x32b4`: compute each original row's sum `sum(d)` in a 16-bit accumulator.
3. `0x32c0..0x32d4`: only if `d[i] > 10`, replace the local byte with `(d[i] + floor((sum(d)-d[i])*5/100)) mod 256`.
4. `0x32d8..0x32e4`: if the resulting byte is at least 16, replace `T[i]`; otherwise retain its previous T.
5. `0x32fa`: persist the entire T table; `0x32fe`: regenerate every R from the configured mode.

This is within-row correction, not correction across all 66 keys. The signed division helper `0x168` delegates to unsigned `0x13c`; these nonnegative operands use integer truncation/floor.

**Offline counterexample:** one row B=`[20,20,20,20,20,20]`, old T=`[40,40,40,40,40,40]`, sample S=`[120,31,20,20,20,20]`. Intended target delta is 100; a released neighboring sensor's positive delta is 11, below its normal R=24. DD computes corrected deltas `[100,16,0,0,0,0]`. It persists T=`[100,16,40,40,40,40]`. At mode 1 the neighbor's R changes from 24 to 9. This is a valid firmware-input counterexample, not evidence of its physical occurrence rate. It shows that absence of another reported key press does not prove another calibration cell cannot update.

A sufficient simple condition for T preservation is every unselected sampled delta `<=10`. A more exact condition permits larger deltas whose corrected byte remains `<16` or equals the prior T. Fresh retained B and mechanically released keys make that condition plausible, but neither the host keyboard event stream nor DC reads prove it for the later DD sampling interval.

**Resolution under accepted scope:** require exact complete pre/post R checks and a factual neighbor-change outcome. Do not describe these checks as preventive isolation or rollback. Keep the hold-only-selected-key instruction, with the actual device measurement behavior and unavailable analog restoration in concise details. This is no longer an implementation blocker under the explicitly accepted conditional workflow.

### P1: full R regeneration can change the other 65 thresholds even when their T cells do not change

DD unconditionally calls `0x7624` at `0x32fe`. Mode RAM byte `0x20000959` selects coefficient 20/24/28; `0x754c` recalculates all 66 R cells. For a valid non-FF T, R is `max(9, floor(T*coefficient/40))`. FF is a default sentinel with mode-dependent R=12/14/18.

**Offline counterexample:** retained configured mode is 1 (coefficient 24), old unselected T=40. An earlier D8=28 changed live R to 28 without changing configured mode. DD leaves that cell's T=40 but regenerates R=24. All similarly configured unselected R cells change.

Existing read-only DC0/1/2 gives B/R/T, not the configured mode byte. Inferring configured mode from current R/T is unsound because D8 can make R imitate another mode while leaving the configured mode unchanged. The preflight can detect table shapes inconsistent with ordinary mode regeneration, but a matching shape alone does not prove the configured mode. The candidate must not silently send D8, mode hotkeys or reset as a workaround; those are full-table changes outside this selected-key request.

**Resolution under accepted scope:** classify this as another postflight-detectable risk. All-66 R comparison catches it without introducing a new configured-mode reader or a mode-changing command. Test the clean normal-mode result and the simulated D8 mismatch result. Do not require unqualified B/T reads merely to proceed with the accepted R-comparison scope.

### P1: comparing after DE detects changes after flash erasure; there is no supported arbitrary analog restore

T writer `0x8954` erases the MCU page rooted at `0x10e00` at `0x896c`, then programs 4-byte words over the 66 meaningful bytes (68 programmed bytes) at `0x8972..0x8986`. The return values from erase/program are not used to construct a failure result before DD sends DE. DE therefore means the routine reached its notification, not host-verified persistence or an atomic one-cell transaction.

Power/disconnect/failure during this whole-page write can affect unselected cells even under perfect sensor conditions. There is no established supported setter that can write an arbitrary saved B/T/R snapshot back. Storing preflight data is evidence/diagnosis, not an analog backup with a restore promise. D5 resets all T and cannot be used to restore the old 65 values. Automatic DD retry is also unsafe and must be prohibited.

### P2: existing begin-calibration entry point necessarily sends full released calibration

`crates/niz-device/src/calibration.rs:268-345` creates the current session and unconditionally sends DB at line 337 after D9. DB at `0x31e4` samples all keys, persists the full B table and reference through `0x3206 -> 0x88fc`, then replaces all meaningful B RAM cells (samples >=128 become 16). It is not a selected-only operation. Normal scanner `0x4982..0x4990` subtracts B before comparing against R at `0x4998..0x49a2`, so changing an unselected B can change its effective activation point even if its displayed R number happens to stay identical.

A selected-key candidate cannot be implemented by simply calling current `begin_calibration` with a selected-key label. It needs a separate retained-B preparation branch/state, must never dispatch DB, and must state that it uses the keyboard's retained release baseline. Automatically refreshing DB broadens the analog changes and is outside this minimal accepted candidate. Official initial release+press procedure and retained-B repeated press calibration are distinct paths. The host cannot prove retained B validity from R alone; therefore improvement must be measured/retested after completion, not promised in advance.

### P2: B/R/T collection needs complete mode-specific parsers and owned-stream boundaries

DC handler `0x670c` reads the B/R/T RAM table selected by mode 0/1/2. It does not measure current ADC/sensor values. All three emit eleven 64-byte DC rows; **mode0 additionally emits one DF reference report** at `0x678a..0x67a6`. The physical-to-matrix mapping is shared by these same-size tables.

`crates/niz-core/src/raw_thresholds.rs:33-62` currently requires mode1, and `crates/niz-device/src/session.rs:264-283` reads exactly eleven rows with an excess-input check. Reusing this path for mode0 would reject/leave DF at the boundary. Under the minimal accepted scope, defer DC0/DC2 and retain precisely the existing DC1 framing, physical mapping, opaque tails and provenance. A future B/T collector must consume and validate DF and undergo separate qualification. Keep the owned Session/lock even during user-held waits. After possible DC/DD delivery, a partial stream may not be silently drained and called a fresh run.

### P2: OS key events are guidance; they are neither sensor-mask proof nor target-device identity

`crates/niz-gui/src/input_test.rs:49-73` collects aggregate GUI events, and `:117-125` maps emitted codes back through the current key assignments. Multiple physical slots can have one output assignment; Fn/macro/modifier cases may be absent or ambiguous, and events from another keyboard are indistinguishable. D9 intentionally suppresses normal typing, so it cannot continuously confirm the held target through this stream. A or another weak key may also fail to produce a normal event while still being relevant for calibration.

The UI should show the immutable selected physical legend/position as the hold instruction, freeze selection while the operation owns the device, and require pointer activation with the selected physical key held. Software highlighting may help the user before D9 but must not be used as evidence that the firmware will write precisely that one cell or exclude all other sub-threshold sensor deltas. Never require a normal successful A event as the only eligibility gate for fixing A.

## Minimal candidate under the explicitly accepted official conditional flow

1. Freeze exact device path, VID/PID/usage, fresh F9 firmware, connection generation, selected physical position and displayed legend. Preserve editor/draft locally. No automatic calibration on selection, connection, tab entry, keyboard input or release.
2. Prepare durable keymap/RGB snapshot plus complete pre-R evidence using only the existing exact DC mode1 path, without DB, DC0, DC2 or changing mode. Reject incomplete, duplicate, out-of-order, wrong-mode, changed-version and excess replies before D9/DD. R is a normal-scanner RAM snapshot, not current sensor ADC, stored T, B, travel in mm or proof across all operating modes.
3. Show a short modal with selected key and “hold only this key”; execution/cancel side by side and execution red. Explain retained baseline use and unavailable analog rollback in concise details. Pointer-only execution; no Enter/Space starts a sensor write. Input events remain advisory.
4. Through one device-owner state machine, D9=0 once, DD once, wait boundedly for exactly the fresh DE, complete post-R collection while still D9-locked, then D9=1 once. Keep ownership and selection freeze during all waits. DC1 saves scan-enable 0 and restores 0, so it does not unlock typing; it does restore active compensation from configured/default state. No DB, D5, D8, F0/F1/F6, E0/E6, firmware command or automatic retry. Cancellation before DD emits no DD; cancellation/timeout after possible delivery reports unknown/change possible and performs only justified bounded cleanup.
5. Compare all 66 complete R values and original keymap/RGB under the same identity/session. Report target and neighbor changes as factual normal-scanner R results; distinguish “only selected changed,” “no R change,” and “other keys also changed.” Any missing post-R data prevents a successful comparison claim. Any settings/RGB mismatch is a failure, retains evidence and ends poisoned. Never offer a fabricated analog restore. DD/DE completion is not by itself proof the target improved.
6. Do not claim persistence from RAM readback or DE. A separately user-scoped reconnect/power qualification is needed. Actual flash/power failure exposure remains a property of stock firmware.

This candidate implements official measured held-key calibration with all-66 displayed threshold comparison. It does not establish B/T persistence, an enforced selected-key-only mutation guarantee, analog restoration, or behavior across alternate scanner modes. Acceptance criteria/documentation must retain that distinction.

## Remaining integration requirements for the DC1-only candidate

- `Session::read_raw_thresholds` calls `ready()` at `crates/niz-device/src/session.rs:225-226`, while `ready()` at `:88-97` rejects active calibration. Reuse the same existing mode1 transaction through a narrowly scoped internal method allowed only at the owned pre/post phases; do not expose a generic calibration-exclusive bypass. The private `raw_thresholds_inner` at `:235` currently needs sibling-module access or factoring, with its exact version, framing, excess-input, deadline and passive-settling safeguards unchanged.
- Prefer post-R before unlock. Normal keys and Fn/mode actions remain suppressed while the comparison is gathered. DD saves the scan-enable state at `0x3276..0x327c`, emits DE, and restores that saved state at `0x331e..0x3322`; if it began D9-locked, it ends locked. DC similarly saves/restores it at `0x671e..0x6722` and `0x67aa..0x67ae`. Its `0x6728..0x672a` clears active compensation and resets output; `0x67b0..0x67b4` restores configured compensation, which is a documented additional read effect. Do not label the entire procedure as affecting only an R table cell.
- A DC1 failure after possible dispatch must preserve the owned transport through its existing bounded passive capture/settling. Only after the justified cleanup boundary may the dedicated state machine attempt its one D9=1 cleanup. Preserve original DD/read error, independent unlock outcome, any captured rows and pre-R snapshot. Dropping/reopening and replaying DD or ordinary recovery commands is prohibited.
- The current cancellation flag/state machine must distinguish cancellation before DD, during DE wait, and after possible completed mutation. No canceled/incomplete read may produce a successful 66-value comparison. A fully delivered DD is never described as canceled without changing hardware. Whether evidence capture continues after cancellation or produces an explicit incomplete outcome must be deliberate and tested.
- Because DC mode1 restores active compensation, use the same pre/post reader and describe the measured values as normal-scanner R snapshots. Input test remains useful after unlock, but do not equate an unchanged R with unchanged sensor baseline, compensation, alternative scanning behavior or persisted calibration.
- If post-read completion and one unlock both succeed, a changed-neighbor result is a completed-but-neighbors-changed outcome, not a blanket calibration-failed message that hides the factual changes. Still invalidate the ordinary write baseline and require fresh native settings before another changing action.

## Required offline and hardware evidence

Offline before enabling the minimal native path: exact identity/firmware/path/generation freezing; no changing command on preflight failure; complete DC1 frames before and after; DD non-target delta11/correction counterexample; unchanged T but configured-mode/D8 R mismatch; released DB/DC0/DC2 prohibited; target-only/no-op/all-neighbor change classifications; missing pre/post R data; RGB/keymap mismatch; cancellation before and after attempted delivery; disconnect with no reopen/unlock of another device; wrong/stale/duplicate/late DE; exact trace records; owned operation while pointer/user waits; post-DC retains D9 lock until one unlock; no normal keyboard event prerequisite for a weak A; no retry/hidden restore. Mock sensor/table models should include disputed all-cell effects instead of assuming every DD only edits the selected cell.

Hardware qualification for the minimal scope: actual complete pre-R capture; one identified physical key held through DD completion; full DE timing/frame; fresh complete post-R and keymap/RGB comparison; typing recovery; and truthful retention of any changed-neighbor finding. No hardware calibration occurred during this implementation review. B/reference/T collection and controlled persistence verification are separate later qualification; do not claim them from R data.

## Audit limits

The direct aligned BL scan agrees DD has only normal dispatcher call `0x5e70`, DB only `0x5e6a`, DC only `0x5e7c`; D8 and configured-mode regeneration share `0x754c`. Normal/class-control/PS2 routes inspected do not reveal a supported DD key mask or a current-S diagnostic endpoint. This is a bounded established-interface finding, not a proof that no undocumented exploit or unseen bootloader capability exists. Firmware replacement and malformed-memory writes are outside the user's stock-firmware requirement and this review.
