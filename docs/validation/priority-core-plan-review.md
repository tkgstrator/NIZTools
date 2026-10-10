# Priority amendment: independent core plan review

Read-only independent Codex fallback; preferred LocalGPT/Claude MCP tools unavailable. Verified `devonlys-MacBook-Pro.local`, workspace `/Users/devonly/Developer/personal/NIZ`. Reviewed priority amendment, captured baseline JSON, current core editing/validation/patch logic, relevant public reference Fn rules, and the device validation call site. No repository edits, hardware/HID operations, commits or subagents. A standalone read-only probe under this temporary directory loaded the supplied capture; it did not open a device.

## Findings

### [P2] Fn correction alone leaves the actual live baseline unwritable because six unchanged actions are outside the core code whitelist

References: `crates/niz-core/src/protocol.rs:52-61`, `crates/niz-core/src/profile.rs:364-369`, `crates/niz-device/src/session.rs:348-349`; `/private/tmp/niz-implementation-20261010/hardware-baseline.json:98`, `:99`, `:106`, `:164`, `:165`, `:172`; amendment `docs/superpowers/specs/2026-10-10-priority-calibration-keyswap.md:13`.

The verified capture contains code182 at group2/3 key21, code178 at group2/3 key22, and code179 at group2/3 key29 (zero-based indices86/87/94/152/153/160). `known_code` accepts only1..177 and199. The actual captured profile therefore fails `validate_for_writing()` with `unknown or reserved action code` before it reaches the existing Fn rejection. Device application validates both baseline and desired through this method. Fixing Fn will permit unrelated offline edits, but a basic remap/swap trial will still fail on these six unchanged records.

The plan must specify how previously captured, structurally understood but semantically opaque actions survive a baseline-derived bounded trial. Preserve these six records exactly. Do not add178/179/182 to generated/editable actions simply because the hardware returned them, and do not relax validation for arbitrary externally supplied profiles. Either obtain independent semantic evidence for the exact codes or introduce a baseline-aware validation path that accepts an unknown action only when its complete original raw record remains unchanged in the desired profile. Touching/moving such an action should still reject. The write path must continue comparing all records on readback and retain all model/firmware/session/backup gates.

Meaningful tests: use the actual capture or an independently constructed fixture with these extra Fn assignments and opaque action codes. Test unrelated basic remap and two-key swap eligibility without sending HID; exact retention of the six opaque records; touched/moved/newly introduced unknown code rejection; externally supplied changed opaque bytes rejection; and full readback detection of altered opaque bytes. Generic synthetic fixtures with only old known codes will not catch this actual acceptance blocker.

### [P2] Explicitly validate all six affected records for a linked-column swap

References: amendment `docs/superpowers/specs/2026-10-10-priority-calibration-keyswap.md:12`; current edit validation `crates/niz-core/src/profile.rs:227-236` and `:246-250`.

The amendment says both selected source definitions must be understood, then expands an anchor swap into moving two columns across all three layers. Understanding only the two selected-layer records is insufficient: another layer at either physical position can contain an unknown/uneditable action. Moving it changes its physical meaning and addressing despite its payload being preserved, contrary to the requirement that unknown records remain unchanged. Make the precondition explicit: validate all six records before an all-layer swap and fail atomically if any affected record is unsupported; validate two records for a layer-local swap. Existing patch-based linked edits already reject an uneditable later-layer record atomically, and raw swap should preserve that protection.

Test an unknown action in each otherwise unaffected alternate-layer source position. Assert no record changes on rejection. This is a plan clarification, not a demonstrated defect in an unimplemented swap API.

## Supported Fn correction

The captured stream independently confirms normal-layer LeftFn166 at physical key57 and RightFn156 at physical key62. Both assignments are synchronized on all three layers. Physical key61 is RightAlt71 on normal layer and an extra RightFn156 on layers2/3. The existing `validate_fn` incorrectly requires every occurrence of any Fn to be identical across all layers, and a standalone actual-capture probe reproduced an unrelated edit failure `Fn differs between editable layers`, with exact unchanged raw reports after rejection.

The proposed normal-layer-anchor rule matches the reference `protocol.ts:277-292`: inspect Fn assignments in normal layer, then require each anchor's other layers to match. The reference requires at least one Fn overall; keep this project's stronger requirement of at least one normal-layer anchor for each LeftFn and RightFn side. Do not hardcode keys57/62 or require exactly one anchor per side: adding a duplicate synchronized anchor is how the existing safe move workflow works.

