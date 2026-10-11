# Task 2 GUI implementation evidence

Host/workspace: local Mac, `/Users/devonly/Developer/personal/NIZ`.
Owned edits: `crates/niz-gui/src/{app,app_tests,controller,protocol,worker}.rs`, GUI integration tests including new `tests/held_calibration.rs`. Existing GUI status literals and legacy backend test implementations adapted to the new device interface. No device/document/script edits, commits, pushes, real HID operations, native app launches, or delegated workers were performed by this worker.

## Implemented behavior

- `Command::BeginHeld { backup, position }` is classified as calibration ownership immediately while pending and carries the attached concrete target. Controller entry validates pointer authorization, physical range, exact device/firmware/group qualification, live baseline and existing metadata safeguards; stale/duplicate/read/edit/close guards remain in effect.
- Native backend consumes the planned Session API. Worker routes held-only successful DE status directly through finish to `Response::Finished`; legacy full calibration still exposes its separate explicit Finish route.
- Select physical key, prepare with keys released, then hold only that physical key and use the red Execute button. Execute and Cancel share a horizontal row. Both prepare and Execute require an actual primary-pointer click; focused Enter/Space and AccessKit Click do not dispatch either action. No OS-key-event requirement blocks weak/remapped physical keys.
- The physical keyboard remains below controls and freezes during pending/active operation. Selection alone never writes or edits the draft.
- Complete 66-key before/after observations show target literal values and other-key change count; other changes use the warning palette. Complete post observations update the selected threshold display after raw read cache invalidation. Missing/partial post observations are unavailable, never unchanged. No observed change explicitly does not establish improved response.
- Collapsed results show a quick changed-neighbor list with official physical legends, all 66 literal before/after pairs, target, backup/settings/unlock state and caveats. Complete values survive a later settings failure. DEMO result and table provenance are explicit simulation.
- Concise neighbor/analog-restore warning is visible; detailed limitations stay collapsed. Existing light/dark theme, default fit, hover geometry, disclosure glyph alignment, font, input test, editor history and legacy cancel controls remain covered.

## RED and GREEN evidence

1. Before production edits, `cargo test -p niz-gui --lib held_calibration -- --nocapture` produced two intended rendered failures: missing preparation action and absent compact prepare/hold/result flow (`gui-red.log`).
2. After the initial flow implementation, targeted held flow/cancel tests passed. A full GUI run caught four regressions/obsolete expectations (`gui-green-attempt.log`): old unsupported capability text in two tests, extra legacy active-state height clipping, and expanded legacy unlock detail clipping. Preparation moved into the existing read row; pending preparation no longer duplicates legacy active controls; held-only detail caveats no longer affect legacy result layout. Old unsupported-text expectations were updated to the new visible held-only action.
3. A new test for complete post-value fallback failed at `しきい値: 31` before fallback/warning behavior was added (`gui-result-red.log`). It now passes and checks neighbor warning color and selected neighbor value.
4. `cargo test -p niz-gui --lib default_collapsed -- --nocapture` passed all 22 light/dark default-state combinations with wheel scroll range 0.00 (`gui-fit.log`).
5. `cargo test -p niz-gui --test held_calibration` passed 3 consumer contract tests: controller ownership/target/nonpointer/duplicate/close; auto-finish/complete values/replay rejection; foreign target/range/generation/active read/cancel (`gui-contract.log`).
6. `cargo test -p niz-gui --lib held_` passed 10 filtered tests, including actual focused keyboard and AccessKit rejection, all 66 literal table pairs reachable through scrolling, and popup/modal/close/pending guards (`gui-held-green.log`).
7. Full `cargo test -p niz-gui` passed 135 tests before the final two rendered tests were added (`gui-suite.log`). Final complete GUI run, including those two tests, passed all 137 tests (59 library, 27 controller, 12 editor, 3 held contract, 17 keyboard, 6 raw controller, 4 raw worker, 9 worker; zero failures): `gui-suite-final.log`.

Only the rendered flow and complete-after fallback tests were observed failing before their respective production changes. Additional controller/worker and edge-case tests were added for regression coverage, and are not represented as individually observed RED cycles.

## Boundaries

Native hardware held-key calibration and visual verification were not executed by this worker. The coordinator owns independent review, whole-workspace fmt/clippy/tests, app build/signing and native screenshot verification. GUI production files were frozen before coordinator review; only tests and this evidence document continued afterward.

## Subsequent independent-review fix

Independent review found P2 display-source invalidation after selecting/replacing/removing a concrete device. It was fixed with four controller status invalidation assignments; four regressions were observed RED then GREEN, and a fresh read-generation check was added. Final scoped 15 held-filter tests passed. See `gui-review-fix.md`; the initial 137-test full run predates these five added regressions. Root owns the subsequent full workspace run and rebuilt app.
