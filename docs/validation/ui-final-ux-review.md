# Final generic calibration UX source review

Reviewed on 2026-10-11, on native host `devonlys-MacBook-Pro.local`, in `/Users/devonly/Developer/personal/NIZ`. This is a bounded independent source and headless actual-App review of the generic physical calibration picker, its guidance and state guards, the confirmation danger actions, and the final default-fit refinement. It continues the prior input/theme UX review; it does not repeat firmware research or claim native screenshots. Repository source, Git, GUI, HID, firmware and OS state were not mutated. Preferred LocalGPT/Claude review tools are unavailable; the authorized Codex fallback was used.

## Findings

**No remaining P0, P1, P2 or P3 finding in the reviewed final scope.** The actual source matches the user's clarified goal: choose a physical key and follow calibration instructions to improve its response. It does not claim software-selected numerical sensor thresholds or a verified analog outcome.

- **Physical selection and assignment separation:** `app.rs:426-437` renders the 66-position picker below the calibration steps. `keyboard.rs:384-425` takes only a slot and a blocked flag, uses immutable physical legends and the existing board geometry, and returns an optional slot. It has no Editor/profile/controller access. Choosing S, Fn, or another position changes only the guide; it does not create an assignment draft or dispatch an operation. The selected legend, position number and tooltip distinguish the physical position from mutable output assignments.
- **Procedure provenance and original guards:** `app.rs:378-423` retains the explicit experimental/irrecoverable warning and acknowledgment, requires a live baseline and existing controller availability before start, and uses primary-pointer activation for all three calibration stages. The guide is captured only after `Controller.begin` successfully dispatches (`app.rs:395-398`); pending/active/modal/popup state disables subsequent picker changes (`app.rs:434`). The held-key instruction uses the captured guide (`app.rs:403-408`). Completion remains generic (`app.rs:439-447`), so selecting the next key cannot relabel the prior calibration result. Existing controller exact-target, ownership, cancellation and cleanup gates remain in `controller.rs:82-96,252-310`.
- **Honest scope and evidence:** The visible step text states that the calibration uses all released keys and the keys actually held, and that screen selection is guidance (`app.rs:385-387`). The global two-stage shortcut remains separately labeled as global, with current value unavailable (`app.rs:361-376`). Its qualified locator uses the live Editor baseline and existing action149 qualification, then requires a unique basic156 position (`app.rs:835-850`, `editor.rs:270-290`); the fixture correctly resolves position #62 with physical legend Right Alt rather than assuming the printed Right Fn position. Absent, duplicate, offline or customized evidence falls back to explicitly unconfirmed manual guidance. No draft assignment qualifies a current-device claim.
- **Capability limitations and hierarchy:** The numerical/per-key unsupported statement is now muted text inside the existing target/result disclosures (`app.rs:450-454,492-496`). This is proportionate to the user's clarified calibration intent. The primary physical-scope explanation, irreversible analog warning, acknowledgment, and stage actions remain visible. Target, original error, backup, byte-verification and uncertain unlock evidence remain available (`app.rs:449-489`). The completion text asks for an input test instead of asserting response or persistence success.
- **Danger actions and themes:** Actual native/demo execution labels use the red danger role, with neutral Cancel on the same horizontal row (`app.rs:566-580`). Cancel retains draft/history and sends no operation. The calibration start uses the same red role. Both Light and Dark provide explicit white foreground on their red surfaces (`theme.rs:66-67,95-96,110-113`); the actual rendered confirmation contrast and row center are tested, rather than only mirroring palette literals. The review/apply opener remains a normal primary action; the final execution is the danger action.
- **Final fit and interaction continuity:** The refinement only shortens/qualifies global guidance, reduces that card's paragraph gap, and moves the numerical limitation into an existing disclosure. It preserves window dimensions, fonts, 46-point key geometry, the board's place below the step card, warnings and all operation guards. No scroll suppression or clipping workaround is introduced. The actual-App no-wheel regression now verifies the relevant active and terminal calibration layouts as well as idle/default assignment, dirty assignment, swap and input-test layouts.

## Fresh verification

After the final re-freeze, I independently ran `cargo test -p niz-gui --lib` in the verified native workspace. It completed with exit code 0: **37 passed, 0 failed**, including 30 actual-App/contrast tests and 7 input-model tests. The test harness uses the in-process demo backend, not a physical device. I inspected the actual assertions as well as the result:

- `app_tests.rs:135-235` covers 11 states in each Light/Dark theme at 960×820: clean assignment, dirty assignment, swap, idle calibration, bounded input transcript, held stage, finish stage, complete, aborted, pending release and pending held. It asserts visible closed diagnostics, all 66 calibration labels, applicable phase/cancel/result text, and unchanged header geometry after a wheel event. All 22 combinations passed.
- `app_tests.rs:878-921` checks both execution labels, rendered red surface, ≥4.5 text contrast, execution/Cancel text centers within 1 point, and cancellation preserving draft/history/generation with no pending operation.
- `app_tests.rs:926-1005` checks physical guide independence, visible unsupported disclosure, #62 Right Alt locator qualification, popup-closing/modal/pending guards, and selectable Fn positions.
- `app_tests.rs:1009-1032` performs an actual in-process demo calibration sequence and checks captured guide, active freeze, generic completion and next-selection provenance.
- `app_tests.rs:1036-1062` checks all 66 positions visible at default size and reachable by vertical scroll at 700×580, without horizontal label clipping. Existing theme, rendered text, shared row centers, input-event isolation and assignment tests also remained green.

Source/hash evidence: immediately after the final freeze, I independently hashed all six specified sources and the debug binary; **all seven match** `/private/tmp/niz-ui-reference-20261011/generic-calibration-source-sha256.json`:

| File | SHA-256 |
| --- | --- |
| `crates/niz-gui/src/app.rs` | `bffcefe9f488ca920f86737caed1bb85fd26a146a1cac6fa49a5c80db6d1e42a` |
| `crates/niz-gui/src/keyboard.rs` | `18deca00070f919d8f75e2729cf45429b784e9fd6dd8937b32f191ccfd80ca2b` |
| `crates/niz-gui/src/app_tests.rs` | `0b038b8c19cb80ce8577e71cde65b7e1baf32c17852b49f5e6bc0498e4cdc364` |
| `crates/niz-gui/src/theme.rs` | `fc7016316f63c689877bff4388eea79db7eaadf9f7912b62f0bed228fd05b8c9` |
| `crates/niz-gui/src/input_test.rs` | `5b0f21003a492214df2c631fc793b28a6825e767053ca532e35e0165833fd4b5` |
| `crates/niz-gui/src/lib.rs` | `e8f574c5ab061c2da2189acf38225e0d9eee07b69bae809d023dcd06e5fb02c3` |
| `target/debug/niz-gui` | `18284b3a99f108f631ebdd0d8a05aee878aa449eda7e756c25330371ba11e442` |

## Practical boundary

This review establishes source behavior and headless layout/state assertions for the frozen files. It does not establish actual analog calibration, firmware persistence, typing recovery after unlock, a per-key numeric setter, release packaging, or native pixel rendering. Expanded details, variable failure messages and minimum-size windows may use the retained vertical scroll area. Parent Codex owns final native visual and artifact verification; those results are not claimed here.
