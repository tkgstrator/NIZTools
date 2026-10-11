# Compact UI and temporary global sensitivity validation

Validated on 2026-10-11 in `/Users/devonly/Developer/personal/NIZ`, on `devonlys-MacBook-Pro.local` (Apple Silicon, macOS 26.4). The stock USB target was `66EC-XRGB`, `0483:502a`, usage `008c:1`, path `DevSrvsID:4295010726`, firmware `66EC(XRGB)BLe;V1.0.48;V1.0;`. No firmware was flashed.

## Result and UI

The threshold view now displays the selected physical key and **FROM → TO**, followed by its read/prepare/calibration controls and the clickable keyboard. Normal success no longer displays the lengthy threshold explanation or a zero-change neighbor warning. Actual other-key changes and failures remain visible; complete comparisons and transport evidence remain in collapsed diagnostics. Values from another target or incomplete observations cannot populate a successful comparison.

Input Test displays only the keyboard. Key-down lights matching positions and key-up clears them. It no longer keeps text, a held-key list, a Clear control, or layer selectors. It uses the loaded normal-layer mapping and keeps modifier, repeat, focus-loss, blocked-operation, and reference-change handling. Its input capture cannot edit mappings or trigger device operations.

The collapsed **全体の反応（一時）** control adds a slider and red, white-text **適用** button. Left means harder to actuate; right means easier. Its initial position is a desired next setting, not a reading of the device's current coefficient. The released-key/input-pause instruction is shown on hover. Primary-pointer activation, source ownership, fresh-baseline, modal/popup, and operation gates remain enforced. After calibration or adjustment, a fresh settings read is needed for Input Test.

Native CUA inspection of the packaged application confirmed the simplified screen, A `FROM: 11`, the Light palette, slider, red Apply button, all 66 physical keys, and collapsed diagnostics without scrolling at the default window size. Dark Input Test was inspected in the explicitly simulated no-HID demo. The final raw-read footer was shortened to Japanese after this inspection; its nine relevant rendered tests were rerun and the release rebuilt. The native accessibility Click action does not activate the deliberately primary-pointer-only Apply button; native D8 qualification therefore used the actual guarded device API, while rendered event tests verify GUI dispatch. No native GUI write-button execution is claimed.

## Actual hardware observations

Before replacing the old application, the coordinator observed the user's completed official held-A calibration in the native GUI: **A #31, 23 → 11; other keys changed 0 / 65**. That was user-operated calibration, not a synthesized physical press by the coordinator. The later complete D8 baseline independently measured A at 11. Restarting the app does not fabricate the earlier in-memory comparison.

The new `Session::adjust_sensitivity` API was qualified on the exact target with explicit private helper flags. Each distinct operation acquired a fresh baseline on the same owned session, saved/reparsed keymap and lighting snapshots, read all 66 normal-scan R values through DC1, attempted D8 once, read all 66 values again, and verified unchanged keymap and lighting. There is no D8 success acknowledgment; the native output byte count and subsequent observed comparison are separate evidence.

| Step | D8 coefficient | Changed keys versus immediate before | A before → after | After range |
| --- | ---: | ---: | --- | --- |
| Qualification 1 | 20 | 66 / 66 | 11 → 9 | 9–21 |
| Qualification 2 | 24 | 65 / 66 | 9 → 9 | 9–25 |
| Restore observed starting R | 28 | 66 / 66 | 9 → 11 | 11–30 |

Every transaction completed with settings and lighting verified. The coordinator checked the second before vector equals the first after vector, the restore before equals the second after, and **all 66 final R values equal the first starting vector**. Thus the temporary test effects were removed, including A returning to its user-calibrated 11. Coefficient 28 was the only compatible candidate among the firmware's configured mode coefficients 20/24/28 under all 66 observations. This was an inference, not a direct read of the original coefficient. No retry, calibration, reset, mode save, or arbitrary analog rollback was performed.

