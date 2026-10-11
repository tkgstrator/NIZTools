# NIZ GUI reference redesign: independent feasibility review

Reviewed on devonlys-MacBook-Pro.local in /Users/devonly/Developer/personal/NIZ.
Verified clean HEAD f78d1617405e91387f03515df469d32c9e6ca5d4. Native file access was verified before review.
Read-only repository review; no source edits, Git mutation, native GUI, HID, or Simulator operations.
HHKB/VIA/other software may inform presentation only. No new protocol, threshold, calibration, or layer capabilities are inferred from those products.

## Conclusion

The bounded GUI-only plan is feasible. Existing App/editor/controller separation supports a compact device toolbar, clear tabs, a selected-key card above the existing board, smaller consistent typography, layer controls, distinct draft/apply status, and simpler calibration cards without changing backend qualification. Proceed once the constraints below are included in the implementation acceptance criteria. No architectural rewrite or new GUI framework is warranted.

## Material constraints and findings

### P2: A connection badge must represent the attached baseline, not USB enumeration

References: crates/niz-gui/src/controller.rs:145-195, 344-419; crates/niz-gui/src/app.rs:488-560.

`selected_device()` indicates the selected enumerated descriptor. It does not prove that the displayed draft belongs to that device or that a fresh native baseline exists. Selecting B while retaining A's draft intentionally sets `live=false`; settings application and calibration also invalidate `live` after completion. A compact header saying simply "connected" or "ready" from `devices`/`selected_path` can make an old/offline draft appear attached and applicable.

Use product/selected descriptor as selection, and a separate explicit state derived from `live`, `attached_target`, pending, failure, and offline editor presence. Keep "read again"/detached status visible when relevant. Never re-enable Apply from the badge; keep `exact_trial`, `prepare_trial`, `approve_trial`, and worker validation unchanged.

### P2: Compressing the editor is insufficient unless actual frame geometry is checked

References: crates/niz-gui/src/app.rs:47-55, 176-269, 581-610; crates/niz-gui/src/main.rs:4-10; crates/niz-gui/src/keyboard.rs:274-315.

The current default window is 960×820, minimum 700×580. Existing outer vertical scrolling permits reachability but the repeated headings, 16-point body/button font, 14×9 button padding, multi-line editor, and legend push the board below the fold. A headless run of the actual current App with its explicit demo backend emitted 66 board label shapes at the default size, with only 55 fully visible; at minimum size no key labels were visible at the initial scroll position. See frame-probe.rs and frame-probe.log in this directory.

Acceptance: at the default size, all five board rows and their selection/draft context should be visible without scrolling; at minimum size, every board key and draft/apply/file/calibration control must remain vertically reachable with no horizontal overflow. Fixed-width ComboBoxes use width as a minimum and long advanced/current labels may expand, so test long product/path/current-action strings and use responsive width/wrapping or truncation with accessible detail. Keep immutable 0..65 geometry, duplicate outputs, actual current labels, and official legend separation.

### P2: Clearer tabs/cards must not hide a running calibration's status or cancellation

References: crates/niz-gui/src/app.rs:312-440, 588-592; crates/niz-gui/src/controller.rs:82-103, 252-300.

The current tabs are always navigable, while the calibration cancel control is rendered only inside the calibration view. With a more polished tab bar, users may leave the running procedure and lose sight of its active stage/cancel action. The backend still owns the device and blocks unrelated operations, so this is a visibility/recoverability issue rather than a missing protocol gate.

Either keep the calibration tab selected/disable switching during active stages, or keep an always-visible active-stage status and cancel action outside the tab content. Preserve pointer-only start/held/finish activation, physical A instructions, pending/close gating, and waiting for owned cleanup. Manual trigger guidance and the typing field must remain visually separate from the experimental analog procedure.

### P2: Disclosures should hide diagnostics, not actionable failures or irreversible calibration consent

