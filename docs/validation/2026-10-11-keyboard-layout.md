# Clickable keyboard layout validation — 2026-10-11

Host: `devonlys-MacBook-Pro.local`, Apple Silicon macOS26.4. Workspace: `/Users/devonly/Developer/personal/NIZ`. Base commit: `bcc272fbe1706914666a1714838a78148d21ca01`. No Simulator was used.

## Scope and layout evidence

This enhancement changes only the GUI and its tests. The 66-button layout sits below the From/To settings and above Undo, change details and Apply. Five rows contain15/14/13/13/11 immutable slots, each spanning15 width units. Labels show the current draft assignment and fixed1-based slot number; official physical legends are separate selected details/tooltips. Selection is green for From, gold for swap To; an orange dot compares the complete active-layer raw record with the baseline.

The [primary layout investigation](keyboard-layout-evidence.md) documents manufacturer manual/resource geometry and a bounded main-view click→selected ID→F2 index chain. [Main-view records](keyboard-main-view-records.json) contain66 factual view IDs/rectangles. The parent independently checked immediate ID/coordinate writes for all66 records against the supplied PE bytes, including A31, LeftFn57, RightFn61 and RightAlt62. The host software preserves those indices rather than deriving positions from assigned outputs. Proprietary executables, DLLs, manual images and full disassembly are not published.

This is static evidence for the official host software's template, not a physical key-scan test of attached firmware. Captured slot61/62 assignments differ from their template legends; the GUI displays the captured current assignments without forcing factory defaults. Core layout constants, device protocols, target qualification and guarded Apply/calibration behavior are unchanged.

## Behavior and tests

Clicking a different From key or layer loads its current supported single output into To without changing raw records or history. Same-slot selection retains a pending output choice. Advanced, unknown, special and unassigned outputs are preserved until an explicit supported replacement is chosen. Import, Undo, Redo and applied-profile acceptance refresh settings only when the selected complete raw record changes; unrelated changes preserve the pending choice.

Swap mode starts with From targeting, advances to To after the first selection and leaves that target explicit. Two selections only choose operands; adding and applying changes remain separate actions. Board and target controls are disabled during owned operations, confirmations and popup ownership, including the popup's closing frame. Existing target/generation validation remains active.

Parent ran the final frozen source:

- `cargo test --locked --workspace --all-targets`: **208 passed**, zero failed/ignored;47 core,82 device,14 CLI,65 GUI. GUI has27 controller,12 editor,17 keyboard and9 worker tests. [Log](keyboard-layout-test.log).
- `cargo fmt --all -- --check`: passed. [Log](keyboard-layout-fmt.log).
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed. [Log](keyboard-layout-clippy.log).

The [independent plan review](keyboard-layout-plan-review.md) required evidence-backed positions, current-setting initialization and explicit swap targeting. The [actual source review](keyboard-layout-source-review.md) found stale selected-output readiness after import/history. The fix compares complete selected raw records during draft replacement. Five regressions and a separate independent assertion probe cover both directions and preservation of unrelated pending choices. The [rereview](keyboard-layout-source-rereview.md) found no unresolved material issue and independently ran all65 GUI tests.

LocalGPT and Claude Code MCP generation/reviewer tools were not callable in this environment. Authorized independent Codex workers performed the plan and source reviews; no unavailable MCP review is claimed.

## Parent native visual verification

All UI actions and screenshots used CUA. A disposable app wrapper always supplied `--demo`; this separate backend loads the sanitized captured target fixture and never constructs native HID. Parent observed:

- Clicking A31 selected From31, loaded current A into To and left the draft clean. Current assignment and official legend are distinct.
- Opening the To popup disabled board interaction.
- In swap mode, clicking A31 advanced targeting to To; clicking S32 highlighted both operands and previewed A→S and S→A. Adding the change updated the two button labels and orange markers without applying.
- Undo restored A/S labels, cleared markers and reported no unapplied changes; Redo became available.
- At the700×580 minimum window, vertical scrolling made all five rows/all66 keys visible without horizontal clipping. Long Ctrl/Cmd labels fit. The default window is960×820.
- Backspace uses visible ASCII `BS` rather than a missing-font glyph. Japanese labels and current outputs were readable.

After native resize, an older CUA binding temporarily returned stale screenshots/accessibility state. A fresh `cua.getApp` binding showed the actual scrolled state and completed Undo result. Read-only source inspection confirmed finite viewport/Frame/ScrollArea constraints; no application patch was made for that observation. Screenshots and AX evidence remain in this task's tool record.

Owned demo preview processes were closed after verification. The previously opened distribution app was inspected as clean, idle and read-only before closing for replacement. No real Apply, calibration, lighting or firmware operation was performed during layout verification.

After rebuilding, parent opened the actual distribution app and performed a fresh read of the attached `66EC-XRGB` at `DevSrvsID:4295010726`. The new board displayed all66 current assignments. Clicking A31 selected From31 and To A, with Add disabled and no unapplied changes. All five rows fit in the default960×820 viewport. This verifies integration with a native read; it is not a physical remap or calibration test. The owned release-verification process was then closed.

## Distribution

The final `scripts/build-macos.sh` run succeeded on the frozen reviewed source. Both GUI and CLI contain arm64 and x86_64 slices; strict ad hoc codesign verification and plist lint passed. ZIP integrity validation reported no errors. [Build log](keyboard-layout-universal-build.log) and updated [SHA256 hashes](release-sha256.json) are retained. The CLI hash is unchanged from the earlier validated release. Ad hoc signing is used; Developer ID signing, notarization, Intel physical HID and Bluetooth qualification remain outside this verification.
