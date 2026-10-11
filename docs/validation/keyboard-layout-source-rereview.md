# Clickable Atom66 layout: scoped source rereview

Host `devonlys-MacBook-Pro.local`, workspace `/Users/devonly/Developer/personal/NIZ`, base HEAD `bcc272fbe1706914666a1714838a78148d21ca01`. Read-only rereview of the frozen P2 fix in `crates/niz-gui/src/editor.rs` and its five added regressions in `tests/keyboard.rs`. Also checked the final keyboard source/caption and unchanged controller/worker integration boundaries. No repository edit, Git mutation, HID operation or GUI operation was performed.

## Findings

No unresolved material finding identified in this scope. The prior P2 selected-output readiness inconsistency is resolved.

`Editor::replace_draft` (`editor.rs:109`) compares the selected layer/slot's complete raw record before and after replacement. It refreshes selected output/readiness only when that record changes. `commit`/import, Undo, Redo and applied-profile acceptance now use this path (`:103`, `:123`, `:132`, `:150`). Thus advanced, unassigned, unknown and unsupported special records require a new explicit supported output choice after selected-record transitions. Conversely, unrelated record changes and acceptance of an identical selected record preserve the pending To value. Existing same-slot navigation remains a no-op and does not add history or change records.

The five new tests cover selected advanced import plus history in both directions, unassigned/special history, known↔unknown applied-profile acceptance, unrelated transitions preserving To, and selected raw-padding changes refreshing state. They exercise observable setting behavior and exact raw records rather than simply mirroring the helper implementation.

Final board labels use `BS` for Backspace, avoiding a missing-glyph dependency. Static physical template legends are explicitly separated from mutable current assignments and described as `公式配列`. The primary host-software indexing trace supports this template; physical key scan/firmware qualification is still not asserted. Existing exact-trial regeneration, target/generation pinning, native/demo separation and pending/confirmation/popup guards remain unchanged from the earlier reviewed candidate.

## Independent verification actually performed

- `cargo test -p niz-gui --locked`: exit0, all65GUI tests passed (27controller,12editor,17keyboard,9worker). Full log: `reviewer-gui-rereview-tests.log`.
- Compiled and ran a separate native assertion probe against the rebuilt GUI dependency: `readiness-rereview-probe.rs` / `readiness-rereview-probe`. It proves the original import/same-slot defect now refuses implicit replacement; basic↔advanced Undo/Redo refreshes correctly; explicit replacement remains supported; unrelated import/history/applied acceptance retains a pending To. Exit0 and output:

```
PASS: import/same-slot/basic↔advanced history are refreshed; unrelated import/history/acceptance preserve pending To; no HID/UI opened
```

## Assessment

The source finding is resolved and this bounded GUI candidate passes the independently run regression suite. Parent visual verification, minimum/short-window checks, Universal2 build integration and final workspace verification remain separate evidence; this rereview does not claim those actions occurred.
