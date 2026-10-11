# Focused source-provenance rereview

Host/workspace: local host, `/Users/devonly/Developer/personal/NIZ`.
Reviewed supplied `gui-provenance-review.diff`, actual controller/app source, four new rendered source-invalidation regressions and the fresh-generation regression. No code edits, Git operations, HID I/O, app launches, subagents or test reruns were performed.

## Verdict

**The P2 finding from source-review.md is resolved. No new material regression finding was identified in this focused fix.** The earlier device and GUI execution/ownership verdicts remain unchanged. The coordinator still owns the final independent full-workspace test and native verification.

- `crates/niz-gui/src/controller.rs:168-178` clears calibration together with the raw observation when the user explicitly switches the selected path. The app's after-value fallback can no longer publish device A's value under device B after this source boundary.
- `controller.rs:182-203` clears calibration if the selected device disappears or the exact attached Target is no longer enumerated. Exact Target equality includes path, identity and product, so same-path product or identity replacement also invalidates the result.
- `controller.rs:311-321` clears calibration before advancing the Read generation and dispatching the new request, preventing the previous result from appearing while the new source is still pending or if the new Read fails.
- Switch, list and Read entry points call `available()` before these mutations. Pending operations and active owned calibration therefore cannot enter them through the normal controller API. Devices responses are accepted only for the matching pending operation/ticket/generation. The added clearing does not interrupt a held read, discard active cleanup state, reopen the handle or add a hardware command.
- Completed/failure observations remain retained while the same source is selected and before an explicit new Read/source invalidation. On source invalidation, historical calibration is deliberately removed from the current result UI; this is a bounded result-lifetime choice rather than cross-device attribution. Error objects still carry their captured failure evidence.

## Regression evidence

`crates/niz-gui/src/app_tests.rs:2428-2470` obtains a real completed result through the production App/controller/DEMO worker path, then drives second-device selection or exact enumeration replacement/removal. Assertions check rendered unavailable threshold, disappearance of the old selected literal pair and neighbor count, absent calibration, and invalid live baseline. These regressions check the actual UI consequence of the original P2, not merely an implementation-specific field assignment.

`app_tests.rs:2473-2486` checks that an explicit Read clears the result before generation advance completes and before the new worker response, with the old literal pair absent from the rendered pending frame.

The inspected `gui-provenance-red.log` shows the four source-invalidation regressions failing with the stale observation before the fix. `gui-provenance-green.log` shows all four passing after the fix. The fresh-generation regression was source-reviewed here; its execution belongs to the coordinator's final suite.

All of this remains deterministic simulated evidence. The fix and this rereview establish source attribution behavior, not physical calibration response, flash persistence or measured hardware origin.
