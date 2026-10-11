# Input-test tab, centered rows, and default no-scroll: bounded independent plan review

Verified native host devonlys-MacBook-Pro.local and /Users/devonly/Developer/personal/NIZ. Read-only review of existing GUI, key-code definitions and pinned egui/eframe 0.36.2 native APIs. No source edits, Computer Use, HID, OS permissions, Git mutation, or Simulator activity.

## Conclusion

The GUI-only extension is feasible. Keep the accepted card/keyboard geometry, add a third input-test view, render a read-only board and bounded local received-text display, and center actual row controls while fitting collapsed default content inside 960×820. Preserve vertical scrolling at 700×580 and for expanded details. No window-height increase, global event tap, HID read loop, extra permission, new dependency, or protocol capability is needed.

## Material plan corrections and acceptance constraints

### P2: This egui version supports side-specific physical modifiers

The pinned Key enum includes ShiftLeft/Right, ControlLeft/Right, AltLeft/Right and SuperLeft/Right (key.rs:199-234), and egui-winit maps native KeyCode sides (lib.rs:1644-1651). Therefore the blanket assumption that modifier sides are always unknown is incorrect.

Preserve the side when an actual side-specific physical Event::Key is received. For an aggregate ModifiersChanged-only observation, use an explicitly ambiguous both-side fallback; do not turn a precise left/right observation into both. Test modifier-only input, both sides held together, releasing only one side, aggregate false, and focus loss. Fn and Caps Lock are absent from the pinned Key enum and must not be fabricated. No OS-source attribution should claim that the selected NIZ is necessarily the keyboard that emitted an event.

### P2: Consume test input before egui's focus navigation, not inside App::ui

egui processes Tab/arrow focus direction in Memory::Focus::begin_pass before App UI (memory/mod.rs:570-610). Removing those events later from InputState.events does not undo the earlier navigation. `consume_key` also only removes pressed events, so it is not a complete press/release collector.

Use the parent-verified `eframe::App::raw_input_hook(ctx, &mut RawInput)`, called by the native integration before egui.run. Collect the permitted in-focus input and remove keyboard/text/clipboard/IME activation events there before the framework sees them. Headless Harness.render must explicitly invoke the hook once before Context.run_logic/run_ui, reproducing native sequencing; otherwise its safety tests can pass on the wrong path. Preserve pointer events so users can switch tabs/appearance, read, or clear the tester with the mouse.

Native egui-winit detects Cmd+C/X/V before emitting a Key press and can instead emit Copy/Cut/Paste (lib.rs:1019-1055). It is impossible to reconstruct a physical C/X/V press or side from those clipboard events alone. Ignore/consume clipboard payloads for this physical-input tester; do not append pasted clipboard text as typing or infer a key press from it. State briefly that OS shortcuts/IME and unsupported keys may not produce detectable key events. System-wide shortcuts and window close remain controlled by native OS integration.

### P2: Match against the displayed baseline and never infer a live Fn layer

The assignment board shows an unapplied draft, so reusing that draft for input-test matching would incorrectly move the highlight before hardware application. Use Editor.baseline for the reference board/output mapping and identify its source as the current reference/last-read snapshot, including detached/offline state. Selected_path alone does not prove the source of either the snapshot or the input event.

Match supported single basic definitions only; preserve unknown, advanced, unassigned and special data. Multiple slots with the same output legitimately produce multiple candidate highlights. A logical-layout translation is different from a device remap: physical_key identifies the OS-reported output position before layout translation, not the physical NIZ scan slot. Prefer it for shifted punctuation, with logical fallback when absent, and label highlights as output matches/candidates rather than exact hardware sensor activity.

Do not infer which Fn layer is active from an unreported Fn key or automatically use the editor's last selected layer as hardware state. If the tester permits choosing a reference layer, maintain its own explicitly labeled reference-layer selection rather than changing Editor.selection; otherwise specify that the normal reference layer is used. Include a remapped A→S baseline and an unapplied A→S draft in tests to prove the distinction.

## State and input requirements

