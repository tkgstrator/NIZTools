# Independent D8 ownership fix rereview

Read-only review of frozen device-rereview.diff and matching actual sensitivity/session/error/tests sources; private qualifier unchanged and reviewed again structurally. No HID, app, firmware, or source mutation performed.

P1 is resolved. sensitivity_error now checks attempted D8 and Error.passive_settle_complete before making status terminal, then retains the same owned transport/lock through bounded two-second passive input capture with no output. Late reports remain in active sensitivity trace and error.raw_reports. Fresh post-F9 write/short-read/timeout failures are covered, as are D8 short/error deliveries. An already settled DC failure sets the private error marker and avoids a duplicate wait. Existing failed post-DC tests assert exactly 2000ms requested waits, covering no double hold. Status carries original error, backup, complete before and any valid after; failure still poisons/invalidate baseline and never retries or rolls back.

No remaining material source finding for offline structural hardware-qualification clearance. The qualifier still uses the explicit confirmation flag, exact single identity target, same owned session, coefficients20 then24 only after first Complete/settings/RGB verification and a new baseline, stops on any error, writes actual trace/vector result evidence, and prints measured final_R_equals_initial_R without a restoration claim. Hardware execution remains conditional on parent verification of required offline tests and rebuilding the private qualifier against this fixed source. This reviewer did not independently rerun the full test suite or execute hardware.

## Final bounded integration rereview

P2 resolved in actual sensitivity_ui.rs: apply now has a released-key / temporary-input-pause hover instruction, preserves the primary-pointer predicate and popup/modal/live gates, and a focused hover test checks instruction without dispatch or control movement. Apply label uses explicit white on its red background; parent-reported light/dark contrast regression test is appropriate. Condensing the nested error branch into the same primary-click predicate does not change operation activation.

Error::new is now public to replace external CLI struct literals after the private passive-settle marker was introduced. CLI err helper and test fixture initialize through that constructor; the marker remains private and defaults false. This does not bypass the owned-settling logic or alter the device transaction.

No remaining material source/integration findings. Code is structurally clear for the explicit qualification after parent verifies final offline workspace checks and rebuilds the private qualifier against current fixed source. No HID/app action or full-suite rerun was performed by this reviewer.

## Private observed-R restore helper review

Read-only inspection of probe/src/bin/restore_observed_r.rs after reported successful coefficients20/24 qualification. No hardware execution by reviewer. Structurally clear: explicit flag, complete66u8 original/20/24 evidence, coefficient candidate inversion requiring only28 compatible among20/24/28, original exact path and descriptor identity, new owned native baseline and current R equal24.after before the one guarded coefficient28 adjustment. Complete/settings/RGB and final all66 equality to initial are required before a restoration claim; failure stops without retry or reset/calibration/mode flash.

The inversion establishes a uniquely compatible coefficient within the tested stored-mode set, not an independently measured original factor or unique T values; integer rounding can permit multiple T values. Actual final all66 equality is decisive. Checking result.before==r24 additionally would record/detect a last-minute RAM change during the API preflight; no material blocker found for the single explicit adjustment and fail-closed final claim.

## Final concise Japanese raw-read footer review

Read-only bounded review of current controller.rs raw read pending/result strings against the previously reviewed transaction/response context. The three changes only replace message text: `しきい値を読み込み中`, `しきい値を読み込みました`, and `DEMO · しきい値を表示しました`. Read dispatch and exact-source validation, complete snapshot storage, source distinction, generation/baseline ownership, and failure branches are unchanged. No material findings or semantic regression found. No product edit, app/HID/Git operation, or test rerun performed by reviewer.