References: crates/niz-gui/src/app.rs:301-340, 394-438, 596-607; crates/niz-gui/src/controller.rs:432-450.

Moving byte/firmware/path/backup details under disclosures is appropriate. Keep a concise actionable failure/pending/detached state outside them, with a clear way to inspect the original error and uncertain unlock result. The irreversible analog limitation and explicit consent should stay visible beside Start; do not reduce them to a generic "calibrate" button whose warning exists only in a disclosure. Do not add threshold millimeters, numeric levels, sensor charts, per-key analog parameters, or current-stage readouts: these are not implemented/qualified.

## Integration requirements

- Preserve `Editor::select_from`, `select_layer`, `choose_output`, and raw-record-aware `replace_draft`; no direct `to_code` assignment that bypasses readiness.
- Board clicks and From/To selections are navigation only. They never create a draft change, Undo entry, Trial, or calibration request. Preserve explicit Add and confirmation.
- Preserve `KeyboardState` swap targeting and mode-reset rules. A and B outputs may be duplicates and must not be used as key identity.
- Preserve board blocking for busy, confirmation, open popup, and the popup-closing frame. A UI restructure must not discard `popup_was_open` (app.rs:166-167, 233-234).
- A true modal or a single screen-wide confirmation block is preferable to a floating confirmation window with active file/device controls behind it. The existing approval gate correctly rejects target/draft/selection changes; do not relax it to compensate for background interaction.
- Name labels consistently: physical position/official legend, current draft output, read baseline output, and output choice are distinct. A dirty draft's board label must not be presented as already applied device state.
- Retain file editing/backups and Undo/Redo as compact controls/disclosures. Exporting a draft must not claim hardware application or analog rollback.
- Preserve the fallback key-position selectors for keyboard/accessibility use if the visible board omits numeric slot labels; keep each board widget's accessible label unique, including slot/layer/current output.

## Verification proportionate to the change

Existing GUI behavior tests already cover readiness/import/Undo/Redo, unsupported data, stable slots, duplicate assignments, swap targeting, popup/busy/confirmation guards, native attachment, exact bounded trials, and pointer-only calibration. Keep these green and add tests only for new interaction or actual geometry regressions.

The pinned eframe/egui 0.36.2 already exposes `CreationContext::_new_kittest`, `Frame::_new_kittest`, and `Context::run_ui`. A meaningful headless actual-frame test can run the existing App with `demo=true` and inspect egui shape/widget rectangles. No new dependency or public rendering helper is necessary merely to get a headless frame. In the probe, texture deltas must explicitly be cleared before dropping a headless frame (`output.textures_delta.clear()`), and the App's owned demo worker is joined by Drop. Avoid an asynchronous spin with no completion bound in permanent tests; use a deterministic test fixture or an explicit bounded demo settle.

Useful regressions: loaded normal/swap default viewport shows all 66 keys in the clip area; minimum viewport can scroll to #66 and Apply with no horizontal clipping; pop-up closing input cannot select an underlying key; confirmation blocks board/toolbar mutation; active calibration has a reachable cancel action and does not accept Enter/Space.

Avoid tests asserting exact fonts/colors/padding constants or reimplementing row_geometry. These mirror styling rather than verifying usability. Native visual verification remains necessary for Japanese glyphs, visual hierarchy, physical app resizing, disabled states, and long labels. Headless rectangles are supporting evidence, not native screenshots.

## Evidence

- Current actual-frame probe source: /private/tmp/niz-ui-reference-20261011/frame-probe.rs
- Successful probe output: /private/tmp/niz-ui-reference-20261011/frame-probe.log
- GUI baseline test output: /private/tmp/niz-ui-reference-20261011/plan-review-gui-tests.log
- Initial probe harness mishandled unapplied texture deltas, causing a headless destructor panic. The harness was corrected to explicitly clear them; this was not a project UI/runtime failure.

No proprietary images/assets copied, no workers delegated, and no real device reads or writes performed by this reviewer.
