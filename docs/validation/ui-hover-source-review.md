# Final independent technical review — selectable hover jitter

**Verdict: the confirmed P2 hover-layout regression is resolved; no remaining material P0–P2 finding in the reviewed frozen source.** This is a read-only technical assessment of the shared selectable helper, all changed GUI callsites, and executed geometry/behavior checks. Native-screen QA and packaging remain separate parent verification.

Host: `devonlys-MacBook-Pro.local`. Workspace: `/Users/devonly/Developer/personal/NIZ`.
Source matches `hover-jitter-source-sha256.json` before and after the review. Independent records are `hover-jitter-reviewer-source-sha256.json` and `hover-jitter-reviewer-final-integrity.json`.

## Original finding and root cause

The independent corrected diagnosis (`hover-jitter-independent-probe.md`) reproduced unselected swap mode, reaction tab, and right-Fn layer growing by 2 points on hover, moving their own text by 1 point and following sibling text by 2 points. Selected controls and ordinary buttons/keycaps remained stable. Both themes and default/minimum viewports reproduced it, with memory-popup false.

The pinned egui 0.36.2 `Button::selectable(false, …)` omits the inactive frame while retaining inner padding already reduced by the 1-point theme stroke. Hover retains that stroke, making total width 2 points larger. This is supported by `widgets/button.rs:78-82`, `:364-367`, and `widget_style.rs:163-167`. It was not expansion, scrollbar allocation, or tooltip ownership. The first independent zero-result matrix omitted those unselected targets; this coverage gap and the correction are explicitly recorded in the diagnosis report.

## Actual fix and retained semantics

`crates/niz-gui/src/theme.rs:5-21` uses a single stable child scope for selected and unselected controls. Only unselected inactive fill/border colors become transparent. Stroke width, selected style, hovered/active styling, global style, and ordinary button outlines remain intact. `Button::selectable(...).frame_when_inactive(true)` consistently reserves the frame in every state. Pinned `Frame::total_margin` and `outer_rect` (`frame.rs:327-330`, `:350-351`) include stroke width regardless of color, so the transparent inactive border still reserves the correct layout space.

`theme.rs:23-35` returns the inner Button Response and mirrors the native `Ui::selectable_value` comparison, clicked, assignment, and mark_changed behavior exactly (`ui.rs:1939-1950`). Reclicking the already selected value reports clicked without changed; hovering does not change data. The scoped style does not globally remove outlines or make disabled widgets interactive.

All application selectable callsites use the shared helper:

- Assignment mode/layers: `app.rs:177-183`.
- Output menu: `app.rs:216-225`.
- Input reference layers: `app.rs:526`.
- Device menu: `app.rs:621-634`.
- Appearance menu: `app.rs:790`.
- Main tabs: `app.rs:805-807`.
- Position menus: `app.rs:915-930`.
- Swap From/To chips: `keyboard.rs:260`.

A source search finds no remaining native selectable_label/value GUI callsite. Existing callbacks, menu closing, selected-value comparisons, output readiness, popup closing-frame protection, modal/disabled guards, controller calls, and mouse-only hardware activation paths are preserved. `input_test.rs` and `lib.rs` retain their previously reviewed hashes. No new device command or hardware parameter is introduced.

## Independent verification

`cargo test -p niz-gui --all-targets` passes **104 tests**: 39 library, 27 controller, 12 editor, 17 keyboard, 9 worker. Log: `hover-jitter-reviewer-gui-tests.log`.

The new actual-App regression `app_tests.rs:211-331` covers unselected mode/tab/layer plus selected/ordinary/keycap controls, first hover frame, 120-frame dwell, pressed state with release outside, and leave. It compares galley rectangles, actual glyph mesh bounds, bounded control shapes, and following neighbors at both themes, default/minimum viewports, and native scales 1/2. It also verifies baseline/draft, selection, readiness, undo/redo, target, generation, pending state, and current view remain unchanged by hovering/canceled press. The appearance-menu case checks the same geometry while a real menu is open. The former rendering bug produces a text delta in the red case, so the test is not merely mirroring color constants.

The helper behavior test `app_tests.rs:333-393` verifies hover/press without release does not click/change, real release selects and marks changed, same-value reclick stays unchanged, disabled activation fails, and keyboard activation works in both themes. Its value/changed logic is also directly checked against the pinned implementation above. Existing actual-App modal/popup/appearance/input tests, 11-state zero-default-scroll test, row centers, all-66/minimum reachability, rendered contrast, raw assignment preservation, controller target checks, and worker sequencing/cleanup all continue passing.

The separately compiled independent actual-App probe uses the final immutable library and runs **32 hover targets**: the former failing controls, selected controls, ordinary button/keycap, plus appearance-menu choice, across both themes and both viewport sizes. It records every intermediate text/control displacement, live interaction state, logical Response rectangles on hover frames 0/1/60, and after leave. Results are **0.00 pt text/control delta in all cases**. The previously failing widths are now fixed at 125.91 pt (swap mode), 90.00 pt (reaction tab), and 52.19 pt (right-Fn) during both inactive and hovered states. They remain constant after leaving.

The four appearance-menu cases retain real popup ownership during hover; clicking Dark switches the effective theme and closes the popup correctly. Tooltip cases remain outside menu ownership. Probe source/log: `hover-jitter-independent-fixed-response-probe.rs/.log`. Ordinary controls and keycaps retain their prior geometry. This probe supplements the executed press/changed/disabled checks rather than replacing them.

## Limits and integrity

The reviewed defect is source-level selectable frame allocation. The independent tests/probe substantiate logical/text/control geometry and application behavior; they do not constitute native GPU screenshot QA, formal accessibility certification, or live-device analog validation. No source/Git edits, native GUI, HID, or Simulator operations were performed by this reviewer. Only temp artifacts and ordinary build/test outputs were produced.

Final source hashes:

- `crates/niz-gui/src/app.rs`: `ccbc87e81c032640068a348c3a82c7f384f17dfae2147150e01c3c34dc2ae006`
- `crates/niz-gui/src/keyboard.rs`: `cf2bb292ee861daaad5a7a27361cc31ea47e0238be6e03891e89683f1a8b2f3b`
- `crates/niz-gui/src/app_tests.rs`: `ddd789a5990044429f57dedeb49fdac1a46d462df355fee9566426f30f877fe4`
- `crates/niz-gui/src/theme.rs`: `9838343720683b60dd279b1a66eea6ebaf087c1a62efa75167c1e38d832e8ac6`
- `crates/niz-gui/src/input_test.rs`: `5b0f21003a492214df2c631fc793b28a6825e767053ca532e35e0165833fd4b5`
- `crates/niz-gui/src/lib.rs`: `e8f574c5ab061c2da2189acf38225e0d9eee07b69bae809d023dcd06e5fb02c3`
- Probe-linked immutable GUI library: `e8a6f34398eece5d84a0237ce913100ff6107f5d9251db78ec079861156f8908`
