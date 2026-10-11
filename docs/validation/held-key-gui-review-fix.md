# Task 2 independent-review P2 provenance fix

Host/workspace: local Mac `/Users/devonly/Developer/personal/NIZ`.
Finding: the complete calibration post-observation fallback and target/other-change summary could remain visible after switching the selected concrete device, or replacing/removing the captured descriptor. This presented calibration A as a current value under device B.

## Fix

The controller clears its calibration result when the user selects another device, the selected device disappears from enumeration, or enumeration no longer contains the exact attached `Target` (path/product/identity). It also clears the result before an explicit Read advances the connection generation. Thus the selected threshold falls back to `未取得` and the old comparison summary/table disappears at source invalidation. Complete observations remain available on same-source terminal failures until the source changes. Existing physical-key selection continues to retain the result, and the compact layout is unchanged.

Production semantic changes after the initial frozen implementation are only four `self.calibration = None` assignments in `controller.rs` at device-switch, selected-device-removal, exact-attached-descriptor-invalidation, and read-start boundaries. GUI rendered tests were added in `app_tests.rs`.

## RED → GREEN

Before the fix, `cargo test -p niz-gui --lib held_result_is_invalidated -- --nocapture` failed all four rendered regressions on the same expected stale-value assertion:

- switching to a second enumerated device;
- same-path product descriptor replacement;
- same-path identity replacement;
- complete device removal.

See `gui-provenance-red.log` (0 passed, 4 failed). After the controller invalidation fix, the same command passed all four tests (`gui-provenance-green.log`). Each asserts unavailable selected threshold, absent old target/other-count result, cleared status and non-live baseline.

The added read-generation test verifies result clearing happens before the new generation can expose an old post-value. `cargo test -p niz-gui --lib held_` passed all 15 filtered tests including all five new source-invalidation tests, actual keyboard/AX guards, pending/modal/popup/close guards, 66 literal pairs, automatic completion, complete observations after settings failure, no-change/partial outcomes, and cancellation. See `gui-review-scoped-green.log`.

Production files are frozen again for root review/rebuild. Whole-workspace verification and native visual checks remain coordinator-owned. No HID I/O, native app launch, Git operation, device-file edit or further delegation was performed for this fix.
