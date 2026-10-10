# Final validation — 2026-10-11

Host: devonlys-MacBook-Pro.local, Apple Silicon macOS26.4. Workspace: /Users/devonly/Developer/personal/NIZ. Rust1.98.1; declared workspace minimum1.95. No Simulator was used.

## Automated and independent source checks

Parent ran the final frozen source with `cargo test --locked --workspace --all-targets`:189 tests passed, zero failures or ignored cases. Breakdown:47 core,82 device,14 CLI,46 GUI. `cargo fmt --all -- --check` and `cargo clippy --locked --workspace --all-targets -- -D warnings` both passed. Logs are saved beside this report.

The [independent final source review](final-source-review.md) found no remaining material finding. It rechecked target selection/attachment, immutable apply approval, worker-side target validation, calibration ownership and cleanup, the actual RGB end marker, manual trigger guidance and packaging. LocalGPT and Claude Code MCP were not callable in this environment; the authorized independent Codex fallback was used. The reviewer performed its own189-test run, then reran46 GUI tests after the final UI fixes.

The earlier P1 target mismatch is fixed: selection changes detach live eligibility, enumeration preserves paths rather than indices, and each changing request and approval carries the attached concrete descriptor. Draft/history remain available. Regression tests include reorder, removal, target changes, descriptor changes at the same path and changed approvals.

## Parent Computer Use verification

All GUI interaction and screenshots used CUA. A disposable app wrapper always supplied `--demo`; the separate demo backend never enumerates or opens native HID. The previous unbundled owned demo and both disposable app processes were closed or stopped; final native process inspection found none remaining. Pre-existing test GUI processes were recorded as empty. Screenshots and accessibility observations remain in this task's tool record.

Observed improvements and checks:

- Original variable Japanese font had default weight100. A static weight400 Regular instance is now embedded; metadata and hashes are in `assets/fonts/provenance.json`. Parent compared before/after screenshots and confirmed readable Japanese glyphs.
- Primary From/To controls are grouped, details/backups collapsed, clean drafts cannot Apply, and unchanged selections cannot Add.
- Output and position selection menus close after accessibility activation. Parent repeated output selection after the explicit close fix and confirmed the popup disappeared.
- One-way reassignment was previewed, added, confirmed with the concrete target/path and simulated through completion. The resulting local configuration backup was retained under the task's temporary evidence folder.
- Optional swap showed both changed positions and a2-record draft; Undo restored a clean draft.
- The A adjustment view displays Right Fn+7 manual guidance, unknown selected stage and an input check. A demo profile without the captured action149 is correctly labeled as an unconfirmed/custom shortcut.
- Mouse pointer clicks completed the simulated released → held → finish sequence. During calibration the device selector/read, backup controls and typing field were disabled. After completion a fresh Read was required before another device operation. No OS-generated key press was used as a physical sensor-calibration substitute.

The original Documents backup location stalled inside native directory opening during durable synchronization. A separate listing and owned-folder rename also stalled; the underlying filesystem cause is unconfirmed. The blocked demo process was stopped after verifying that it was demo-only. The default is now `~/Library/Application Support/NIZTools/Backups`. Parent repeated demo Apply there and observed completion. File synchronization/reopen gates remain intact. Arbitrary custom paths can still block native file I/O; no changing HID command is sent until the durable backup gate returns.

The successful Application Support test snapshot was moved into temporary evidence and its owned empty Backups directory was removed. The interrupted test folder at `~/Documents/NIZTools Backups` could not be moved within a bounded cleanup attempt and remains; its cleanup is not claimed successful. No pre-existing user backup was modified.

## Distribution checks

`scripts/build-macos.sh` completed successfully after correcting the native `lipo FILE -verify_arch ...` argument order. Cargo release stripping is disabled; `/usr/bin/strip -S` strips the Mach-O outputs before signing, avoiding this installation's broken rust-objcopy library lookup.

Both `dist/NIZ Utility.app/Contents/MacOS/niz-gui` and `dist/niz` contain arm64 and x86_64 slices. Strict ad hoc codesign verification and plist lint pass. Both app slices record minimum macOS13.0. Dynamic dependencies are macOS system libraries/frameworks. `dist/NIZ-Utility-universal.zip` was generated; hashes are in [release-sha256.json](release-sha256.json).

The Universal2 CLI successfully printed help when explicitly run as arm64 and x86_64 through installed Rosetta. This verifies CLI startup, not Intel physical HID operation. Developer ID signing and notarization are not included.

## Hardware evidence boundaries

Native read-only work identified the exact XRGB V1.0.48 target, parsed198 configuration records, captured198 lighting bytes and preserved unknown assignments. See [hardware read evidence](2026-10-10-read-only-hardware.md).

No persistent configuration, lighting, calibration or firmware command was sent to the real keyboard during parent or delegated verification. Physical remap/swap output, power-cycle persistence, analog calibration outcome, Intel hardware HID and Bluetooth remain unverified. Ordinary qualification is intentionally empty; GUI trial/calibration are explicitly experimental and exact-target bounded. Threshold switching is the existing physical firmware shortcut, with no invented USB setter/current-stage getter.
