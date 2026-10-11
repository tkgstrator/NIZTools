# Clickable Atom66 layout: independent source review

Workspace `/Users/devonly/Developer/personal/NIZ`, host `devonlys-MacBook-Pro.local`, base HEAD `bcc272fbe1706914666a1714838a78148d21ca01`. Reviewed the frozen GUI candidate after parent reported source freeze: new `keyboard.rs`, app integration, editor navigation/readiness, demo fixture backend, viewport size, and added keyboard/controller tests. No repository edits, Git mutations, HID calls or GUI operations were performed by this reviewer.

## Material finding

### P2 — Refresh selected-output readiness when the selected raw record changes through import or history

References: `crates/niz-gui/src/editor.rs:100`, `:110`, `:119`, `:128`; initialization/gate `:69` and `:91`; UI use `crates/niz-gui/src/app.rs:197` and `:231`.

The new `output_ready` state correctly prevents an unsupported selected record from acquiring an implicit basic replacement on navigation, but it is only refreshed on construction, different From selection or a layer change. Import, Undo and Redo can change the selected record while leaving readiness and To initialized from the previous record. Clicking the same board key does not repair this because same-slot selection intentionally preserves a pending To value.

Concrete native reproduction using the compiled GUI crate and its simulated captured fixture:

1. Construct `Editor` with the captured baseline. Default selected slot #2 is a basic action with To 15 and readiness true.
2. Import a valid profile changing that selected slot to a kind-1 combination `[43,44]`, with interval 100. No `choose_output` call is made.
3. The selected draft record is advanced, but `output_ready()` remains true and `proposed()` succeeds. Clicking the same selected key still leaves readiness true.
4. Calling the explicit add-change action replaces the imported advanced record with basic `[15]`, although the user never chose a replacement output for the imported record.
5. Undo restores the advanced record but readiness remains true. Navigating away/back correctly sets readiness false; a subsequent Undo restores the original basic record while readiness incorrectly remains false.

The actual probe is saved at `readiness-probe.rs` in this directory. Observed output:

```
imported selected action kind=1 output_ready=true old_to=15 proposal_ok=true
same-slot click changed_selection=false
navigation preserved raw=true output_ready=true proposal_ok=true
add-change after import without choose_output: selected action kind=0 keys=[15]
undo replacement to advanced: selected action kind=1 output_ready=true proposal_ok=true
navigate away/back to advanced: output_ready=false
undo import to baseline basic: selected action kind=0 output_ready=false proposal_ok=false
redo import to advanced: selected action kind=1 output_ready=false proposal_ok=false
```

Refresh selected-output state when the selected raw record changes on successful import, Undo, Redo and applied-profile acceptance. Preserve a pending To when the selected raw record does not change, and retain no-op selection behavior. Add tests for imported selected advanced/unknown/unassigned actions and for history transitions in both directions. Core `Definition.kind` accepts 0..4 (`niz-core/src/protocol.rs:58`); a valid kind-1 combination already proves the issue, so a proposed kind-8 definition is unnecessary.

This finding does not bypass hardware gating: arbitrary imported/advanced drafts remain subject to exact baseline trial regeneration and worker checks. It is a material selected-settings UX regression in the new readiness promise.

## Reviewed behavior without additional findings

- Board clicks update selection only; they do not commit draft records or send worker requests. Added controller tests explicitly prove no dispatch and rejection of a stale approval after navigation.
- Same-slot clicks preserve an in-progress output; mode changes reset the GUI-only click target to From; swap From automatically advances to To and To remains explicit. The second operand click does not apply a swap. Same operands remain visible and regenerate to a no-op.
- Busy, confirmation and popup guards are passed to both target changes and board clicks. App snapshots popup ownership before selector processing, so a click that closes an existing popup cannot leak into the board in that frame. Confirmation also disables the original assignment controls.
- Dirty markers compare full active-layer raw record vectors, with tests for unchanged labels plus changed padding and Undo. Opaque/advanced labels are compact and full preserved details stay in tooltips.
- Row geometry covers all66unique stable slots and ends every row at the same width. Added tests exercise 620/660/920px available widths; these are arithmetic/source checks, not a screenshot verification.
- The updated official-template legends remain separate from current mutable assignments. The parent-provided primary main-view/F2 trace strengthens the slot bridge beyond the earlier lighting-only evidence. Tooltip and selected-key labels say `公式配列`, and current assignments remain primary. This review does not establish a physical key scan or hardware qualification.
- Demo backend now loads the already-sanitized captured profile fixture; its branch remains separate from native enumeration/open. Core `Profile::demo` and core/device source are unchanged. Experimental trial guards and target pinning are unchanged.

## Verification actually performed

- `cargo test -p niz-gui --locked`: exit0, all60GUI tests passed (27controller,12editor,12keyboard,9worker). Log: `reviewer-gui-tests.log`.
- Independent native compiled readiness probe reproduced the material finding above, without opening a window or touching HID.
- Reviewed current dependency implementation of egui `Ui::put` and button layout for bounds behavior; parent remains responsible for screenshots, actual pointer navigation, minimum/short window fit and app build integration.

## Assessment

Resolve the P2 selected-readiness/history finding and run its regression tests before declaring this candidate complete. No other material source finding was identified in the bounded GUI diff. Request a scoped rereview of the readiness fix; parent visual verification remains outstanding separately.
