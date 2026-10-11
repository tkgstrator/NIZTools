# Manual raw threshold snapshot GUI integration

Implemented on native host `devonlys-MacBook-Pro.local` in `/Users/devonly/Developer/personal/NIZ`, with ownership limited to `crates/niz-gui/**`. The Core/Device API belongs to the device worker; CLI, documentation, native probes, screenshots, and final native visual/HID verification belong to the coordinator. This worker performed no native GUI/HID, vendor-execution, distribution, Git-history, or Simulator operations.

The feature reads and displays a **manual snapshot of normal-scan raw threshold values**. It does not implement individual threshold writes, physical travel/millimeters, continuous ADC readings, persistence verification, or qualification of every scan mode. The existing individual-setter unsupported statement remains prominent; legacy calibration Begin/Held/Finish controls and the global Fn+7 threshold explanation remain absent.

## API and ownership

The device worker directly confirmed and supplied `RawThresholds` with exact identity/firmware, private fixed 66 physical-order values, immutable accessors, original DC rows, and a validated pure `from_reports` constructor. `Session::read_raw_thresholds()` owns the exact fresh F9/DC01 transaction and its poison/failure behavior. The GUI uses that announced contract without copying its key mapping or packet parser.

`protocol.rs` adds only targeted `Command::ReadRawThresholds`, explicit `RawThresholdSource::{Device, Simulation}`, and a complete RawThresholds response bound to the concrete target. The Worker rejects stale/replayed ticket/generation, absent/wrong concrete target, active calibration, invalid baseline, or unsupported baseline metadata before entering its reader. It validates returned identity/firmware before publication and invalidates its baseline eligibility on reader I/O/parser or returned-metadata failure. Complete rejected response rows and native error evidence remain in Failure; there is no partial table cache or speculative cleanup.

NativeBackend calls its retained owned Session and normal per-path lock. It does not open another Session, re-enumerate into a different device, bypass a lock, or send fallback commands. After reader error it drops the poisoned Session/baseline, preserving the device API's passive evidence collection and error. DemoBackend deliberately generates eleven ordered synthetic DC mode1 rows with flat-index values 0..65, runs them through the validated device parser/mapping, and tags them Simulation. It does not use a captured hardware analog table.

Controller captures the concrete target, generation/ticket, and full immutable baseline at dispatch. A reply is installed only for the matching raw-read command and matching current source/target/identity/firmware; stale/duplicate events cannot replace the snapshot. Wrong response types, target/source changes, and metadata mismatches detach the live source and publish no values. The readonly accessor also refuses a cached value after direct generation/target/baseline/live/close changes. Dispatching a new raw read clears the old value; baseline read, path change/device loss, file import, application/calibration, raw/general error, and close invalidate cached data. Read success preserves output assignment/editor/history and the live baseline.

## GUI behavior

- The lower physical selector changes local position only, and the selected official legend/ID remains independent of remapped output. There is no automatic reader dispatch from selection, unfocused Enter, appearance changes, or InputTest.
- The neutral `しきい値を読み込む` button requires an exact live qualified baseline and attached target, no busy/calibration/close ownership, no confirmation modal, and no current/frame-start popup. The accessibility follow-up now permits ordinary explicit button activation by pointer, focused Enter/Space, or AccessKit Click; destructive operation gates remain pointer-specific. The button adds no confirmation or numeric edit control.
- Before a read, the value is `読み取り時の生しきい値: 未取得`; while owned pending it says `読み込み中…`; success displays decimal plus hex for the selected physical position. Failure returns to 未取得 and the existing diagnostics retain the error/evidence. No zero or older snapshot is fabricated.
- Visible meaning copy says `通常スキャンの内部値。押し込み量(mm)ではありません。` A simulated snapshot visibly states `DEMO: シミュレーション値です。実機から読み取った値ではありません。`
- The exact observed caveat is present in idle helper, read-button tooltip, and snapshot details: `読み取り中は入力が一時停止し、補正状態が既定値に戻ることがあります。` The helper asks users to release keys first. It does not describe the request as stateless. While legacy calibration owns the device, the unavailable manual-read helper is omitted to preserve default fit; active scope/cancel remain visible.
- Snapshot details preserve concrete product/path/identity, firmware, successful receive timestamp, connection generation, device-versus-demo source, original row count, and the normal-scan/snapshot-only boundary.
- Existing physical geometry, themes, Body14 device-font rows, InputTest behavior, confirmation/busy/write guards, and owned active-calibration cancellation were preserved.

## Verification evidence

`gui-ui-red.log` records the initial real rendered GUI failure on the missing 未取得/read surface before production changes. Six new rendered tests cover manual mapped display (A slot31=13, S slot32=14, '=' slot13=60 for synthetic flat-index rows), simulation provenance, timestamp/source, pending/error without old values, source/modal/popup/close gates, Enter/theme/key/InputTest without refresh, both-theme neutral-button hover geometry, and default/minimum snapshot fit.

Six new Controller integration tests and four Worker integration tests exercise real routes with captured boundary requests, source/ticket/target guards, unchanged editor/history, cache lifecycle, and error poison behavior. Supplemental failure-evidence assertions captured RED for metadata rejection before both Worker and Controller retained original rows (`gui-rejected-evidence-red.log`, `gui-worker-evidence-red.log`; final integration GREEN). The independent fallback source reviewer found one further P2: a raw response to an owned List command lost its raw rows in the early command-tag rejection. `gui-review-wrong-command-red.log` verifies the missing evidence, and `gui-review-wrong-command-green.log` verifies the fix: rejection still empties cache/detaches while retaining all eleven rows. The independent fallback reviewer subsequently inspected both evidence-retention fixes and reran all 122 GUI tests with zero failures; all 17 GUI source/test hashes matched the freeze. No preferred MCP review is implied.

