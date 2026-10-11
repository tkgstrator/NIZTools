# Official held-key calibration validation

Host: `devonlys-MacBook-Pro.local`, workspace `/Users/devonly/Developer/personal/NIZ`. Stock target: USB 66EC-XRGB, 0483:502a, usage 008c:1, `66EC(XRGB)BLe;V1.0.48;V1.0;`. No Simulator is used.

## Accepted scope and independent planning

The user explicitly accepted the official held-key calibration with the possibility that other keys change, requiring before/after comparison of all 66 normal thresholds. This replaces the earlier absolute unselected-key preservation requirement. Arbitrary numeric setters and modified firmware remain outside scope.

[Client plan review](held-key-client-plan-review.md) and [firmware plan review](held-key-firmware-plan-review.md) independently inspected local files and the pinned private image. LocalGPT/Claude generation tools were unavailable; authorized separate Codex reviewers were used. They performed no hardware commands.

The reviewers differed on post-read ordering. The coordinator chose complete DC mode 1 capture **after DE while D9 output remains locked**, followed by one D9 unlock and existing keymap/RGB consistency checks. The firmware review confirmed DC saves/restores prior scan-enable, so a successful DC read preserves the lock. This avoids ordinary key/mode input during threshold comparison. Public manual-reader ownership guards remain intact.

## Operation and evidence boundaries

Select the physical position in the lower keyboard, prepare with every key released, hold only that key, and click the red Execute button with the mouse. Preparation retains existing release calibration and sends no DB. Selection is frozen until completion/cancellation. Held-only GUI execution automatically compares and finishes after one DD/DE sequence.

Before and after snapshots are complete 66-value **normal scanner R** observations, not stored T/B equality, all scan modes, power-cycle persistence, a physical key mask, or proof that a weak key has improved. Neighbor changes are factual attention results; no change is a valid observation. Partial/missing after capture is unavailable comparison. Analog observations are retained if a later keymap/RGB verification fails.

The keymap/RGB backup cannot restore analog calibration. No D5/D8, analog reset, firmware modification, guessed setter or automatic DD retry is used.

## Verification record

Automated, independent source review, native GUI and distribution evidence are recorded below after the frozen implementation checks. Native DEMO verification exercises the separate simulation backend and does not open HID. A physical held-key calibration requires an actual switch to be held; generated OS input is not a substitute.


### Automated checks

The coordinator ran `cargo test --locked --workspace --all-targets` on the integrated release source: **319 passed, zero failures or ignored cases** (core53, device109, CLI15, GUI142). Final log: [held-key-test.log](held-key-test.log). After the final single-element test-loop lint cleanup, the affected external held contract suite was rerun:3 passed. No behavior changed in this cleanup.

`cargo fmt --all -- --check`, `git diff --check`, and `cargo clippy --locked --workspace --all-targets -- -D warnings` passed. Clippy initially found three style issues and one single-element test loop; they were corrected, and the final clean log is [held-key-clippy.log](held-key-clippy.log). No failure is being treated as success.

Device tests use the real Session and strict transport boundary to require pre-DC before locking, no DB, one DD/DE, post-DC while locked, then one unlock. Ten held cases cover every physical position, target/neighbor/no-change snapshots, unknown delivery, late/malformed/missing replies, excess/partial rows, cancellation and later verification failures. GUI tests use real rendered App/controller/worker routes, including primary mouse versus focused keyboard/AccessKit actions, source/ownership guards, all66 literal result pairs, default-size Light/Dark fit, hover geometry and disclosure alignment. [Device report](held-key-device-implementation.md), [GUI report](held-key-gui-implementation.md).

### Independent implementation review

[Actual source review](held-key-source-review.md) found one P2: completing calibration on A then selecting another device could leave A's measurement visible under the new source. The controller now invalidates the result on device switch, exact descriptor replacement/removal, and new read generation. Four rendered regressions were observed failing before the fix and passing afterward; a fifth verifies read-generation invalidation. [Focused rereview](held-key-source-rereview.md) confirms resolution and no new material finding. The coordinator additionally corrected only lint/style issues after review.

### Native GUI and cleanup

CUA opened an owned native DEMO bundle with a fixed `--demo` launcher and independently confirmed the process argument. Its backend displays that it does not connect to a keyboard. Native screenshots/AX confirmed both Dark and Light, A/#31 and Esc/#01 selection, complete66-key board, centered disclosure/controls, and main content within the default window. Screenshots remain in the task's CUA output.

CUA's coordinate mouse action failed with `noWindowsAvailable`, including after raising the window. Accessibility actions work for reading, tabs, selection and themes; they intentionally cannot run the primary-pointer-only calibration actions. Therefore **native DEMO prepare/execute/cancel completion was not verified**. Those execution paths were covered by rendered interaction and strict synthetic transport tests. No native execution success or physical hardware result is claimed.

Both owned DEMO processes were closed via CUA; native process inspection showed neither remained. The first launcher had exec'ed outside its bundle, so it was replaced by an internal payload for coherent app ownership. The final release app was opened separately and successfully read the actual66EC-XRGB settings via CUA. It is left at Input Threshold, A/#31 selected, before preparation. No D9/DB/DD/D5/D8 or persistent configuration/RGB/firmware write was sent to real hardware during this implementation/verification run.

### Distribution

The final `scripts/build-macos.sh` completed after all production changes. Both app and CLI contain arm64+x86_64 slices; strict ad hoc signature and plist checks pass. Arm64 and Rosetta x86_64 CLI help both launched successfully. The ZIP has no corrupt member, its GUI executable hash equals the signed local app, and the extracted app passes strict codesign and both-architecture checks. [Release hashes](release-sha256.json), [source hashes](held-key-source-sha256.json), [build log](held-key-universal-build.log).

The build has no Developer ID notarization. Intel hardware HID, Bluetooth, physically-held calibration, response improvement and power-cycle persistence remain unverified. Standard firmware and the accepted conditional comparison scope are preserved.