- Keep held-state identity stable between press and release despite logical punctuation/Shift changes. Ignore repeats as additional held-key acquisitions; a repeated key must not require multiple releases. Match all duplicate outputs without manufacturing independent physical identities that the backend cannot distinguish.
- Clear highlights on release, raw focused=false or WindowFocused(false), test-tab exit, confirmation, pending/calibration ownership, and fresh-read/profile-baseline change. On re-entry/focus regain, require new events rather than resurrecting stale keys_down state. For modifiers, clear aggregate and side state together when ownership/focus is lost.
- The transcript is local, read-only, bounded to at most256 Unicode scalar values without slicing a UTF-8 byte boundary, and excluded from files/backups/exports. Use fixed-height wrapping/scrolling so a long string or many newlines cannot create default main-window overflow. Do not record input in other tabs or in the background.
- Capture modifies only input-test presentation state. It must not touch raw records, selected output readiness, Undo/Redo, generation, attachment, consent, requests, trial confirmation, or calibration phase. Busy/modal/calibration frames clear and suspend capture.
- The read-only board must not route clicks through KeyboardState::click or alter the editor. Share immutable geometry and label helpers where useful; avoid disabling the existing editor board as a shortcut if that unnecessarily washes out all tester labels/highlights.
- Add a theme role for pressed matches if needed; test its normal label contrast and meaningful outline, and keep pressed indication distinct from From/To and dirty-draft markers.

## Alignment root cause and bounded remedy

Current rows rely on egui horizontal Center alignment, but a later taller widget can grow the row after an earlier label has already been positioned. The initial row height derives partly from spacing.interact_size.y, while buttons/ComboBoxes also include font height plus vertical padding. This can make otherwise Center-aligned controls have different actual center Y values. Header heading vs ComboBox and body/status text vs buttons need a known row extent before placement.

Use an explicit shared row extent based on the actual included font/widget heights and padding, or allocate/align response rectangles within a known row. A common interact height can help initialize wrapped rows, but blanket minimum-size shrinking is not a reliable fix. Heading rows may need a different known maximum than ordinary body rows. Keep wrapped controls on their own centered row at minimum width and verify the actual centers rather than merely checking that Layout says Center.

Compare button/ComboBox response rectangles and label galley rectangles within the same row at <=1 logical point. Test device/header/status/mode/footer rows, both themes, and wrapped minimum-width rows. Do not compare text baselines or glyph mesh edge centers across fonts; use allocated galley/widget rectangles and corresponding actual visual context.

## Zero default scroll requires content evidence

Previous tests proved keys and primary actions were visible, not that all collapsed main content fit; later file/operation disclosures can produce a small scrollbar. Keep the outer ScrollArea for smaller windows and expanded details, but measure its actual content_size and inner_rect (or observe unchanged scroll positions after a wheel attempt) at 960×820. Require zero usable vertical overflow and offset for clean, one-key dirty, swap dirty, and input-test frames in both themes. Include collapsed file/operation disclosures and ordinary status messages, not only the board/footer.

Prefer small reductions of repeated explanation/vertical spacing or bounded transcript/status panels. Do not hide actionable failure/cancel/consent, disable scrolling over real overflow, or increase default/minimum viewport heights. Expanded technical detail, long errors, and the minimum viewport may scroll normally. An inline test-only geometry report from the actual ScrollAreaOutput is appropriate if it avoids duplicating the layout.

## Proportionate verification

Actual native-sequenced input-hook tests should cover A press/release/repeat, Shift+punctuation physical fallback, aggregate and exact-side modifiers, duplicates, unknown/unassigned/advanced records, Fn/Caps absence, release/focus loss/tab exit/profile reload, and UTF-8 transcript bounds. While the test view is active, Enter/Space/Tab/arrows must not move focus, edit, invoke read/apply, or start/advance calibration; no pending request/generation/history changes should occur. Include Copy/Cut/Paste/IME payload handling and prevent clipboard/IME data from being mislabeled as physical key activity.

Retain the existing popup-closing/modal/busy/appearance/contrast tests, extend default no-scroll and row-center assertions to both themes, and retain min-width all66 reachability. Parent native QA should verify centered rows, no slight default scrollbar, visible pressed/released matching, focus-loss cleanup, input-test mouse exit, and light/dark readability. No physical write or calibration sequence is necessary for this GUI-only verification.

No material plan blocker remains once these API corrections and acceptance checks are incorporated. The InputTest feature is an OS input display and reference-output matcher, not a selected-device scan or analog sensor measurement.
