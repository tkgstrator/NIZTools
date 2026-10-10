# Independent final integration/source review

Date: 2026-10-11. Verified host: `devonlys-MacBook-Pro.local`. Workspace: `/Users/devonly/Developer/personal/NIZ`.

Verdict: **no new actionable findings in the reviewed final source**. The prior P1 displayed-device/attached-device mismatch is resolved by independent checks at the controller, captured confirmation, and worker boundaries. The later ComboBox close fix, local Application Support backup default, and corrected `lipo` verification argument order are included in this review.

This is a read-only independent Codex review of actual files, not a claim that an empty/untracked Git diff was reviewed. Parent verified that the preferred LocalGPT and Claude MCP tools are absent and explicitly assigned this fallback. No repository source edits, Git mutation/commits, native HID calls, GUI launch/interaction, or Simulator resources were performed by this reviewer. Only this review report is written. Parent retains responsibility for actual visual verification, final rebuilt release artifacts, and human hardware qualification.

## Device selection and immutable confirmation

- `crates/niz-gui/src/controller.rs:142-176` resolves selection by concrete path. Selecting another path sets `live=false` while retaining the editor and history. List reordering does not move the selection; removal or descriptor replacement detaches eligibility instead of automatically attaching another device.
- `controller.rs:179-228` requires selected path, attached target, current descriptor, and live state to agree before a changing operation. A trial confirmation captures the target, backup path, baseline, desired records, operands, and generation. Approval rejects changed target, generation, draft, baseline records, or operands. Switching away and back still needs an explicit read.
- `controller.rs:233-271,343-408` advances the generation on explicit reads, attaches the target associated with that request, and reconnects retained drafts only when path, identity, firmware, and baseline records all match. Failed reads preserve the draft. Reading B while retaining A's draft remains detached until an explicit replacement read succeeds.
- `crates/niz-gui/src/worker.rs:66-125` separately rejects replayed operation numbers, stale generations, and missing/mismatched concrete targets for Trial/Begin/Held/Finish/Cancel. Even bypassing UI/controller eligibility cannot send a changing request to another attached device.
- `app.rs:430-469` displays the captured target path, identity, firmware, exact changes and backup path, then passes the captured approval to `approve_trial`; it does not reconstruct approval from whatever the UI currently selects.
- `app.rs:185-193,496-508,633-648` explicitly closes each ComboBox after an activated item, including the already selected item. Only popup item responses call `ui.close`; ordinary layer/mode selectors do not. This source fix was inspected and the GUI suite was rerun after it. Actual popup visual behavior remains the parent's Computer Use check.

Regression evidence: the 25 controller tests include A-to-B selection, reorder, removal, same-path changed identity, switching back, retain/discard on B, open approval followed by selection change, and a changed draft after confirmation. The worker suite verifies missing/foreign path/foreign identity and a mismatched calibration target cannot enter the backend.

## Assignment, opaque data, and native trial boundary

- Default `Selection` is Reassign; From is a stable layer-local slot and To is an output action. Swap uses two slots. Core regeneration remains atomic, preserves unedited opaque records, and moves entire understood raw records on swap. The GUI now explicitly calls labels current assignments and states that legend correspondence is unverified (`app.rs:181-200,618-648`); it no longer presents demo key-row names as verified physical legends.
- `editor.rs:175-237` requires the exact observed firmware/identity/three-group baseline, basic single-key operands, no Fn-bearing columns on any layer, matching profile metadata, and byte-exact equality to regeneration from that baseline. `worker.rs:127-150` independently evaluates this gate using its own baseline; `NativeBackend` passes only bounded operands into native `Session` trial APIs.
- Native `LiveBaseline` is private, session/generation bound, not a file-created token (`niz-device/src/session.rs:11-19,326-339`). `trial_single_key` and `trial_swap` independently apply exact target/basic/Fn-column gates (`session.rs:345-396,452-486`). Ordinary qualification remains empty.
- Changing transactions freshly compare F9/F2 with the baseline, read lighting, durably publish/reopen settings, lighting and manifest before F1, send the existing raw reports once, then byte-compare settings and lighting (`session.rs:537-655`). Failure does not retry or perform an assumed rollback. Existing unfamiliar codes and padding cannot be changed through an external imported draft.
- NoChange returns an explicit outcome, configuration-only backup, and no hardware-write/postflight claim. Controller and CLI format that outcome distinctly. The GUI disables applying an actually clean draft.

## Calibration and trigger guidance

- The A panel clearly separates the manual Right Fn+7 trigger action from analog calibration, states that the current stage cannot be queried, and provides ordinary local keyboard text entry rather than a fabricated sensor reading (`app.rs:266-283`). `trigger_guide` verifies only a live exact target and the existing right-Fn-layer action149 at slot8; custom/missing/unverified mappings are labeled accordingly. No threshold/mm value is inferred or USB setter invented.
- README states the same limits, manual two-stage/power-off behavior, lack of USB setter/getter proof, unqualified hardware analog result/persistence, and absence of analog rollback. It distinguishes raw readback verification from physical output/persistence qualification. CLI examples/options and zero-based vs GUI one-based positions match the inspected CLI definition.
- All three mutating calibration stages require primary pointer activation in App and `Activation::Pointer` in the controller. Instructions say release all keys, physically hold A, then release all keys; no key ID/mask or synthesized keyboard event is sent.
- Active calibration owns the worker/session through user waits. Pending cancellation sets the shared flag and waits; cancellation cleanup is queued only at a safe waiting stage, terminal failure does not cause another unlock, and close waits for the owned worker and cleanup (`controller.rs:82-96,277-336,438-449`; `worker.rs:66-176,394-443`).
- `niz-device/src/calibration.rs:142-221,268-446` preserves the original failure and cleanup result, records unlock Sent versus Uncertain rather than asserting typing recovery, prevents duplicate unlock, backs up before locking, uses quiet-input guards before mutating stages, and preserves full trace/raw error evidence. Native preparation during both Preparing and Verifying uses quiet-input checking rather than silently draining (`session.rs:174-184`). The cancellation flag does not claim to cancel an already pending native physical send.
- Native and demo backends remain separate branches. Demo does not enumerate/open HID; simulation is visibly labeled. Demo success is not evidence of device calibration or RGB preservation.

