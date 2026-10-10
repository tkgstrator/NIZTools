# Priority GUI review corrections and simpler adjustment view

Host/workspace: devonlys-MacBook-Pro.local, /Users/devonly/Developer/personal/NIZ. Owned source changes only crates/niz-gui/**; parent owns font asset, root manifest/build/package work and actual Computer Use/screenshots. No native HID operation, calibration, physical shortcut, Git or Simulator was performed by this worker.

## P1 concrete-device binding

Reproduced the independent source review's five device-selection regression failures in priority-gui-selection-red.log. A selected dropdown no longer uses a mutable numeric index. Controller stores selected_path independently of attached_target, and list refresh reconciles by path. Reordering the list preserves selected A, removing A clears selection and detaches hardware eligibility, and changing A's descriptor at the same path detaches eligibility. Selecting B immediately detaches hardware eligibility without discarding the editor or undo history. Selecting A again does not silently reattach; an explicit Read is required.

Retain-read reconciliation still requires the original editor source path and exact baseline identity/firmware/records. Reading B with Retain therefore preserves A's draft detached. Explicit Discard reread replaces it only after success. A current selected-versus-attached path/descriptor guard now exists in the controller in addition to UI disabling.

Every changing request, including calibration stages and safe cancellation, carries the concrete attached Target path/product/identity. The worker stores its actual read target, checks its descriptor against the read profile, and rejects changing requests with a missing, foreign-path or foreign-identity target before invoking the backend. Native Session and LiveBaseline remain worker-owned.

Apply confirmation captures an immutable TrialApproval with target, generation, baseline, draft, selection and backup directory. It displays that concrete path/product/identity and snapshot's exact changes. Confirmation cannot follow a dropdown switch or a changed draft, and approve_trial rechecks the current source and snapshot before dispatch. Calibration displays attached product/path information and uses the same controller/worker binding. Pending calibration still prevents switching/list refresh.

## Simpler Japanese interface

The primary assignment surface now has two aligned selectors in a card, compact layer controls and concise changed-record preview. From/physical destination labels use stable slot numbers plus explicitly marked current assignments. All unverified KEY_ROWS physical-legend labels were removed from GUI selectors/previews. The interface explains that slot-to-printed-legend correspondence is unverified; current assignments are never presented as physical legends.

Detailed identity/path/firmware, backup paths, English gate errors, protocol phase enums, trace counts and preservation flags are collapsed. File/save/backup actions are accessible through a collapsed section. The unused disabled normal-Apply dummy was removed. Valid no-op selections disable Add, clean drafts disable Apply, and the preview explicitly says 変更なし. Controller/device no-op reporting remains truthful and tested.

The root corrected the Japanese glyph weight by replacing assets/fonts/NotoSansJP.ttf in place with verified static Regular400. This worker did not edit that asset or include path. Final debug GUI was rebuilt after the asset timestamp: font01:14:41 and executable01:16:55 on2026-10-11. Actual visual verification is performed separately by the parent.

## Physical trigger guide and A-key calibration

Calibration view begins with a physical Right Fn + printed7/& shortcut guide based on trigger-threshold-research.md. It tells the user to release the shortcut and compare light A presses, says the current trigger stage cannot be retrieved, and cites the manual's return-to-default on shutdown. No USB setter, software threshold slider, stage inference, sensor measurement or changing command was added.

Whether the shortcut is mapped is derived from the currently live exact XRGB V1.0.48 /0483:502a three-group baseline at right-Fn layer/index66+7, requiring a single action149. A file-only/detached/wrong-target profile never receives a verified-mapping statement. A current custom assignment receives an explicit standard-manual-operation disclaimer. Demo statements remain under the visible DEMO banner. The trigger guide classification regression failed first in priority-gui-trigger-guide-red.log and passes in the final suite.

A transient memory-only typing field lets the user compare A input. It is disabled during pending operations and throughout active calibration/user waits, and is explicitly described as keyboard input rather than a sensor measurement. It is never saved or converted into threshold state. The actual factory calibration prompts are now release every key -> physically hold only printed A and click with mouse -> release A/all keys and finish with mouse. Existing clicked_by(Primary), keyboard-activation rejection, cancellation waiting and unlock distinctions remain.

## Evidence and source freeze

- priority-gui-selection-red.log:5 material target regressions failed before correction.
- priority-gui-selection-green.log:22 controller and7 worker tests passed after target binding.
- priority-gui-trigger-guide-red.log:classification behavioral red captured against a stub.
- priority-gui-fixes-workspace-green.log:full workspace189tests passed:4CLI unit+10CLI integration+31original core+16priority core+82device+25GUI controller+12GUI editor+9GUI worker; no failures/ignored tests. This full run preceded only final visual-only no-op button/collapsed-section/string changes.
- priority-gui-fixes-focused-green.log:final46GUI tests pass after every source/visual-only change, including25controller+12editor+9worker. Tests cover target switching/reorder/removal/same-path descriptor drift/switchback, retained draft/history, explicit replacement, changing approval invalidation, worker foreign/missing target refusal, action149 guide qualification and the retained calibration/thread cleanup tests.
- priority-gui-fixes-clippy.log:final owned all-targets Clippy with-Dwarnings passes.
- priority-gui-fixes-fmt.log:final owned formatting check passes.
- priority-gui-fixes-build.log:final host debug build passes and includes the parent's corrected font.

Source frozen and parent notified. Scoped independent actual-source re-review, final packaging and real visual verification remain parent-owned. No claim of real trigger state, calibrated analog output, typing recovery or persistence is made.
