# Independent actual GUI source review

Reviewed on 2026-10-11, verified host `devonlys-MacBook-Pro.local`, workspace `/Users/devonly/Developer/personal/NIZ`. Read-only review of `crates/niz-gui/**`, GUI brief/report, priority amendment/plan, core profile operations, and native Session/calibration/transport integration. No repository edits, Git operations, native HID calls, GUI launch, Computer Use, or Simulator resources. Only this review artifact is written. LocalGPT/Claude callable tools are absent in the currently observed tool metadata; this is the coordinator-authorized independent Codex fallback, not a Claude review.

Verdict: one material target-selection defect must be resolved before hardware GUI use. Other reviewed ownership and bounded-write paths meet the inspected acceptance criteria, subject to actual visual and human hardware verification by the parent.

## P1 — The displayed selected device can differ from the device that receives Apply or calibration

References: `crates/niz-gui/src/app.rs:533-541` (dropdown changes only `device_index`), `app.rs:323-337` (Apply gate only checks controller live baseline), `app.rs:369` (Begin gate only checks controller live baseline), `app.rs:456` (Apply confirmation lacks source device path), `crates/niz-gui/src/controller.rs:122-136,138-166` (changing commands do not carry/check a selected source path), `crates/niz-gui/src/worker.rs:175-191,200-209` (commands use the originally read Session).

Repro from actual source: enumerate two supported same-model devices A and B; explicitly Read A; create one eligible draft change; select B in the device dropdown without pressing Read; click experimental Apply and confirm. The dropdown visibly names B while `controller.live` stays true and the worker sends the transaction to its existing Session A. Calibration Start has the same mismatch. Merely refreshing the list may also move/reuse the selected numeric index while the live session stays attached to A. Using identical product/firmware devices makes the current firmware-only confirmation insufficient to reveal the mismatch.

This is not an arbitrary-profile bypass: the worker correctly regenerates from A's genuine baseline. The defect is that the device receiving the write differs from the apparent selected target. It violates the explicit target confirmation/device switching requirements.

Required fix: track the attached device path independently from the prospective selection; when the dropdown selection actually changes, detach hardware eligibility (preserving the draft and undo history) and require explicit Read with retain/discard handling. Resolve selection by path across list refreshes rather than allowing list order to silently rename it. Show the concrete attached product and path/identity in Apply and calibration, and bind confirmation to that target. Guard changing operations against a current selected-versus-attached mismatch in the controller so UI disabling is not the only boundary.

Regression tests: Read A, create an eligible one-edit draft, select B, prove no Trial/Begin request can dispatch until B is explicitly read; verify draft/history remain unchanged and retain mode on B remains detached; verify a reordered device list cannot change the apparent writable target. Also exercise selection change with an open Apply confirmation. Current 35 tests contain no App/device-selection or list-reordering coverage, so they pass despite the defect.

## Physical-label evidence gap for parent visual/hardware verification

`app.rs:621-627,635-655` flattens `niz_core::KEY_ROWS` into labels advertised as physical key names. Those rows originate in the saved niz-web demo model, not a newly verified photo of this board. The observed real fixture differs from the demo at important positions: #14 is labeled backslash but outputs grave; #15 is labeled grave but outputs Backspace; #29 is labeled Backspace but outputs backslash; #55 is labeled R Fn but outputs Delete; #57 is labeled Cmd but outputs L Fn; #62 is labeled Menu but outputs R Fn. The real fixture has its normal Fn anchors at #57/#62, whereas the demo places them at #55/#59.

These differences alone do not prove the physical labels are wrong, because existing assignments can be customized. Parent should verify the legends and report-index order against the actual keyboard/photo/official layout before using named physical selectors for writes. Do not derive physical names from the mutable current output: that confuses remapped outputs with physical positions. If the reference layout differs from the board, correct the layout mapping and add a fixture-based position test. This is explicitly an evidence gap, not a confirmed physical-layout bug or claim of hardware testing.

## Checks and positive source evidence

- Independently ran `cargo test -p niz-gui --all-targets`: 17 controller + 11 editor + 7 worker tests passed, zero failed/ignored. It includes the actual demo worker thread cancellation/close test. This run does not independently establish the claimed 178 workspace tests or hardware behavior.
- Default editor Reassign uses a physical source and output action; Swap uses two physical positions. Regeneration clones the profile, uses core atomic operations, and commits only successful raw-record changes. Invalid imports and unknown record edits preserve draft/history.
- The GUI controller rejects a file-only baseline for hardware. The worker separately compares the entire draft against regeneration from its own baseline and rejects extra edits, changed raw padding, foreign metadata, unqualified firmware/identity/groups, and Fn-bearing physical columns. NativeBackend passes operands into `Session::trial_single_key`/`trial_swap`, not an external Profile into Apply.
- Ticket generation/operation matching prevents stale responses from changing the current editor. Worker rejects replayed operation IDs and stale generations. Pending requests block repeated clicks and unrelated work.
- Read retains/reconciles only when source path, identity, firmware, and baseline records match. Failed reads retain editor/history; explicit discard replaces only on a successful response. Import uses baseline-relative merging rather than replacing the trusted baseline.
- All three changing calibration stage buttons use `clicked_by(Primary)`; the separate controller Activation guards reject Keyboard. Source does not infer keys are released and sends no key mask.
- Dedicated worker owns the native Session and private LiveBaseline through user waits. Native transport keeps the per-path OS lock in its owned File. Controller and worker both prohibit ordinary work during calibration and reject out-of-order stages.
- Pending cancellation sets the shared flag and waits for completion. Dedicated cleanup is queued only at AwaitingHeldKeys/AwaitingFinish; terminal Failed/Aborted does not queue another unlock. Close waits for owned requests/cleanup; channel drop removes its sender and joins its exact owned worker.
- Demo/native branches are separate. Demo startup/read never instantiate NativeBackend Session or invoke HID enumeration. Demo output is labeled as simulated in App.
- NoChange wording expressly avoids hardware-write or RGB-postflight claims. Original Failure message, raw reports, began_write, backup path, and complete boxed CalibrationStatus are retained. The UI distinguishes unlock Sent/Uncertain and physical recovery/persistence uncertainty.
- No actuation-threshold setter was found in this GUI review. Calibration is factory release/held calibration; it must not be presented as implementing the user's latest weak-A trigger-sensitivity request. Parent's separate research remains required.

## Reviewed source snapshot

`app.rs` SHA256 `4cce91ec5c1a59db5ebab8decb2ccef3803db48c7c41611bb46c5ad1867d331b`

`controller.rs` SHA256 `8c55e3f22584c74f6f1dd27d93e288643558aceeda1b8c3280bb39dfbd0a7637`

`editor.rs` SHA256 `f0ec609c52ce98331a7c3ccb8a9c5fcb642ce012ffbb290133a9e47e3074a099`

`worker.rs` SHA256 `fbba912956aca1768e544e85d4d133ca71a2e1e4495431947be50feb8244b16d`