Proposed set_definition routing is sound if based on the normal-layer definition at the selected physical position, not on whether an alternate-layer current definition happens to be Fn. Thus replacing an extra layer2/3 Fn with an ordinary action edits that one record and leaves normal RightAlt intact. A genuinely new Fn assignment remains an explicitly all-layer edit. Preserve the current exact no-op early return: selecting an existing extra Fn and making no semantic change must not silently promote it onto normal layer. Removing the last normal-layer anchor must reject even if extra alternate-layer Fn assignments remain.

## Swap implementation and regression criteria

- Require both indices inside editable records, valid structure, and same layer before deciding same-index no-op. No malformed/out-of-range index can succeed via an early no-op.
- Determine column expansion from whether either physical position is a normal-layer Fn anchor. An extra alternate-layer Fn is not a normal-layer anchor. UI preview must identify two versus six moved records from the same rule used by core.
- Perform vector/record moves on a draft, rewriting only bytes2/3 in **every** continuation report to the destination group/key. Never implement swap as two calls to set_definition/patch: those routines regenerate supported fields onto destination padding and synchronize Fn differently. Raw swap must carry source padding, macro short/native representation, and opaque terminal fields with its source.
- Do not suppress a raw swap just because decoded definitions are equal. Equal semantic definitions can have different padding or terminal bytes; an explicit swap exchanges those complete records, and a second swap restores their original bytes.
- Validate the resulting anchor invariant before assigning the draft. All six preconditions and mutation must be atomic; no first-half commit before a later source failure.
- Test each layer; two ordinary sources; a normal anchor selected on each layer; both anchors; duplicate anchors; extra-only Fn without a normal anchor; swapping an anchor with the actual key61 column; same-index; cross-layer/out-of-bounds/extra-group rejection; unknown affected sources; native/short macros with unequal report counts; equal semantics but unequal raw bytes; exact double-swap identity. Extra groups and all unselected records must remain byte-identical.
- The all-layer decision remains anchored after a swap because the normal-layer Fn moves to the other selected position, supporting involution. Tests should make this property explicit.

## Priority scope and readiness

The amendment appropriately keeps ordinary writes, bounded trials, Fn movement, macro editing, lighting, and calibration qualification separate. Basic trial qualification must not certify all-layer Fn swaps; those remain offline editing until separately authorized and qualified. Calibration is explicitly gated on command-level independent static evidence and requires human operation of released/held stages; this core plan review does not verify a calibration protocol or authorize a hardware write. Analog calibration rollback is correctly excluded from the meaning of an F2 settings backup.

The normal-layer-anchor correction and raw atomic swap plan are feasible with current record storage. Resolve the two P2 plan findings above before considering the concrete live target ready for bounded write trials. Offline core implementation can proceed with the stated guards and failing regression fixtures; no broad architecture change is required.

## Coordinator extension reviewed during this plan review

The coordinator proposed `Profile::validate_changes_against(&baseline)` after receiving the actual opaque-code evidence. This **resolves the first P2 at the design level**, subject to implementation and regression verification:

1. Validate structure for both profiles, exact identity/firmware/groups, and corrected normal-layer Fn anchors on baseline and desired.
2. Permit semantically opaque actions only at raw-identical unchanged positions. For every changed editable position, require both old and new definitions to pass the existing editable-action validation. Every additional-group record must be unchanged.
3. Preserve strict standalone `validate_for_writing`, so arbitrary unknown-code profiles still cannot pass standalone write validation.
4. Device validation runs baseline-against-itself and desired-against-baseline; qualification checks only affected actions, with the current bounded trial still limited to known old/new single basic keys. This must not certify moving Fn, macro sources/targets, or calibration.
5. Native same-device/version imports may retain unchanged opaque records. For a3-group import onto9 groups, construct the merged target-sized draft before invoking exact-group change validation. Legacy semantic imports retain strict complete editable-definition validation.

This contextual validation is preferable to expanding the whitelist: independent inspection establishes the bytes exist on this firmware but does not establish code178/179/182 semantics. Exact unchanged raw equality is the evidence-backed condition for preservation. Setter validation of the actual edited record plus corrected full-draft Fn checks remains appropriate for offline edits; unrelated opaque records need not block them.

Tests should demonstrate: actual capture structural/Fn validation after correction; six opaque records preserved through unrelated basic remap/swap and mock complete transmission; changed unknown source, changed unknown target, and padding-only change of an unknown record rejection; extra-group delta rejection; exact metadata/group rejection; legacy unknown action rejection; native3→9 retention; stale/session baseline checks retained; opaque-byte readback mismatch detection. The all-six affected-source requirement in the second P2 still applies to linked-column swaps.

No additional material flaw was identified in this extension as stated. This is approval of a proposed contextual validation design, not an assertion that it has been implemented or tested.
