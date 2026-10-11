# Final hover-jitter UX source review

Date: 2026-10-11. Native host/workspace verified as `devonlys-MacBook-Pro.local`, `/Users/devonly/Developer/personal/NIZ`. This is a bounded read-only review of the frozen selectable-control hover fix and relevant actual-App regressions, continuing the completed theme/input/calibration reviews. No repository edits, Git operations, native GUI, HID, firmware, OS or Simulator actions were performed. Preferred LocalGPT/Claude tools remain unavailable; the authorized independent Codex route was used. Native visual verification remains parent-owned.

## Assessment

**No P0, P1, P2 or P3 finding in this bounded final scope.** The fix addresses text displacement and surrounding layout movement by preserving selectable-control frame allocation. It retains visual hover/selection feedback and existing action semantics.

- **Geometry is independent of hover:** `theme.rs:5-20` always requests `Button::selectable(...).frame_when_inactive(true)`. For an unselected choice, only inactive fill/weak-fill/stroke colors become transparent; the stroke width is retained (`theme.rs:11-16`). It does not change padding, font size, minimum dimensions, stroke width or parent layout according to pointer state. The style mutation is inside `ui.scope`, so it cannot introduce full borders or transparent fills on unrelated controls. Unselected choices remain visually unframed when idle; selected, hovered and pressed choices retain their existing roles.
- **Feedback and readability remain intact:** The helper does not change foreground colors, selected state, hover/active/open visuals or disabled opacity. The existing Light/Dark palette and selection foreground therefore remain in force. Normal controls, keyboard keycaps, red execution actions and cards are unchanged. Actual rendered text/disabled-label/confirmation contrast tests still pass; the palette checks include ordinary text on card/root/hover/pressed surfaces.
- **All selectable paths use the same allocation rule:** Assignment modes and layers use the helper (`app.rs:174-185`); output options use `choice` (`app.rs:214-225`); input-reference layers use `choice_value` (`app.rs:524-527`); device choices use it with the original selection dispatch (`app.rs:615-642`); appearance options and task tabs use it (`app.rs:789-807`); physical position dropdown options use `choice` (`app.rs:913-929`); swap From/To chips use `choice` and the original `set_target` guards (`keyboard.rs:256-263`). A fresh source search found no direct `selectable_label`, `selectable_value` or `Button::selectable` path outside `theme::choice`.
- **Selection and actions keep their original meaning:** `theme.rs:23-34` updates a value and marks the response changed only on a click selecting a different value. Clicking an already selected value still returns the click, which preserves popup-closing behavior. Disabled controls remain disabled, keyboard activation remains a normal egui Button action, and hover/press without a completed click cannot edit state. Existing modal, popup-closing, busy, input-capture and calibration ownership guards remain unchanged at the call sites.
- **User-visible fit remains valid:** The actual-App regression compares text rectangles, glyph-mesh bounds, previously painted control bounds and surrounding labels through hover dwell, pointer-down and pointer exit. It covers both themes, 960×820/700×580 and native pixel scales 1/2 (`app_tests.rs:142-329`). It includes selectable modes/tabs/layers, ordinary buttons, the appearance combo, a keycap and appearance-menu options. State assertions protect draft/baseline/selection/readiness/history/generation/target/pending state. The existing default no-wheel, all-66-key, minimum reachability and actual row-centering tests also pass after these call-site replacements.

## Fresh verification and exact scope

I independently ran `cargo test -p niz-gui --lib` after the freeze in the verified native workspace. It completed with exit code 0: **39 passed, 0 failed**, in 49.74 seconds. This includes the eight theme/size/pixel-scale combinations in `hover_and_press_keep_choice_glyphs_controls_and_neighbors_fixed` and the actual response semantics test (`app_tests.rs:333-384`). The tests use the in-process demo backend, not physical hardware. I inspected the assertions rather than relying only on the implementation report.

I independently hashed all six source files and the debug binary. All seven match `/private/tmp/niz-ui-reference-20261011/hover-jitter-source-sha256.json`:

| File | SHA-256 |
| --- | --- |
| `crates/niz-gui/src/app.rs` | `ccbc87e81c032640068a348c3a82c7f384f17dfae2147150e01c3c34dc2ae006` |
| `crates/niz-gui/src/keyboard.rs` | `cf2bb292ee861daaad5a7a27361cc31ea47e0238be6e03891e89683f1a8b2f3b` |
| `crates/niz-gui/src/app_tests.rs` | `ddd789a5990044429f57dedeb49fdac1a46d462df355fee9566426f30f877fe4` |
| `crates/niz-gui/src/theme.rs` | `9838343720683b60dd279b1a66eea6ebaf087c1a62efa75167c1e38d832e8ac6` |
| `crates/niz-gui/src/input_test.rs` | `5b0f21003a492214df2c631fc793b28a6825e767053ca532e35e0165833fd4b5` |
| `crates/niz-gui/src/lib.rs` | `e8f574c5ab061c2da2189acf38225e0d9eee07b69bae809d023dcd06e5fb02c3` |
| `target/debug/niz-gui` | `fc4b6392918d31c1494b706cca9396a1e0f3bd6b47cff07f8c0db0a6c40d01e0` |

This is source/headless-render evidence for the known hover symptom and retained UX requirements. It does not claim native screenshot verification, every possible OS/font/scale combination, final release packaging or physical keyboard calibration. Those remain outside this review's scope.
