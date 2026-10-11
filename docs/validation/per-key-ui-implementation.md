# Bounded per-key threshold UI correction

Codex implementer verified native access on `devonlys-MacBook-Pro.local` at `/Users/devonly/Developer/personal/NIZ` on 2026-10-11. The substantive edit owns `crates/niz-gui/src/app.rs` and `app_tests.rs`; the coordinator additionally authorized one physical-board tooltip copy correction in `keyboard.rs`. Rustfmt adjusted whitespace in owned files. No controller, worker, protocol, core, device, distribution, Git-history, or Simulator operations were changed or performed by this worker. No native GUI or live HID interaction occurred.

This implements the truthful unsupported UI branch of `/private/tmp/niz-perkey-20261011/ui-plan-review.md`. It does **not** implement the requested selected-key hardware threshold adjustment. The coordinator's static firmware evidence establishes that existing calibration commands do not take a selected host key or threshold value. The GUI therefore offers no enabled operation that could imply selected-key isolation, no guessed threshold values, and no alternate software filtering.

## Resulting behavior

- `app.rs:353` keeps the lower 66-position physical selector and shows `調整するキー: #xx · legend`. The visible tab is `入力しきい値` (`app.rs:758`). Official physical legends remain independent of remapped outputs.
- The first card visibly says `キーごとのしきい値変更は未対応です` and `この機種の個別調整にはまだ対応していません。キーの選択だけでは設定を変更しません。` The board tooltip repeats the no-setting-effect statement (`keyboard.rs:445`). There is no numeric editor, guessed current value, adjustment execute button, or acknowledgement checkbox.
- The global Fn+7 threshold card, `trigger_fn_position`, its imports, and obsolete acknowledgement/guide fields were removed. No global-threshold instruction survives inside collapsed details. GUI calls to `Controller::begin`, `held`, and `finish` were removed. Backend APIs and their tests remain unchanged.
- An operation already owned by the controller remains visible as `進行中の校正 · 全キーに作用` with an accessible `中止を要求` button, including pending/cleanup states. There are no continuation write buttons. The selector remains frozen under busy/calibration/close, modal, and current/frame-start popup guards.
- Terminal calibration text is generic and does not acquire the next selected key's name. Existing target details, backup paths, original errors, failure phase, trace, byte-verification, and uncertain unlock statements remain in result/diagnostic disclosures (`app.rs:399-450`). The numerical unsupported fact stays prominent at the top.
- The coordinator's prior Body 14 device-information font correction and regression test were preserved. Appearance, key geometry, dimensions, and the input-test implementation were preserved.

## Test evidence

The first failing rendered-UI assertion is saved in `ui-threshold-red.log`. `ui-behavior-red.log` records the four new behavior tests failing on absent selected-summary/capability/active-scope/tooltip behavior before production changes. The initial tooltip probe repeated PointerMoved events, preventing tooltip delay from elapsing; after examining egui's last_move_time and tooltip gating, the test was corrected to hover once and keep the pointer stationary. `ui-tooltip-red.log` is the valid RED against the prior tooltip copy, and `ui-behavior-green.log` is the final four-test GREEN.

`app_tests.rs:1239-1510` covers the unsupported capability in Light/Dark and loaded/offline states, all 66 hand-listed physical legend/position pairs, remapped output, repeated clicks and Enter, unchanged draft/baseline/undo/redo/readiness/assignment selection/generation/target/path, and a real controller ticket-sequence sentinel proving no unrequested dispatch. No mock dispatch counter was added. The cancel regression begins the in-process demo through the controller only from the test, freezes selection before and after its pending stage, clicks the real GUI cancel, then verifies Aborted, cancellation_requested, unlock Sent, no active/pending operation, and unchanged target/generic terminal text after another selection. Existing popup/modal/pending guard coverage adds close-requested coverage.

- `cargo fmt --all --check`: PASS, exit 0 (`ui-fmt.log`).
- `cargo test -p niz-gui --all-targets -- --nocapture`: PASS, exit 0, **106 tests** (41 app, 27 controller, 12 editor, 17 keyboard, 9 worker; `ui-alltargets-test.log`).
- `cargo clippy -p niz-gui --all-targets -- -D warnings`: PASS, exit 0 (`ui-clippy.log`).
- `cargo build -p niz-gui`: PASS, exit 0 (`ui-debug-build.log`).
- `cargo test --workspace --all-targets`: PASS, exit 0, **249 tests**, zero failures (`ui-workspace-test.log`).

The GUI suite preserves all 66 visible physical positions at 960×820, all positions reachable without horizontal clipping at 700×580 in both themes, unchanged hover/press geometry at scales 1 and 2, and zero main wheel-scroll range in all 11 collapsed default states. The 11-state assertions now check active cancel only and generic terminal results; removed write controls are not tested as present. The existing device-font test passes for all eight expanded Body rows in Light/Dark.

## Freeze and limitations

`ui-source-debug-sha256.json` freezes app.rs, app_tests.rs, keyboard.rs, lib.rs, theme.rs, input_test.rs, and `target/debug/niz-gui`. Debug binary SHA-256: `ec60838d9ab2edfc3fb90175bb1ae0236d51458ee9ea4a122a8e5c81557aee33`.

The source is frozen for coordinator integration and independent actual-source review. This worker does not claim a Claude Code MCP or LocalGPT implementation review took place. Native screenshots/visual verification and final acceptance belong to Codex coordinator; headless rendered-shape tests are not native screenshot evidence. The user's selected-key hardware capability remains unsupported and must be reported plainly. No Simulator resources were created or booted, so no Simulator cleanup was needed.