The sanitized [hardware fixture](fixtures/atom66-xrgb-v1.0.48-global-sensitivity.json) contains all before/after vectors, D8/DC1 frames, verification flags, and hashes of the original captures. It intentionally excludes the private keymap/macro and RGB packet contents. Complete original captures and durable snapshots remain in `/private/tmp/niz-global-sensitivity-20261011/hardware/`. The [qualification](global-sensitivity-logs/hardware-run.log) and [restoration](global-sensitivity-logs/restore-run.log) summaries are retained.

## Protocol scope and limits

Independent static review found stock D8 coefficients 10..40 recompute the 66 RAM R entries from stored T and global mode. For non-sentinel cells the formula is `max(9, floor(T * coefficient / 40))`; sentinel/default cells use the current mode instead. Therefore some keys can remain unchanged due to the minimum, default handling, or rounding. The slider adjusts overall response; it does not impersonate physical ADC input or upload arbitrary per-key numbers.

The reviewed D8 path writes R rather than B/T or a durable coefficient. A startup, mode regeneration, or calibration can replace this temporary setting. Empirical qualification verifies observed R and unchanged keymap/RGB; it does not independently prove B/T preservation, power-cycle behavior, press depth in mm, physical response improvement, or a general restoration API. Arbitrary selected-key sliders, bulk R-table uploads, and factory reset remain outside the implemented stock protocol. See [feasibility review](global-sensitivity-feasibility.md) for addresses and conditions.

Firmware internally samples sensor values, but a supported live sensor/depth retrieval endpoint was not established. DC1 reports R thresholds, not the current press depth. Relative sensor units would also require additional evidence before conversion to mm. See the [live-depth review](live-depth-review.md).

## Tests, independent review, and release

The coordinator ran `cargo test --locked --workspace --all-targets`: **350 passed, zero failed or ignored** (core 53, device 124, CLI 15, GUI 158). The [full log](global-sensitivity-logs/test-complete.log) is retained. A final single-loop lint cleanup changed only one external test body; the affected [nine-test contract suite](global-sensitivity-logs/contract-final.log) was rerun. After the final footer-only change, all [nine raw-threshold rendered tests](global-sensitivity-logs/wording-check.log) passed. Formatting and `cargo clippy --locked --workspace --all-targets -- -D warnings` passed on the final source.

New coverage includes coefficient/range/formula edges, fresh source/baseline checks, complete physical-order comparisons, snapshot failure, missing/extra/malformed replies, stale/replayed worker replies, cancellation before and after D8, primary versus keyboard/secondary/AccessKit activation, close while ownership is active, Light/Dark contrast, and hover geometry. Failed or short D8 delivery and post-D8 F9 failures hold the same lock through two seconds of bounded passive capture without output, retries, or double-settling an already settled DC failure. Text input is ignored while pressed/released/modifier signals still drive keyboard highlights.

Test-first compact-UI and contrast failures were observed before fixes and then passed. Independent review identified a P1 missing owned settle after uncertain D8/post-F9 failure and a P2 missing released-key instruction. Both were fixed, regression-tested, and rereviewed before hardware access. Constructor integration errors and lint findings were corrected before the successful final checks. [Source review](global-sensitivity-source-review.md), [rereview](global-sensitivity-source-rereview.md), and separate device/GUI implementation reports preserve the evidence and review scope.

LocalGPT and Claude Code generation tools were unavailable in the current callable tool inventory. Authorized independent Codex workers provided static feasibility, plan/source, and actual-diff reviews; no LocalGPT or Claude review is claimed. The coordinator verified the integrated code, tests, native UI, hardware, release, and actual Git result.

`scripts/build-macos.sh` built the final GUI and CLI for arm64 and x86_64 and produced Universal 2 artifacts. Ad-hoc signature verification, plist lint, both Mach-O slices, extracted ZIP verification, and native/Rosetta CLI help checks passed. [Release hashes](release-sha256.json) and [source hashes](global-sensitivity-source-sha256.json) bind the evidence to the checked contents. The application is ad-hoc signed, not notarized.

No Simulator resources were used. Owned private qualification sessions exited and dropped their connection locks; the owned no-HID demo was quit and its process absence verified. The packaged actual application was reopened for the user's input test after final build verification. Private diagnostic files were retained, and unrelated applications or processes were not stopped.
