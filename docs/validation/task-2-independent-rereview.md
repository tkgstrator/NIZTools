# Task 2 scoped independent rereview

2026-10-10, native host `devonlys-MacBook-Pro.local`, workspace `/Users/devonly/Developer/personal/NIZ`. Independent Codex fallback reviewer. Read task-2-review-fixes-report.md and inspected current source for all three original P2 findings. No production source edits, Git operations, hardware I/O, Simulator use, or qualification expansion. Temporary offline evidence is retained in task-2-review-offline.

## Result

All three original P2 findings are resolved in the reviewed source. No new actionable finding was identified within the requested scope.

1. **Complete descriptor framing:** `crates/niz-device/src/transport.rs:197-234` now counts all Input/Output fields, independently of collection membership, rejects totals exceeding512bits, and requires complete and selected configuration widths each to equal512bits. Numbered reports reject before unnumbered framing can be assumed. Descriptor length, global/collection stack depth, arithmetic, zero widths, outside-collection fields, and balance are bounded or rejected. An isolated probe of the current function accepts the valid64-byte fixture and the saved native descriptor, and rejects the original520-bit counterexample. Evidence: descriptor-rereview-probe.rs and descriptor-rereview-evidence.txt.

2. **Concrete semantic and raw diffs:** `crates/niz-cli/src/main.rs:164-208` now displays kind, named keys, interval, cycles, custom_delay and delays, with exact report/byte hex deltas. Equal decoded definitions explicitly label an opaque-byte-only change. The same renderer is used by diff and the apply/restore previews. Rerunning the original independent raw repeat fixtures now displays interval100→200 and byte6:64→c8. Evidence: cli-diff-rereview-evidence.txt. Regression tests also verify macro timing/cycles/custom delays and an exact opaque-byte-only delta.

3. **Actual readback diagnostic bytes:** `crates/niz-device/src/session.rs:173-196` returns the original received settings report sequence together with the parsed profile; session.rs422-430 attaches that sequence on a byte mismatch. The transaction error wrapper retains it and adds began_write and backup_path without retry/rollback. Tests assert exact reports for opaque corruption, preserve intentionally noncanonical arrival order, and save the exact hex payload and write/backup context through the CLI capture helper.

## Verification performed

`cargo test -p niz-device -p niz-cli` passed:41device tests,8CLI offline integration tests and1CLI diagnostic-capture unit test; doc tests passed. These use fake transport, temporary files/advisory locks and offline command invocations only. No real hardware was opened by this reviewer.

The original isolated descriptor counterexample was rerun against the new function and returns `Err("complete unnumbered HID report exceeds 64 bytes")`; valid64 and saved native descriptor both return Ok. The original offline repeat diff reproduction now shows both timing values and the exact raw difference.

Ordinary qualification remains empty in session.rs39. No write qualification or calibration/key-swap functionality was added as part of these fixes. This scoped rereview does not claim hardware write qualification or preservation of independent lighting state on XRGB hardware.
