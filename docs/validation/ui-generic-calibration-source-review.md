# Final incremental technical review — generic physical calibration and danger buttons

**Verdict: no remaining material P0–P2 finding in the reviewed frozen GUI change.** The independently reproduced P2 default-viewport calibration overflow is corrected and rechecked below.

Verified host: `devonlys-MacBook-Pro.local`.
Workspace: `/Users/devonly/Developer/personal/NIZ`.
This read-only incremental review follows the earlier final six-file GUI assessment (`ui-final-technical-review.md`) and covers generic physical-key calibration, theme-specific red execution/Start buttons, and the resulting layout fix. No repository source or Git state was edited. No native UI, HID, or Simulator operation was performed.

## Actual implementation and retained behavior

- `crates/niz-gui/src/app.rs:357-438`: the reaction tab provides global trigger-switch guidance and a separate physical calibration step card above the full 66-position official-layout selector. Default A is only the initial guide; S and Fn-bearing positions are selectable. Selection updates local `calibration_position`, without changing the editor draft, pending output, history, generation, selected device, or worker request. The selector is frozen for pending/active/closing operations, confirmation, and popup ownership, including the popup's closing frame.
- `app.rs:395-408`: the run guide is captured only after successful Begin dispatch and used for held-key instructions. Begin, held, and finish still require a primary-pointer click and call the existing controller path. The original consent remains. Exact attached target, generation, supported firmware/identity/groups, and worker stage/cleanup protections remain exercised by the controller and worker tests.
- `app.rs:439-499`: terminal result copy is generic and cannot be relabelled by later idle selection. Calibration is described as the released-all-keys plus physically held-key procedure; the selected ID is not represented as a protocol mask or numeric threshold. Existing irreversible-calibration and unacknowledged-unlock warnings remain. The unsupported numeric capability detail is present inside the existing disclosure. This matches the user's clarification that selecting a physical key for calibration is the desired feature.
- `app.rs:835-850`: the current RightFn position comes from a unique basic single-code 156 assignment in the loaded normal-layer baseline, gated by the existing qualified firmware/identity/group and group-2 slot-8 trigger action. Missing/duplicate/custom/unsupported or nonlive data does not produce a confirmed locator. The captured baseline resolves to physical position #62, official Right Alt legend, so the printed key and firmware function are kept distinct.
- `crates/niz-gui/src/keyboard.rs:384-425`: the new board uses the immutable official legends and existing 66-position row geometry. It does not depend on mutable output assignments and does not gate physical choices by assignment type.
- `app.rs:566-593` and `crates/niz-gui/src/theme.rs:110-114`: the original Apply modal retains target, firmware, concrete diffs, backup path, simulation/experimental explanation, and approval action. Execute and Cancel now share a horizontal row; Execute and calibration Start use red theme surfaces with explicit foregrounds. No new approval stage or hardware command is added. Modal/background guards remain covered by actual-App tests.

The reviewed GUI does not send a selected key ID, mask, depth, numeric threshold, or additional calibration parameter. UI selection is an instruction for physical holding in the existing released/held sequence. It does not establish that only that position's analog state changes, and it does not prove real-device improvement or persistence.

## Resolved P2 — default calibration stages previously overflowed

The initial frozen candidate (`app.rs` hash `8d0f2047…`) had usable wheel ranges at 960×820 in both themes: AwaitingHeldKeys **22.00 pt**, AwaitingFinish **2.00 pt**, Complete **10.00 pt**, Aborted **10.00 pt**; idle was 0.00. All 66 positions and main phase controls fit, but the bottom diagnostic disclosure initially clipped in held/complete/aborted. Evidence remains in `generic-calibration-pre-fix-review.md`, `generic-calibration-frame-probe.log`, and the immutable pre-fix artifact `libniz_gui-generic-review.rlib`.

The final fix (`app.rs:358-377`) shortens the global guide, scopes its vertical spacing to that card, and moves numerical capability detail to existing disclosures. Fonts, key geometry, default/minimum window sizes, controls, warnings, and hardware gates are preserved. The no-scroll test `crates/niz-gui/src/app_tests.rs:135-235` now checks 11 states in both themes: clean assignment, single dirty assignment, swap dirty assignment, idle calibration, input test, held/finish/complete/aborted calibration, and both released/held wait states. It checks diagnostic visibility, all 66 positions, relevant phase/Cancel controls, and actual main-header displacement after wheel input.

