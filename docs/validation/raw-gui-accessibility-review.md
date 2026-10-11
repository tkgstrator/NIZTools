# Focused independent review: accessible raw-read activation

## Plan review

**Plan accepted: replace `clicked_by(Primary)` with ordinary `clicked()` only for the ReadRawThresholds button.** This is a small, useful accessibility correction. Pointer-only execution was an earlier plan choice, not a user requirement. The explicit manual action can be mouse/touch, focused Enter/Space, or a targeted accessibility click. It must remain distinct from automatic reads caused by key selection, theme changes, InputTest, timers, or keyboard events directed elsewhere.

This is an authorized Codex fallback review; LocalGPT and Claude Code MCP did not perform it. Native file access was verified for `/Users/devonly/Developer/personal/NIZ` on `devonlys-MacBook-Pro.local`. No repository source edits, Git, HID, or native UI actions were performed. Native coordinate/AX capability statements come from the coordinator, not verification by this reviewer. Final implementation review was pending at plan-review time; the frozen actual-source result follows below.

### Feasibility and constraints

- Pinned egui 0.36.2 `response.rs:174-198` explicitly documents that `clicked()` recognizes accessibility and focused Enter/Space, while `clicked_by()` excludes both. `context.rs:1467-1480` creates the synthetic primary click only for an enabled click-sensing widget; keyboard activation additionally requires that widget's focus. Thus a one-line change is the native framework path, not a global key handler or an accessibility bypass.
- Current raw button guard (`app.rs:371-378`) requires the exact live source through `can_read_raw_thresholds`, no modal, no current popup, and no popup at frame start. `Controller::can_read_raw_thresholds`/`read_raw_thresholds` (`controller.rs:219-253`) also reject pending, calibration, close, wrong/missing source, unsupported identity, and firmware. Preserve these guards unchanged. Targeted accessibility clicks must still fail when the widget is disabled; focused keyboard activation must not bypass them.
- InputTest consumes keyboard/text activation events before egui focus handling (`app.rs:702-738`) and the raw button is not rendered in that view. Keep this code unchanged. There is no need to broaden event capture or add a shortcut.
- Scope is the raw-read button alone. Do not change Apply confirmation, trial target/baseline checks, calibration activation/stage checks, cancellation, or device-level behavior. Existing Apply uses its existing modal/controller safety path; this plan does not assert that it is pointer-only. Do not revive Begin/Held/Finish controls or add a threshold setter.
- The getter's runtime-effect explanation, snapshot/non-mm meaning, exact target pin, cached physical-key mapping, and unsupported setter copy remain applicable. Keyboard activation does not turn this table read into a held-key sample or a physical-depth measurement.

### Material testing adjustment

**P2 if omitted: make the old “Enter does not refresh” test explicitly unfocused.** `app_tests.rs:1365-1376` currently sends Enter after clicking the read button without asserting/surrendering its focus. Once focused Enter is legitimate, a blanket no-read assertion is the wrong acceptance criterion. Capture/verify the actual raw button ID and explicitly remove its focus before testing Enter/Space events directed elsewhere. Preserve the ticket sentinel for key selection/theme/InputTest navigation. Do not weaken that regression to merely checking displayed text.

Add bounded positive tests for exactly one read from focused Enter and Space, and one targeted AccessKit Click action for the actual read-button node. Verify the actual pending command/operation ticket or completed read timestamp rather than only a fake click flag. A rendered AccessKit event test is preferable to a generic response-helper test because it proves the application's guarded handler receives the action. If native accessibility testing is unavailable, state that limitation rather than claiming it occurred.

For focused keyboard and accessibility actions, include disabled exact-source, modal/popup, pending, close, and active-calibration refusal as appropriate, using the existing GUI/controller gates. Repeated events during pending must not overlap operations. Keep unfocused Enter/Space, repeated physical-key selection, appearance, and InputTest actions from dispatching raw reads. Preserve geometry/tooltip/label content and editor/source state. No new hardware test or full parser/Device retest is required for this UI activation-only change.

No material blocker remained to implementation with this bounded scope.

## Final actual-source review

**Result: clean; no unresolved material findings in the accessibility delta.** The implemented raw-read handler uses ordinary `.clicked()` at `app.rs:376` with its existing enable/source/modal/popup/close/calibration gates intact. Mouse activation, deliberate focused Enter/Space, and targeted accessibility activation are now the explicit supported action. Pointer-only execution was the previous design choice; it was not a user requirement. No global keyboard shortcut, automatic refresh, extra device request, or destructive-action change was added.

This reviewer verified the exact bounded production difference without Git or repository edits: replacing the current raw button's single `.clicked()` line with its previous `.clicked_by(Primary)` text **in memory only** reproduces the previous reviewed `app.rs` SHA-256. Comparison against the previous independent GUI hash record shows only `app.rs`, `app_tests.rs`, and `target/debug/niz-gui` changed. All other 15 GUI Rust source/test files are identical. Thus existing Apply/calibration handling, Controller/Worker ownership, Device-facing methods, InputTest capture, theme/physical key geometry, and raw snapshot lifecycle were not modified by this follow-up.

The new tests exercise the actual application control rather than a generic response helper:

- `app_tests.rs:1365-1384` captures the actual clicked read-button ID through `Context::interaction_snapshot`, requests its focus, and demonstrates a fresh raw request/completed snapshot from Enter and Space. Editor state remains intact.
- `1388-1420` forces focus and verifies both Enter and Space fail to start a reader when the source is offline, another operation is pending, close is requested, or a modal/popup is open. Pending ownership remains unchanged.
- `1424-1483` enables AccessKit, finds the actual Button node labeled `しきい値を読み込む` from the generated tree, and sends an ActionRequest Click to that node. Enabled activation starts the reader and produces the simulated snapshot; offline/pending/close/modal/popup activation is refused. This is a headless framework accessibility-action test, not a native macOS AX/Computer Use test.
- `1509-1541` explicitly surrenders any widget focus before its negative Enter case, then retains the existing timestamp and actual operation-ticket sentinel through key selection, appearance, and InputTest navigation. The previous P2 testing concern is resolved: intentional focused activation is separate from unfocused Enter/non-reader navigation.

Independently executed after owner freeze:

`cargo test -p niz-gui --lib raw_threshold_`

**Exit 0; 9 passed; 0 failed; 0 ignored; 41 filtered out; 1.62 seconds.** Log: `/private/tmp/niz-raw-20261011/gui-accessibility-independent-tests.log`. This focused run includes the new keyboard/AccessKit cases and the existing manual/demo, source guard, pending/error, non-refresh, hover, and default/minimum snapshot-fit checks. The unchanged full GUI/workspace portions were not rerun by this reviewer; the coordinator/implementer's fresh broad runs remain separate evidence.

Before and after the independent tests, **18/18 entries match** `/private/tmp/niz-raw-20261011/gui-accessibility-source-debug-sha256.json`: all 17 GUI Rust source/test files plus the debug binary. Current debug SHA-256 is `76503900fafffa6b5d565c018fa7f14a3d0843f14ec78b448aba20ffd7d32f3e`. Independent evidence record: `/private/tmp/niz-raw-20261011/gui-accessibility-independent-sha256.json`, including the one-line production-difference proof and pre/post test checks.

No additional GUI fix is requested. Native accessibility/visual verification belongs to Codex coordinator. This follow-up does not establish actual HID behavior, millimeter/continuous sensor semantics, persistence, or a per-key setter; those existing limitations remain unchanged.
