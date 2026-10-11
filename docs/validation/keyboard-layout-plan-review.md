# Clickable Atom66 layout: independent plan review

Reviewed natively on `devonlys-MacBook-Pro.local`, workspace `/Users/devonly/Developer/personal/NIZ`, clean HEAD `bcc272fbe1706914666a1714838a78148d21ca01`. Read-only review of current `app.rs`, `editor.rs`, `controller.rs`, GUI tests, core layout/constants and profile validation. No Git mutation, HID operation, GUI operation, source edit or other review result was used.

The bounded implementation is feasible without changing the hardware protocol, worker requests or core profile format. Existing `Selection` already carries the physical slot, layer and mode; a keyboard component can update it without invoking `Editor::edit` or `Controller::trial`. Existing byte-exact trial approval, target binding and worker regeneration remain useful independent defenses.

## Findings to resolve before implementation

### P1 — Establish physical slot correspondence before calling this the actual keyboard layout

References: `crates/niz-gui/src/app.rs:200`; `crates/niz-core/src/keys.rs:259`; `docs/superpowers/plans/2026-10-10-niz-macos-progress.md:46`.

The current UI explicitly states that physical legends are unverified. Core `KEY_ROWS` is a label table, and the prior progress ledger records mismatches between these labels and captured assignments. A captured output alone does not prove the physical legend: the keyboard can already be remapped. A visually plausible board with wrong slot IDs would let a user approve a perfectly valid write to an unintended physical key.

The separate primary layout investigation is therefore an implementation prerequisite for immutable row geometry, legends and slot IDs. Keep core constants unchanged unless primary proof justifies an unrelated core change. If the evidence proves only some positions, clearly mark the board as unverified and retain numbered-slot selection rather than asserting an actual physical layout. Add fixture-based semantic correspondence assertions for independently established positions, including A and the two Fn keys; uniqueness/count tests alone cannot catch a shuffled slot map.

### P2 — Define selected-key settings initialization rather than retaining an arbitrary previous To code

References: `crates/niz-gui/src/editor.rs:16`; `crates/niz-gui/src/app.rs:182`; `crates/niz-gui/src/app.rs:201`.

`Selection::default()` uses output code 15, and the editor immediately regenerates a preview for that output. If clicking A merely changes `selection.from`, the first click displays an A-to-7 change rather than loading A's existing setting. This is confusing for the requested interaction: click a key, then configure that key.

Recommended semantics: a different From slot or layer navigation initializes To from the selected draft record when it is one supported ordinary single action; selecting a swap operand only changes the corresponding physical slot. This navigation must leave records, baseline, dirty status and undo/redo history unchanged. For an advanced, unassigned or unknown record, show the existing preserved action and require an explicit supported replacement rather than inventing a current value. Apply the same initialization helper to the fallback From dropdown so both selectors behave consistently. Clicking the same selected slot should not unexpectedly overwrite an output the user is choosing.

### P2 — Specify swap click-target transitions and reset behavior

References: `crates/niz-gui/src/editor.rs:8`; `crates/niz-gui/src/app.rs:172`; `crates/niz-gui/src/app.rs:194`.

The plan proposes From/To click targeting plus an optional automatic transition after From selection. Give this one deterministic rule. A small GUI-only `ClickTarget` is sufficient; do not extend serialized profiles or worker protocol. Recommended: entering swap starts with From, clicking From advances to To, explicit targeting remains visible, and leaving swap resets targeting to From. A layer change preserves both physical slot IDs and maps them through the new layer. Define same-slot selection as a no-op and ensure its visuals do not hide that the two operands coincide. Avoid automatically swapping records on the second click: the existing explicit add-change action is the correct mutation boundary.

## Integration and testing guidance

- Keep the board inside the assignment view, below its From/To editing controls and above lengthy change logs. Keep calibration independent. Disable all board interaction while `Controller::blocked()` or a confirmation is open; retain `approve_trial`'s selection and generation checks. Current confirmation is a regular egui window (`app.rs:432`), so the board must receive the confirmation condition explicitly.
- Dirty markers must compare raw records for the active layer against the baseline, not compare display labels. Advanced definitions, padding and macro tails can differ while displaying the same label (`editor.rs:133`). Fn-linked changes span layers and should become visible as the layer changes.
- Rows need consistent width accounting with spacing and variable key widths. The current layout constants total 15 units per row, but the row key counts differ. Use widths that include proportional internal spacing so shorter rows do not visually end earlier. Long actions such as `Advanced assignment (preserved)` cannot be rendered verbatim inside a roughly 35–40 px key at the 700 px minimum window size. Use compact visible labels and retain complete details in a tooltip/selected-key panel. Full row geometry and long-label handling require parent screenshot verification, not only arithmetic tests.
- A five-row board adds roughly 250–300 px. The existing viewport is 860×740 with minimum 700×580 (`main.rs:6`); merely changing height to 820 does not prove fit on a short display. Preserve the outer vertical scroll area, keep the board itself free of horizontal overflow, and verify default, minimum and short-display dimensions. Avoid nested keyboard scrolling.
- Meaningful tests: all 66 immutable slot IDs unique and present; evidence-backed physical-slot correspondence; layer × slot addresses; one-way clicks only affect From and never records/history/HID requests; explicit/automatic swap target transitions; mode/layer reset semantics; blocked/confirmation clicks are ignored; same-slot no-op; dirty markers follow raw active-layer records and Undo; unknown/advanced actions remain preserved during selection; selecting a slot after a pending edit does not bypass exact trial regeneration or approval pinning.

## Assessment

Proceed once the physical map evidence and navigation initialization/targeting rules are resolved. No core/device changes or broader UI architecture are needed for this request. Retain existing guarded hardware application and calibration behavior. This review does not claim a visual check, implementation check or hardware verification.

## Scoped proposal verdict

The revised GUI-only proposal can proceed while the lighting/configuration slot bridge is investigated. It limits the board to vendor-image row geometry (15/14/13/13/11), current draft assignments and fixed logical slot IDs. It does not claim immutable Fn/Alt legends or a fully proven physical correspondence. Clicking a board key is therefore another view of the existing numbered-slot selector, with no raw-record change or HID output. The implemented assignment and hardware guards remain unchanged.

This is an explicit reduction of the initial “actual physical layout” claim; it does not establish the physical slot mapping requested in the original P1. The board should say `キーボード · 現在の割り当て`, visibly retain fixed position IDs and distinguish draft labels from physical legends. Preserve the existing factual caveat that correspondence between position IDs and physical legends is unverified (`app.rs:200`), for example `現在の割り当てを表示します。位置番号と刻印の対応は未確認です。` Merely saying `刻印と異なることがあります` can be read as a normal remapping warning and does not communicate this uncertainty.

With that wording, the uncertainty is disclosed and the P1 is resolved for the narrowed logical-slot component. The newly reported vendor lighting-view rectangles/IDs are useful evidence, but must not be silently promoted to proof of the configuration-slot bridge. The proposed To initialization, deterministic swap selection and busy/confirmation protection address the P2 findings. A sanitized actual fixture may make the GUI demo representative, provided it is identified as a simulation and does not change core's independent demo fixtures or hardware qualification.
