# Official Held-Key Calibration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans. Track checkable work below.

**Goal:** Stock-firmware held-key calibration from the physical keyboard UI with complete before/after normal threshold comparisons.

**Architecture:** Add a separate held-only preparation entry to the owned device session, preserving the existing full calibration API. The GUI prepares that entry, freezes the physical position, sends one mouse-authorized DD, and automatically finishes comparison/cleanup on the same worker/session.

**Tech Stack:** Rust workspace, egui 0.36, native hidapi, deterministic transport and rendered GUI tests.

**Spec:** `docs/superpowers/specs/2026-10-11-held-key-calibration.md`

## Global Constraints

- Exact target: 0483:502a, usage 008c:1, firmware `66EC(XRGB)BLe;V1.0.48;V1.0;`.
- Selected physical position is 0..65. It guides holding; firmware has no key mask.
- No DB/D5/D8, analog rollback, numeric setter, firmware modification or blind retry.
- Same owned handle, qualified DC mode 1 only, complete 11-row captures and existing failure settling.
- Actual changing UI execution uses `clicked_by(PointerButton::Primary)`.
- No worker Git commits, real HID I/O, native app launches, or further delegation. Coordinator integrates, reviews, packages and commits.

## Review Focus

- Post-DD cancellation/partial read must retain before and dispatch evidence without false comparison success.
- DC failure settling completes under ownership before one cleanup unlock.
- Later keymap/RGB mismatch must retain complete analog observations.
- New command belongs to pending/active ownership, target/generation, replay, close/cancel checks.
- Physical selection and real pointer activation remain guarded despite remapped/weak key events.

### Task 1: Owned device calibration and comparisons

Files: `crates/niz-device/src/calibration.rs`, `session.rs`, `lib.rs`, device tests only.

Interface: `Session::begin_held_key_calibration(&LiveBaseline, &Path, usize) -> Result<CalibrationStatus>`. Add `CalibrationStatus::held_key: Option<HeldKeyCalibration>` with public `position: usize`, `before: Option<Vec<u8>>`, `after: Option<Vec<u8>>`; populated snapshots always contain all 66 physical-order values. A changed-position helper may derive factual deltas. Full calibration status uses None.

- [x] Write/run failing transport tests for held-only preparation, no DB, pre/post complete R capture and one DD/unlock.
- [x] Implement scoped internal DC1 reuse, before capture before D9, existing durability/settings checks, retained-B entry directly AwaitingHeldKeys.
- [x] Held uses existing DD/DE; finish captures after R while locked, then unlocks once and verifies keymap/RGB. Preserve full calibration behavior.
- [x] Cover failures/cancel/deadlines/extra rows/public active-reader guard, selected/neighbor/no-change cases, and complete comparison surviving settings failure. Run device suite.

### Task 2: GUI/worker and interaction tests

Files: `crates/niz-gui/src/*` and GUI tests only.

Consumes Task 1 interface above. Add `Command::BeginHeld { backup, position }`, corresponding Backend and controller entry. Worker Held routes held-only successful DE directly to backend finish and Response::Finished; existing full mode stages remain separate.

- [x] Write/run failing controller/worker/rendered GUI tests for physical target freeze, pointer-only DD, compact prep/hold/result flow.
- [x] Preparation button reads all keys released and enters held wait; red Execute and Cancel are horizontal. Explain one physical hold, neighbor possibility, no analog restore concisely.
- [x] Show selected before→after and other changed count prominently; all 66 values and neighbor legends in collapsed results. Mark missing after as unavailable and DEMO as simulation.
- [x] Cover pending/modal/close/replay/metadata guards, no automatic command from key selection, no keyboard/AX execution, no-change/neighbor/error outcomes, themes/geometry/hover and existing editor behavior. Run GUI suite.

### Task 3: Integration, independent review and delivery

Files: documentation/build metadata and any integration-only fixes; no competing edits during Task 1/2.

- [x] Resolve the independent client/firmware ordering difference using verified DC1 prior-enable restoration: compare while D9=0, unlock once afterward.
- [x] Inspect actual diff and request independent read-only reviews; resolve material findings.
- [x] Run full workspace tests, fmt and clippy; build/sign universal arm64/x86_64 app/CLI, verify archive and native GUI appearance (native pointer execution blocked by CUA).
- [x] Document deterministic and native evidence separately from unperformed physical held-key calibration; guarded Git integration and remote verification.
