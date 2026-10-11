# Independent source review before hardware qualification

Read-only review of frozen `/private/tmp/niz-global-sensitivity-20261011/device-review.diff`, relevant device files, and private probe source. No hardware/app/firmware action and no source edit performed.

## Material finding

[P1] Retain bounded passive ownership after every failed attempted D8. In `crates/niz-device/src/sensitivity.rs:202`, a short/error D8 send exits immediately. At line 207, post-read fresh F9 failure also exits before DC dispatch. The DC reader's passive settling only occurs when its own `dc_attempted` is true (`crates/niz-device/src/session.rs:310`), while `sensitivity_error` at sensitivity.rs:122-129 merely marks terminal and poisons the session. A caller/worker can consequently release its owned transport/lock immediately after uncertain D8 delivery, permitting another session's commands while effects are uncertain. Keep bounded passive capture/settling before terminalizing any post-D8 failure, without sending retry/recovery commands; preserve captured reports/status trace. Add D8 short/error and post-F9 error/short tests asserting owned wait/capture and no extra output. Existing uncertain_d8_delivery test checks no retry/evidence but currently does not assert any passive wait.

## Other checked behavior

- Exact firmware/identity/live-baseline/range gates, fresh config equality, durable verified settings/RGB snapshot precede D8.
- Complete DC1 observations are mapped through existing qualified RawThresholds parser; no fake user-pasted vectors.
- Native D8 framing/argument index is consistent with recovered dispatcher, single send and no D8 ack read.
- Live baseline invalidated before write attempt; failures preserve before/backup/delivery uncertainty, no rollback promise.
- Cancellation before send blocks mutation; cancellation after successful D8 runs the owned bounded post-DC observation, then aborts without issuing calibration/reset/rollback commands.
- DC1 does not regenerate R: getter 0x670c uses 0x506c scan/output peripheral reset, never 0x7624; it restores scan enable and configured compensation only. The recovered R coefficient effect survives DC1 table read.
- Current tests cover range, foreign/stale identity, backup/preflight/queue failure, malformed/late/excess DC, config/RGB drift, cancel, and active-session ownership. No full test rerun was performed by this independent reviewer.

## Private qualifier

`/private/tmp/niz-global-sensitivity-20261011/probe/src/main.rs` uses one exact identity target and one owned Session; requires an explicit flag; first coefficient20 must return Complete plus settings/RGB verification before saving evidence and performing an explicit fresh baseline and coefficient24 transaction. Every failure stops; second operation is not an automatic retry. Exact firmware validation happens inside adjust_sensitivity before D8. Complete vectors come from native results. Probe prints actual final_R_equals_initial_R and does not claim restoration without equality. No further blocking issue found in the qualifier, conditional on resolving the ownership finding above before execution.

## Frozen global GUI independent review

Reviewed `/private/tmp/niz-global-sensitivity-20261011/gui-review.diff`, sensitivity_ui.rs, protocol/controller/worker/app integration. [P2] The sensitivity apply button lacks the planned released-key tooltip/instruction (`crates/niz-gui/src/sensitivity_ui.rs:39-49`). This transaction performs two DC1 reads that reset scanner/output/compensation state, so the user must see a concise instruction to release all keys before applying. Add an apply tooltip such as `すべてのキーを離して適用してください` and a bounded UI assertion.

Other checked guards: primary pointer only, keyboard/AccessKit rejection, popup/modal/busy/active calibration/live/exact source gates, target and ticket ownership, native baseline consumption, close cancellation with pending transaction wait, complete66/coefficient/Complete/settings/RGB success validation, selected descriptor/path source match for top FROM/TO, fixed input layer0, explicit simulated result provenance. No other material GUI findings found. Tests were reviewed rather than fully rerun here.
