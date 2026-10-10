# SDD ledger — plan: docs/superpowers/plans/2026-10-10-niz-macos.md

Ruling: User approved the concrete design and proposed next steps, including read-only CLI implementation. Proceed with the authorized work without repeated authorization requests.

Ruling: The project has no Git repository or existing source. Work in the user-selected project directory with explicit file ownership. Do not initialize Git or create commits just for the workflow; keep a file-based ledger and review actual files.

Ruling: LocalGPT and Claude Code MCP remain absent from current tool discovery. Use authorized Codex fallback workers and reviewers on the verified host.

Ruling: Verified hidapi2.6.7 source exposes macos-shared-device, enabling nonexclusive device access. Enable it in the workspace dependency to avoid unnecessarily seizing the user's keyboard.

## Preflight interfaces

| Tasks | Shared interface or file | Assessment |
| --- | --- | --- |
| 1 | codec tests versus raw-preservation requirements | Golden fixtures, malformed records and 3→9 tests cover the contract |
| 2 | transport tests versus backup/apply order | Backup gate precedes F1; errors never trigger full retry |
| 3 | editor tests versus worker ownership | No device handle on UI thread; connection generation is separate from drafts |
| 4 | validation versus distribution claims | Universal packaging does not prove Intel hardware operation |
| 1→2 | Profile, Report, Definition, DeviceIdentity | Signatures are defined once in the plan |
| 1→3 | Profile and key layout | GUI consumes the same indices and core validations |
| 2→3 | Session and DeviceDescriptor | Device ownership remains in a dedicated thread |
| 1,2,3→4 | root Cargo.toml and lockfile | Root manifest/integration belong to parent; workers own separate crate directories |

Plan review: two independent fallback reviewers found qualification and baseline-provenance gaps. Resolved with action-specific fail-closed qualification, opaque LiveBaseline, bounded one-key trial and explicit Glow backend. Both reviewers confirmed the material plan issues resolved.

Task 1: complete — both review findings resolved; 31 tests green including384 macro transition cases; parent native test, strict clippy and independent scoped re-review passed. Source remains in selected workspace with no Git commits.
Task 2: complete — three P2 findings fixed; independent scoped rereview clean. 41 device tests, 8 CLI integration tests and 1 CLI diagnostic unit test pass. Parent live F9/F2 read succeeded for 66EC(XRGB)BLe V1.0.48, three groups / 198 records. No persistent writes sent. Parent full test/clippy/fmt verification is recorded in task-2-parent logs.
Task 3: GUI implemented with35 initial tests; source review found a device-selection/live-session mismatch. Target binding and UI simplification are being corrected before final review.
Task 4: release CLI builds for arm64 and x86_64 pass; final Universal2 GUI packaging and UI verification remain.

User requirement: write thorough test code. Device and GUI briefs now explicitly cover fault injection, session provenance, backup failures, raw readback integrity, and draft preservation.

User priority amendment: key swap and calibration first. See ../specs/2026-10-10-priority-calibration-keyswap.md. This supersedes calibration exclusion and the initial non-RGB S-BLE assumption. Calibration static evidence and independent plan reviews are required before that implementation. Current real-device baseline: /private/tmp/niz-implementation-20261010/hardware-baseline.json.

Priority A: complete —47 core tests pass, native parent verification passed, independent actual-source review clean. Independent reviewer additionally checked6,633 same-layer pairs,396 malformed-address gates and342 opaque-tail changes. Fn anchors, atomic raw swap, unknown baseline retention and native import are implemented.

Priority B: complete after independent review and scoped rereview — Session-owned calibration, exact-XRGB experimental remap/swap, RGB reads and compound durable snapshot are in source; fault/cancellation/backup tests being expanded. Public proposal: read_lighting, trial_swap, begin_calibration, calibrate_held_keys, finish_calibration, cancel_calibration, calibration_status, cancellation_flag. Serializable CalibrationStatus and Error.calibration provide phase/completion/backup/unlock/trace evidence. Independent source review and scoped rereview found no remaining material defect after fixes.82 device tests and14 CLI tests pass. Native RGB read now captures198 bytes and the actual all-E6 terminator. Configuration writes and calibration remain untested on hardware.

UI choice confirmed2026-10-11: one-way From→To reassignment is standard; optional swap mode. Calibration is guided with physical released/held stages and mouse controls. Final command evidence includes official Windows and native Intel Mac software; exact user's502a/V1.0.48 remains experimental pending human-operated hardware checks.

Latest user confirms calibration included and simple GUI. Priority GUI brief /private/tmp/niz-implementation-20261010/priority-gui-brief.md narrows main views to assignment/swap and calibration, retaining backup/save/load and draft protection. GUI source is implemented; final target-binding fixes and layout review are in progress.


2026-10-11 visible UI refinement: user wants a simpler corrected GUI and trigger threshold switching because A is hard to actuate. Static/manual and existing baseline prove Right Fn+7 maps action149. No direct USB threshold setter/getter exists in established evidence. Add a physical shortcut guide, unknown current-stage text, transient A typing check, and A-specific sensor-calibration instructions. Do not infer a threshold or expose speculative commands.

Independent GUI finding: changing dropdown selection left the old live device eligible. Fix path-based selection/attachment, detach on target change/removal, preserve drafts, require explicit read, and include attached target in confirmation. Unverified physical legends must not be presented as confirmed layout: use fixed slot number and current assignment.

UI Computer Use currently blocked by a locked Mac; user has a pending unlock question. Owned unbundled demo PID25515 was stopped after verifying its exact command. A disposable demo-only Preview.app is prepared under the task temp directory, but not launched while locked. No Simulator used.

New gh-commit-identity skill is installed as separately authorized. Three independent source-review findings were fixed;25 tests and fresh native acceptance pass. Scoped rereview is in progress. No NIZ Git repository, commit, or push has been created.

Release packaging adjustment: installed Rust rust-objcopy cannot locate its bundled libLLVM through its rpath. Use Cargo strip=none and the native macOS /usr/bin/strip -S in build-macos.sh before ad hoc signing. No global toolchain modification.


Final integration2026-10-11: GUI fixes and scoped dropdown accessibility correction complete. Japanese font static weight400; default backup now local Application Support after observed Documents directory-open stall. Parent full189 tests/fmt/strict clippy pass and independent final actual-source review clean. Universal2 app/CLI/ZIP built and signed; both CLI slices start, x86_64 under Rosetta. Parent CUA verified demo remap apply, swap preview/Undo, calibration released/held/finish and disabled unrelated controls. All owned demo processes stopped; Documents interrupted test folder remains due bounded cleanup timeout, explicitly recorded. Final evidence: docs/validation/2026-10-11-final-validation.md.

Git ruling updated for user's explicit destination NIZTools: installed guarded skill read, fresh human GH identity checked, selected workspace initialized main with canonical requested origin. Skill25 tests and independent source/rereview pass. No source commit or push yet at this ledger update; publication uses the guarded helper after final source/evidence inspection. Manufacturer archives and full DLL disassembly excluded by .gitignore.
