# macOS UI and raw-threshold integration validation — 2026-10-11

Historical checkpoint: the later [threshold UI follow-up](2026-10-11-threshold-ui-followup.md) records the current 294-test sources and rebuilt distribution. Counts and source hashes below retain this earlier checkpoint.

Verified native host: `devonlys-MacBook-Pro.local`, Apple Silicon macOS 26.4.
Workspace: `/Users/devonly/Developer/personal/NIZ`.
Base commit: `f78d1617405e91387f03515df469d32c9e6ca5d4`.

## Delivered behavior and boundary

The existing 66-key layout remains below the controls. Assignment defaults to one-way reassignment, with an explicit two-position swap mode. System, Light and Dark appearance, stronger contrast, centered mixed label/control rows and uniform Body14 device/version text are integrated. Unselected selectable controls reserve the same frame width while hovered or pressed, preventing the reported label and neighbor shifts. Apply's execution and cancellation sit side by side; the execution button is red.

The InputTest view displays app-received text and highlights loaded-baseline output candidates while held. Draft assignments are not its matching source. It suppresses keyboard activation of other controls, clears held state on focus/view changes, limits text to256 characters, and does not persist or send test text. It cannot identify the input keyboard, measure analog travel or observe OS-reserved shortcuts.

InputThreshold uses immutable physical legends and positions, independently of current remapped outputs. **Selected-key-only threshold writes remain unsupported.** The global shortcut explanation and legacy calibration Begin/Held/Finish controls are absent. Selecting a key does not run calibration or a setter. Existing cancellation of a previously owned calibration remains explicit about whole-board scope; it is not a new GUI entry point to begin one.

The new manual `しきい値を読み込む` action shows a complete normal-scan R snapshot in decimal and hex for the selected physical key. It is qualified only for0483:502a /008c:1 and exact fresh firmware `66EC(XRGB)BLe;V1.0.48;V1.0;`. No numeric slider, mm conversion, continuous sensor stream or individual setter is invented. The visible helper discloses input pausing and active compensation returning to its configured default. Failed/partial/stale/source-mismatched reads publish no values and invalidate old cached data. DEMO values are generated through the parser and explicitly labeled simulation.

The read button accepts a deliberate mouse/accessibility click or focused Enter/Space. This corrects the original pointer-only callback, which excluded the macOS accessibility action. An unfocused Enter, physical-key selection, appearance change or InputTest never requests a read. Existing source, pending, closing, modal, popup and calibration guards remain in force. Destructive execution behavior was not changed by this accessibility correction.

## Final coordinator checks

The coordinator reran the following on the frozen final sources after the accessibility fix:

| Command | Observed result |
|---|---|
| `cargo test --locked --workspace --all-targets` | **292 passed, zero failed/ignored, exit0** |
| `cargo fmt --all -- --check` | exit0 |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | exit0 |
| `./scripts/build-macos.sh` | exit0; both architectures, strict ad hoc signatures, plist lint |
| arm64 CLI `--help` | exit0; includes `raw-thresholds` |
| x86_64 CLI `--help` under Rosetta | exit0; includes `raw-thresholds` |
| ZIP integrity and executable hash comparison | pass; packaged executable equals signed app |

Test totals are Core53, Device99, CLI15 and GUI125. The GUI has50 library tests,27 Controller,12 Editor,17 Keyboard,6 raw Controller,4 raw Worker and9 existing Worker tests. Logs: [tests](ui-final-test.log), [fmt](ui-final-fmt.log), [Clippy](ui-final-clippy.log), [Universal2 build](ui-final-universal-build.log), [arm64 CLI](ui-final-cli-arm64-help.log), [Rosetta CLI](ui-final-cli-x86_64-help.log), [ZIP](ui-final-zip-validation.log). Zero-test binary/doc targets are not counted as additional tests.

The [final manifest](ui-final-source-sha256.json) hashes41 source/build files and all three distribution artifacts. It was checked against the frozen reviewer manifests and rechecked before Git integration. [Release SHA-256](release-sha256.json) identifies the generated app executable, CLI and ZIP. Dist binaries are ignored and are not committed.

## Regressions and independent review

Rendered tests cover both themes, default960×820 and minimum700×580 viewports, and native scales1/2 for stationary hover geometry. First hover,120-frame dwell, press, exit and menu ownership retain text/glyph/control/neighbor positions. The diagnosed2-point growth came from egui's unselected selectable frame allocation; transparent inactive borders now retain the width. [Hover technical review](ui-hover-source-review.md) and [UX review](ui-hover-ux-review.md) retain the diagnosis and independent evidence.

