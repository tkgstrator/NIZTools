# GUI independent review P2 fix

Scope: only `crates/niz-gui/src/sensitivity_ui.rs` on local `/Users/devonly/Developer/personal/NIZ`.

Added `.on_hover_text("キーを離して適用してください。入力が一時停止します。")` to the actual red Apply button Response. Its `.clicked_by(Primary)` behavior is retained. No always-visible explanation or other product behavior was added.

TDD evidence:
- `gui-review-tooltip-red.log`: actual rendered hover test compiled and failed before the tooltip existed because the requested instruction was absent.
- `gui-review-tooltip-green.log`: `cargo test -p niz-gui --lib sensitivity_ui` passes all five tests, zero failures/warnings.
- New test hovers the real Apply button in both Light/Dark themes, observes the rendered instruction, checks Apply/endpoints stay in identical positions, and proves coefficient/generation/live state and pending/sensitivity remain unchanged.
- Existing primary vs focused keyboard/AccessKit/secondary-click test still passes.

No HID operations, native app launch, Simulator, Git, or additional features. Root owns full workspace tests/build after device P1 changes. Module frozen for integration review.