### Observed filesystem limitation and default-path correction

The parent observed a pending demo transaction on this host and retained `/private/tmp/niz-implementation-20261010/demo-pending-sample.txt`. I independently inspected lines1497-1507: the owned demo worker is in `save_backup -> commit_backup -> commit_bytes -> NativeBackup.sync_directory -> std::fs::File::open -> open`. The report establishes a blocked native filesystem call in the previous Documents backup directory; it does not establish its cause, a TCC diagnosis, or physical HID activity.

The final `app.rs:59-62` now defaults to the user's `Library/Application Support/NIZTools/Backups` directory. This bounded location change preserves the durable snapshot gates and makes no automatic retry. Arbitrary custom folders can still block in native filesystem I/O; cancellation/close waits for ownership and cannot claim to interrupt a pending syscall. Both native trial and calibration finish all backup checks before the first changing command. The source review is clear with this limitation disclosed; the parent must verify an actual demo backup completes in the new local directory.

## Actual RGB marker, font, and packaging

- `niz-device/src/session.rs:673-675` accepts the exact native `[E6;64]` terminator as well as the previously observed prefixed marker. This applies to counted read-only lighting/counter streams; exact byte totals and chunk framing are still checked. The new golden fixture test replays the retained native RGB capture and compares its exact 198 bytes, alongside marker/count/fault tests.
- Independently parsed the actual embedded SFNT without relying on the metadata declaration: no `fvar` table, OS/2 weight400, family `Noto Sans JP`, subfamily `Regular`, full name `Noto Sans JP Regular`, PostScript name `NotoSansJP-Regular`. Actual SHA256 matches `assets/fonts/provenance.json`. OFL reserved name `Source` is absent from these primary names. The OFL and reference MIT license are copied into the bundle.
- `scripts/build-macos.sh` builds both ARM64 and x86_64 with a default macOS13 deployment target, combines both binaries, uses native `/usr/bin/strip -S` before ad hoc signing, verifies app and CLI signatures, verifies both architectures using corrected `lipo FILE -verify_arch arm64 x86_64`, validates the plist, and archives the app. Shell syntax passed.
- Independently checked the then-current generated app and CLI with the corrected lipo command and strict codesign verification. The app has both Mach-O slices, and both app slices declare `minos 13.0`; plist validation passed, and the two bundled license files/notices are present. These artifacts were generated before the final ComboBox rebuild; parent must verify the final rebuilt artifacts/ZIP before delivery. No Intel physical HID or Developer ID/notarization claim is made.

## Independently executed checks

1. `cargo test --locked --offline --workspace --all-targets`: **189 tests passed**, zero failed/ignored. Breakdown: 4 CLI unit, 10 CLI integration, 47 core, 82 device, 25 controller, 12 editor, 9 worker. Empty lib/main suites contain zero tests.
2. After the final ComboBox close changes, and again after the backup-default change: `cargo test --locked --offline -p niz-gui --all-targets`: **46 GUI tests passed**, zero failed/ignored.
3. `cargo fmt --all -- --check`: passed before and after the final ComboBox/backup-default changes.
4. `cargo clippy --locked --offline --workspace --all-targets -- -D warnings`: passed, including a fresh run after the final ComboBox changes. A scoped GUI clippy run also passed after the backup-default change.
5. `bash -n scripts/build-macos.sh`: passed. Corrected lipo architecture checks, strict app/CLI codesign checks, plist lint and app minos inspection: passed on the inspected generated artifacts.

These are software/offline/source checks. They do not qualify physical key output, USB changing commands, actual calibration response/timing, typing recovery, analog persistence, Bluetooth, or Intel hardware behavior. The absence of protocol transaction IDs still permits an indistinguishable same-opcode response arriving after the last quiet check; no code/report claims to solve that firmware limitation.

## Source snapshot SHA256

```text
crates/niz-gui/src/app.rs 2dbafcc504ca1d21c61d95faa050c30ca0f05550f4ccb6263b778df9b524becc
crates/niz-gui/src/controller.rs be907fc218256238d01da5009447e9528e755f7594af4717300316a74330bc22
crates/niz-gui/src/editor.rs b1fb6974f90ad2c1bdbf2c021da9cc55aeb2354f284c380f900bd0b8b956e3a6
crates/niz-gui/src/protocol.rs c30a2eaf5611846c9df26a2cb1c8e04a1efc9657724f66da416a6377b22ea493
crates/niz-gui/src/worker.rs 99d5719fee2a1140bcb276f46cfeda6cd434f19001d878531fe43e701422cff5
crates/niz-device/src/calibration.rs 5c7eb1f3a1ddc2ac2cc8c13cb114111f625add3884e3e97c4c0ac61634dfb577
crates/niz-device/src/session.rs 1168f2cc7f28709618ad90e7251a3484466e28a4b3e15f650893515291d24fb6
scripts/build-macos.sh bd6c4e25fbab424479fcf9bd3ed0ae0169d7c243865d609153357f66f4338bb9
README.md e7a4eb7b78c73b87c0b6cce1aecc8d141f3ccc86ec319a75b30c429c36437b14
assets/fonts/NotoSansJP.ttf da9481f60fb614f5213947bd9a7485a824c4bc58d219f58ee8d4c6e7877d4d56
```
