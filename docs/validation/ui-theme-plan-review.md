# NIZ GUI Light/Dark/System appearance: bounded independent plan review

Host and native workspace verified: devonlys-MacBook-Pro.local, /Users/devonly/Developer/personal/NIZ. Reviewed current GUI source and the locally installed, pinned egui/eframe 0.36.2 API. Read-only plan review; no implementation, native UI, OS appearance, Git, HID, or Simulator operations.

## Conclusion

The proposed appearance-only change is feasible and proportionate. Keep the approved geometry and flows; add a compact Japanese appearance dropdown (System/Light/Dark), default System, install an independent readable palette/style for each effective theme, and leave protocols, qualified operations, draft data and history unchanged. No persistence, new dependency, or additional reference-app research is necessary.

The following concrete constraints should be included before implementation.

## Material findings

### P2: Active-widget foreground is also the strong-label foreground in egui

Current app.rs:62-77 sets `widgets.active.fg_stroke.color = WHITE` to fix active controls. The pinned egui implementation shows that `Visuals::strong_text_color()` returns `widgets.active.text_color()` (style.rs:1146-1148), and `RichText::strong()` uses that color directly (widget_text.rs:481-487). Consequently `ui.strong` text on a white card can become white-on-white. A token pair named "active foreground/background" alone does not detect this because strong labels do not use the active background.

Design ordinary strong/card text independently from on-accent text. One simple option is to keep active/default foreground readable against ordinary surfaces, use `selection.stroke.color` for selected tabs/chips, and explicit on-accent text for custom primary buttons; give ordinary pressed widgets a surface on which their foreground remains readable. Alternatively explicitly apply the normal palette foreground to every strong label. Test actual rendered strong labels against their card/modal/root surfaces in both themes, not merely the selected-tab pair.

### P2: Header popup closing must participate in the existing board popup guard

Current assignment captures `Popup::is_any_open` at app.rs:179-180, after the header has already been drawn. A new appearance popup is in that header. If it closes there before assignment begins, the board's local `popup_was_open` cannot represent ownership at the start of the frame. The closing pointer event must not select an underlying board key or change swap targeting.

Capture popup ownership before rendering header/device/appearance controls and combine that with the existing board guard. An actual-App regression should open the appearance popup, click a board key outside it, verify that the first click only dismisses the popup, and verify that a second click selects the key. Explicit theme selection itself must preserve selection, raw draft, Undo/Redo history, attachment, and calibration state.

### P2: Contrast must account for egui weak/disabled opacity and actual surfaces

The palette must cover root/card/window/popup/menu/TextEdit/keycap surfaces, ordinary/strong/secondary text, primary and selected control text, From/To/dirty/warning roles, hovered/pressed controls, focus indicators, and disabled controls. Theme-independent hardcoded light fills or dark warning colors in app.rs and keyboard.rs must be replaced consistently.

egui's default `weak_text_alpha` and `disabled_alpha=0.5` affect rendered colors. `Ui::disable` multiplies painter opacity (ui.rs:496-501); nested disabled scopes can compound opacity, which matters for the board because assignment and keyboard each have disabling scopes. Palette foreground-vs-background math alone can overstate actual legibility. Explicitly choose legible weak text and appropriate disabled treatment, and test composited color roles used by actual frames. Keep disabled state visually distinguishable as well as readable.

4.5:1 is appropriate for the application's normal-size readable text; 3:1 is appropriate for meaningful outlines/markers where those communicate state. Do not demand 3:1 from decorative white-card borders or other purely decorative separation. State must also have text/position cues, rather than relying only on color. Continue distinguishing From, To, dirty draft, active layer, and disabled controls.

## Verified native API and integration

- `Context::set_theme` accepts `ThemePreference`, including System (context.rs:2170). `ctx.theme()` is the effective Light/Dark choice (2158). `ctx.system_theme()` exposes the optional OS-reported theme (2152).
- `Options::begin_pass` refreshes the system theme from every RawInput (memory/mod.rs:356-359), and System resolves it or the configured fallback (363-367). Default fallback is Dark (331), so tests must deliberately provide `RawInput.system_theme` when testing system-follow behavior. Never assume a default headless RawInput means Light.
- `Context::set_style_of(Theme, style)` stores independent styles (2250). Build each from its corresponding `Theme::default_style()` or clone `style_of(theme)` before applying the shared typography/spacing and theme-specific visuals. `set_global_style` only replaces the currently active style; it is insufficient to configure both modes, and copying one Light style into both would corrupt Dark visuals.
- Keep `visuals.dark_mode` consistent with the installed theme. Populate window/popup/TextEdit styles as well as custom cards/keycaps; ordinary popups and modal contents inherit the appropriate active style.
- A UI captures a style when created. Changing Context's theme inside the header applies to subsequent UI/popups while an existing root UI may still hold the previous style for that frame. Request repaint on a preference change and use consistent per-frame palette/style sourcing; verify the settled next frame and avoid stale App-cached light colors. System theme changes must select the installed palette rather than reconstructing Editor/App/backend.
- Default `sync_window_theme` may synchronize this app's native window appearance with its preference; this does not alter the user's system-wide appearance. Do not call external commands to change macOS appearance.
- A small dedicated theme module/helper is reasonable if it prevents duplicated role colors and makes contrast verification use the real styling values. No broad rendering rewrite is warranted.
- Appearance selection may be harmless during pending/calibration, but it must respect modal blocking and must not dispatch an operation, modify controller qualification, reset consent, change phase, or steal the pointer-only workflow. Keep cancellation and phase instructions visible.
- Preserve the current header height/layout so the default viewport still fits all66 keys and the draft footer. At the minimum width, the new appearance control must wrap/truncate without moving any control beyond horizontal reach.

## Acceptance and verification

1. Actual-App user interaction selects Light and Dark through the header; settled frames have matching effective theme, ordinary/strong labels, cards, keycaps, selected tabs/chips, popups, and modal colors. Both themes preserve raw draft, selection, and history.
2. System preference follows RawInput theme changes in both directions; explicit Light/Dark ignore those changes. Re-selecting System resumes following the current input. No hidden persisted preference is required.
3. Retain default/minimum frame tests in both modes, including all66 stable slots, swap selection/Add/Undo, popup closing click guard, busy and modal blocking, and active calibration/cancel visibility. Do not repeat backend tests solely because colors changed; keep existing suites passing.
4. Calculate luminance/contrast using actual role/style colors, with alpha composition where applicable. Include strong labels on ordinary card backgrounds, weak/secondary labels, selected-tab/primary-button text, keycap states, status/warning text, and meaningful state outlines/markers. Avoid exact-color constant assertions.
5. Parent native Computer Use verifies final Light and Dark screens, Japanese glyphs, disabled/hover/selected/focus appearance, modal/popup color consistency, and minimum resizing. Headless geometry and contrast calculations support this evidence but do not replace screenshots.

No material plan blocker remains after these constraints are adopted. No hardware capability should be inferred from appearance references, and the implementation must remain confined to GUI presentation/test files.