A full GUI run exposed a default-size overflow only in existing active-calibration state5 after adding a manual-read runtime warning (`gui-alltargets-test.log`). Investigation identified the extra helper row. The helper now appears when manual reading is relevant, and the unchanged 22-state default-fit assertion passed (`gui-default-fit-green.log`). Later full GUI tests pass with the final caveat copy.

- `cargo test -p niz-gui --all-targets -- --nocapture`: PASS before the final evidence-only review fix, **122 tests** (47 library = 40 app + 7 InputTest; 27 controller, 12 editor, 17 keyboard, 6 raw controller, 4 raw worker, 9 worker), zero failures (`gui-alltargets-final.log`).
- After the final review fix, all 6 raw Controller + 4 raw Worker tests PASS (`gui-review-wrong-command-green.log`).
- GUI-scoped `cargo fmt -p niz-gui --check` and final `cargo fmt --all --check`: PASS (`gui-scope-fmt.log`, `gui-full-fmt-final.log`). Earlier full checks saw other owners' in-progress CLI/Device formatting; final full check is clean.
- `cargo clippy -p niz-gui --all-targets -- -D warnings`: PASS, exit0 after final review fix (`gui-clippy-final.log`).
- `cargo build -p niz-gui`: PASS, exit0 after final review fix (`gui-debug-build.log`).
- `cargo test --workspace --all-targets -- --nocapture`: PASS, exit0, **289 tests**, zero failures (`gui-workspace-final.log`). This run includes the fresh final GUI122 after the review fix plus current Core/Device/CLI integration.

The 22 existing collapsed default states retain zero main wheel-scroll range; all 66 physical positions fit 960x820 and remain vertically reachable without horizontal clipping at 700x580 in Light/Dark. Successful raw snapshot default/minimum fit, active cancel, stationary hover at existing scales1/2, the new read-button hover, Body14 rows, and InputTest regressions all pass in the GUI suite. Rendered-shape evidence is not a native screenshot or live hardware read.

## Freeze and remaining work

The GUI source is frozen for coordinator integration; the independent fallback reviewer inspected the fixes and reran all GUI tests successfully. `gui-source-debug-sha256.json` captures all 17 GUI Rust sources/tests and the debug executable, whose current SHA256 is `76503900fafffa6b5d565c018fa7f14a3d0843f14ec78b448aba20ffd7d32f3e`. If a Core/Device implementation body changes afterward, the coordinator must rebuild/recheck the executable even though the GUI source contract is unchanged.

Exact-mode hardware qualification, independent device/API review, final native GUI screenshot/read verification, complete final workspace freeze/hashes, and any distribution update remain coordinator work. No native-read claim is made by this worker. Individual selected-key threshold changes remain unsupported.


## Bounded accessibility follow-up

The native AX path could not activate the readonly reader because the button used `clicked_by(Primary)`. Local pinned egui Response documentation and Context implementation confirm that ordinary `clicked()` also accepts focused Enter/Space and AccessKit Click via FAKE_PRIMARY_CLICKED. The only production change is the reader callback changing to `.clicked()`; no destructive Apply/calibration callback, Controller/Worker/API, layout, or native-I/O policy changed.

`gui-accessibility-red.log` records a meaningful focused Enter failure against the prior predicate. Tests discover the actual read-button interaction ID before a pending frame disables/surrenders focus, then deliberately focus it after settlement. Focused Enter and Space request a new snapshot, while offline, pending, close, confirmation-modal, and popup states refuse focused keyboard activation. The old blanket Enter-negative test now explicitly surrenders focus and still proves zero automatic requests from unfocused Enter/key selection/theme/InputTest.

`gui-accesskit-red.log` records a meaningful failure targeting the actual accessibility-tree Button node for `しきい値を読み込む` with Action::Click. The test uses enabled egui AccessKit tree output and real ActionRequest events, with no added dependency or fabricated production helper. With the fix, the actual node activates the reader and produces the expected demo snapshot; offline/pending/close/modal/popup paths refuse the same request.

The final focused raw GUI suite passes **9 tests**, zero failures (`gui-accessibility-green.log`). Final whole-workspace all-targets verification passes **292 tests**, zero failures, including **125 GUI tests** (50 library = 43 app + 7 InputTest; 27 controller, 12 editor, 17 keyboard, 6 raw controller, 4 raw worker, 9 worker; `gui-accessibility-workspace.log`). Final full fmt, GUI Clippy -D warnings, and debug build pass (`gui-accessibility-fmt.log`, `gui-accessibility-clippy.log`, `gui-accessibility-debug-build.log`). The independent fallback reviewer is inspecting the focused change after this freeze; native AX/HID verification remains coordinator work.

`gui-accessibility-source-debug-sha256.json` and the refreshed main GUI manifest freeze all 17 source/test files and the current debug binary. Only app.rs, app_tests.rs, and the debug binary differ from the prior raw-read freeze; the other 15 GUI source/test hashes match. The prior manifest is preserved as `gui-source-debug-pre-accessibility.json`. No native GUI/HID, Git, distribution, or Simulator action was performed by this worker for this follow-up.