The default collapsed-state matrix contains eleven states per theme,22 total, with zero main wheel-scroll range. All66 keys and principal actions fit the default viewport; minimum sizing and expanded details intentionally permit vertical scrolling without horizontal clipping. Mixed row centers, all eight device/version text rows, long device paths, disabled/popup contrast, red confirmation controls, source guards and InputTest input/focus semantics are exercised. The final raw snapshot adds default/minimum reachability and pending/error/hover checks.

Core/Device tests independently cover all66 physical mappings, every byte value, opaque report tails, strict eleven-row framing, malformed/partial/extra streams, incompatible fresh firmware, uncertain delivery, no retry, failure poisoning and real path-lock retention during bounded passive settling. A failing-then-passing review-fix cycle corrects post-F9 excess handling, DC failure ownership and compensation disclosure. The [Device rereview](raw-device-preprobe-rereview.md) found no remaining source blocker and independently passed sixteen raw Device tests before the first hardware probe.

The [GUI integration review](raw-gui-source-review.md) covers target/ticket/generation/baseline binding, simulation provenance, stale-cache clearing and diagnostic frame retention. Its122-test count and pointer-only callback describe the preceding checkpoint. The final [accessibility review](raw-gui-accessibility-review.md) independently confirms the one-line production delta and runs nine focused actual-App tests: focused Enter/Space and real generated AccessKit Button-node Click, plus offline/pending/close/modal/popup refusals and explicit unfocused negative behavior. All seventeen GUI sources/tests plus debug binary matched before/after review. Relevant RED/GREEN evidence is retained in `raw-gui-keyboard-red.log`, `raw-gui-accesskit-red.log` and `raw-gui-accessibility-green.log`.

LocalGPT and Claude Code MCP generation/reviewer tools were not callable. Independent authorized Codex workers performed plan/source reviews. Neither preferred service is claimed to have reviewed the work. The coordinator inspected actual files, full output, hashes, captures and native UI independently.

Older theme/input/calibration/hover reports retain their historical source hashes and test counts. They do not certify the final artifact by themselves. This record, the final logs/manifest and focused accessibility review identify the current integrated sources. Earlier calibration UI proposals were superseded by the exact firmware result and selected-key-only requirement.

## Native release verification

All native UI actions and screenshots used CUA. The coordinator closed its own previous GUI with Cmd-Q and verified the process exited, then opened the final signed distribution app without `--demo`. No native path lock was bypassed. A fresh baseline read populated the actual attached keyboard's settings.

On InputThreshold, the accessible read-button action produced the visible pending state and disabled the reader/board during ownership. Success displayed **A/#31 =23 (`0x17`)**. Selecting S/#32 without another reader action displayed **25 (`0x19`)**. Snapshot details showed actual target `DevSrvsID:4295010726`, exact firmware, `読み取り元: 接続中の実機`, eleven packets, connection generation1 and timestamp `1791701738.638`. That timestamp remained unchanged after S/A selection, Light/System appearance changes and InputTest navigation, corroborating no automatic refresh.

Native Dark and Light screenshots showed all five rows/all66 physical keys, raw value/read action and the collapsed footer in the default viewport, with no main scrollbar or clipping. The expanded device-info screenshot showed both target/source blocks and the firmware in the same ordinary body font. Expanding those details adds a vertical scrollbar as intended; closing them restored the collapsed default view.

After the GUI read, CUA-generated OS input `a` appeared in InputTest and did not cause another threshold read. The coordinator requested Clear before returning to InputThreshold, but subsequent AX navigation attempts stopped changing the UI and coordinate actions still returned `noWindowsAvailable`; the clear result was not independently confirmed. That one-character sample is transient app state. This confirms app OS-event integration; it is not a physical hardware keypress or a timing measurement of post-DC scanner recovery. A native held-key glow was not sampled; headless press/release/mapping tests cover that behavior. Native stationary-hover interaction was not claimed; source/rendered regression evidence remains the basis for the hover fix.

The final app is left open at InputThreshold with A23, System appearance and details collapsed. The loaded editor was clean and no Apply, calibration, global toggle, lighting write or firmware operation was performed during final verification. [Hardware capture validation](2026-10-11-raw-thresholds-hardware.md) separately records the two CLI reads, byte-identical received data and exact-image mapping comparison.

## Distribution and cleanup limits

The GUI and CLI contain arm64+x86_64 Mach-O slices, target macOS13.0+, and pass strict ad hoc signature verification. The ZIP executable matches the installed app. Developer ID signing/notarization, Intel-native physical HID, Bluetooth settings transport, physical remap persistence and selected-key analog writes remain unverified or unsupported; a successful build is not evidence for those features.

No Simulator resources were created or booted. Coordinator-owned temporary preview processes/windows from earlier checks were closed. Only the updated intended distribution GUI remains open; no separate CLI hardware Session or test worker remains running. Useful source/review/test/capture artifacts are retained. No proprietary executable, DLL, firmware image, manual bundle or disassembly binary was added to the repository or distribution.