The independent actual-App flow probe linked to the final frozen implementation confirms:

| Theme | Idle | Held | Finish | Complete | Aborted |
|---|---:|---:|---:|---:|---:|
| Light | 0.00 pt | 0.00 pt | 0.00 pt | 0.00 pt | 0.00 pt |
| Dark | 0.00 pt | 0.00 pt | 0.00 pt | 0.00 pt | 0.00 pt |

All 66 positions, the relevant phase controls, active Cancel, footer, and diagnostic disclosure are visible before scrolling in every probed state. Consent and Start actual galley centers differ by **0.00 pt** in both themes. The probe uses the public actual App with the in-process demo worker, calls `raw_input_hook` before egui logic/UI, uses galley bounds and clip rectangles, resets scrolling to the top, and measures main-header displacement after wheel events. It does not construct a HID backend.

## Independently executed verification

`cargo test -p niz-gui --all-targets` passes **102 tests** on the final frozen source: 37 library, 27 controller, 12 editor, 17 keyboard, 9 worker. Final log: `reviewer-generic-calibration-fixed-gui-tests.log`.

The added actual-App tests cover the danger row in both label variants/themes, red appearance, foreground contrast, centers, and cancellation preserving draft/history/generation/pending; generic physical selection with no editor side effects; qualified/missing/ambiguous/custom/remapped baseline Fn locator; popup/modal/pending freeze; actual demo Begin/held/finish guide capture and generic terminal result; all 66 physical positions fully visible at default and scroll-reachable at minimum. The expanded existing geometry regression covers active/wait/terminal calibration frames in both themes. Existing tests continue to cover input capture/consumption, system/light/dark appearance, modifier provenance, unsupported raw assignment preservation, original target/approval gates, and worker cleanup.

Independent probe source/log: `generic-calibration-frame-probe.rs` and `generic-calibration-frame-probe-fixed.log`. Its final linked immutable GUI artifact is `libniz_gui-generic-fixed-review.rlib`.

Contrast checks substantiate the tested palette/rendered labels, including red execution foregrounds, and do not constitute formal accessibility certification. Headless/demo checks do not verify native screen rendering, real analog response, physical key-release/hold state, sensor persistence, or hardware rollback. Native screen QA, full workspace checks, and packaging remain the parent coordinator's separate verification.

## Frozen snapshot and final integrity

All six source files and the debug executable matched the refreshed `generic-calibration-source-sha256.json` before tests and again after tests/probes. Independent records: `reviewer-generic-calibration-fixed-sha256.json`, `reviewer-generic-calibration-final-integrity.json`. `input_test.rs` and `lib.rs` are identical to the earlier reviewed final six-file snapshot.

- `crates/niz-gui/src/app.rs`: `bffcefe9f488ca920f86737caed1bb85fd26a146a1cac6fa49a5c80db6d1e42a`
- `crates/niz-gui/src/keyboard.rs`: `18deca00070f919d8f75e2729cf45429b784e9fd6dd8937b32f191ccfd80ca2b`
- `crates/niz-gui/src/app_tests.rs`: `0b038b8c19cb80ce8577e71cde65b7e1baf32c17852b49f5e6bc0498e4cdc364`
- `crates/niz-gui/src/theme.rs`: `fc7016316f63c689877bff4388eea79db7eaadf9f7912b62f0bed228fd05b8c9`
- `crates/niz-gui/src/input_test.rs`: `5b0f21003a492214df2c631fc793b28a6825e767053ca532e35e0165833fd4b5`
- `crates/niz-gui/src/lib.rs`: `e8f574c5ab061c2da2189acf38225e0d9eee07b69bae809d023dcd06e5fb02c3`
- `target/debug/niz-gui`: `18284b3a99f108f631ebdd0d8a05aee878aa449eda7e756c25330371ba11e442`
- Immutable probe-linked GUI library: `1cab0d9392f60caaba92a2b8b703fce73697d3a0662bb22415482fa9bb5d714d`
