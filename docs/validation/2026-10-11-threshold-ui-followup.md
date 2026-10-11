# Threshold UI copy and disclosure alignment — 2026-10-11

Verified host: `devonlys-MacBook-Pro.local`, Apple Silicon macOS 26.4.
Workspace: `/Users/devonly/Developer/personal/NIZ`.
Base commit: `d54a4bfefed2dce8d7a6a7b9854c529b8e641910`.

## Delivered changes

The InputThreshold card now shows the selected physical key, unsupported-write status, decimal value and explicit read button. Hexadecimal, internal-unit and snapshot explanations are initially collapsed under `しきい値について`. The short visible read notice still discloses input pausing and compensation returning to its configured default. Simulation retains its visible `DEMO値` badge and detailed provenance. Source/pending/modal/popup/calibration/close guards and mouse, keyboard and accessibility read activation remain intact. Selecting a key does not initiate communication.

The reported arrow/title misalignment reproduced in actual rendered shapes before the fix: the visible Japanese glyph center was 1.00 physical pixel below the arrow at scale 1 and 1.50 pixels below it at scale 2. The logical font line box differs from the visible NotoJP glyph bounds. The bounded production fix derives the title's pixel-snapped visible glyph center and translates only the icon painter's response copy. It retains egui's default triangle, animation, color and interaction. Title, header identity, allocation and hit area are unchanged.

The regression tessellates the actual title Text shape and compares it with the actual triangle Path, rather than duplicating the callback's arithmetic. Light/Dark × scales 1/2 × closed/open/closed again gives **0.00 physical pixels of center difference in all 12 final cases**. [RED](threshold-ui-alignment-red.log), [GREEN](threshold-ui-alignment-green.log), and [independent focused test](threshold-ui-alignment-independent-tests.log) logs are retained. Animation endpoints are measured; intermediate animation and arbitrary display scales are not separately quantified.

## Coordinator verification

| Command/check | Observed result |
|---|---|
| `cargo test --locked --workspace --all-targets` | **294 passed, zero failures, exit 0** |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | exit 0 |
| `./scripts/build-macos.sh` after the alignment fix | exit 0; arm64+x86_64 GUI/CLI, strict ad hoc signature checks, plist lint |
| arm64 and Rosetta x86_64 CLI `--help` | both exit 0, `raw-thresholds` present |
| ZIP integrity and packaged executable hash | pass; ZIP executable equals the signed distribution app |

Counts are Core 53, Device 99, CLI 15 and GUI 127. GUI includes 52 library, 27 controller, 12 editor, 17 keyboard, 6 raw-controller, 4 raw-worker and 9 worker tests. No additional tests are counted for zero-test targets. Logs: [workspace tests](threshold-ui-test.log), [format](threshold-ui-fmt.log), [Clippy](threshold-ui-clippy.log), [build](threshold-ui-universal-build.log), [arm64 help](threshold-ui-cli-arm64-help.log), [x86_64 help](threshold-ui-cli-x86_64-help.log), [ZIP](threshold-ui-zip-validation.log).

The [current source/artifact manifest](threshold-ui-source-sha256.json) records 41 source/build files and all three distribution artifacts. All 17 GUI source/test files matched the final status reviewer's current manifest; that reviewer verified the later one-string status change against the prior alignment freeze. The [release hashes](release-sha256.json) identify the current generated distribution; binaries remain ignored. Previous reports and manifests retain their historical counts and hashes.

## Independent review

The [copy review](threshold-ui-copy-review.md) and [alignment review](threshold-ui-alignment-review.md) inspected their respective actual diffs and found no unresolved material findings. The final alignment reviewer independently passed 11 affected GUI tests and confirmed all 12 measured centers. The later [final status review](threshold-ui-final-status-review.md) verified exactly one production string and three test expectations after the user chose stock firmware. It independently passed 11 affected tests plus the capability-status test, with all 17 current source hashes stable. [Status RED](threshold-ui-status-red.log) and [GREEN](threshold-ui-status-green.log) are retained. The coordinator inspected the actual diff and full workspace output, reran the full 294-test suite, and rebuilt after this final copy change.

LocalGPT and Claude Code MCP model/generation tools were not callable. Authorized independent Codex workers were used as the fallback; no LocalGPT or Claude review is claimed. No reviewer performed native UI, hardware operations or Git mutations.

## Native distribution appearance

Using CUA, the coordinator closed its owned preceding app with Cmd-Q and verified the process exited before replacing the distribution. The signed alignment build was opened from `dist/NIZ Utility.app`. On InputThreshold, native screenshots showed the shortened copy and centrally aligned arrow/title in both Light and System Dark, with details closed and expanded. Closing details returned to the normal view. After the final status copy change, the coordinator again closed the owned app, verified exit, rebuilt and reopened it; the final System Dark screenshot confirmed the shorter unsupported status and unchanged aligned header. All 66 keys and both collapsed footers were visible without clipping or a main scrollbar in this unloaded-baseline state.

Native verification of the final app did not initiate a raw read or device-setting write; the reader was disabled while the baseline was unloaded. Existing [hardware raw-read evidence](2026-10-11-raw-thresholds-hardware.md) remains a separate earlier qualification. The preceding app had displayed A 23, but that is not a new final-artifact hardware qualification. Native stationary hover was not newly sampled; headless geometry and source review remain the evidence for hover stability.

The updated app is left open on InputThreshold, A selected, System appearance, details closed and baseline not loaded. No calibration, reset, global toggle, assignment Apply, lighting or firmware operation was performed in this follow-up. No Simulator resources were created or booted. No owned test worker or old app process remains.

## Individual-write feasibility remains separate

Neither a per-cell numeric threshold setter nor a full analog-table upload endpoint was found in the exact stock V1.0.48 image or inspected vendor tools. The user's host read-modify-one-cell/full-upload idea is valid conditional on a real receiving endpoint. The stock keymap and RGB table writers address different data; the updater is a firmware update path. D5 clears all per-key calibration overrides and regenerates defaults for the current global mode, but it is not a qualified single-key adjustment or full factory restoration.

The [firmware write-path review](threshold-write-firmware-review.md), [vendor/update-path review](threshold-write-vendor-review.md) and [concrete feasibility proposal](../reverse-engineering/2026-10-11-threshold-write-feasibility.md) record the scope and next gates. Plan findings concerning crash-safe whole-table persistence, preservation of other-65 stored T and active R, and raw units/rounding/sentinel handling were resolved in the proposal. [Firmware rereview](threshold-write-firmware-plan-rereview.md) describes the checkpoint before the later full-table paragraph; [vendor final rereview](threshold-write-vendor-plan-rereview.md) covers the proposal checkpoint hash `3b950b63cfe8a4e474d232b0919d36b603c61d4605ee2f7e171445b14d402e37`, before the user decision was appended.

This is reviewed feasibility framing, not an implemented modified firmware, API or preservation proof. A threshold slider, all-key reset control, physical A improvement and modified-firmware flash/recovery remain unfinished or unqualified. The user selected keeping the stock firmware. The proposal remains a historical alternative; modified-firmware design, implementation and flashing will not proceed. The visible unsupported status no longer says analysis is in progress. No malformed command or speculative write was sent to the board. No proprietary image, executable, DLL or binary disassembly is included in these changes.
