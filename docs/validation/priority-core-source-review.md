# Independent actual-source priority core review

Reviewed 2026-10-11 JST on `devonlys-MacBook-Pro.local`, workspace `/Users/devonly/Developer/personal/NIZ`. Read-only repository review; no hardware, Git mutation, or source edits. A temporary independent probe was written only under `/private/tmp/niz-implementation-20261010`.

## Result

No actionable P0/P1/P2/P3 findings in the reviewed core extension. Section A of `docs/superpowers/plans/2026-10-10-priority-calibration-keyswap.md` and the priority spec are satisfied by the inspected source and offline evidence. No plan-review conclusions or other reviewer conclusions were read. Preferred MCP reviewers were absent; this is the authorized independent Codex fallback review, not a LocalGPT or Claude review.

The workspace has no Git repository, so an actual Git diff is unavailable. The review examined actual source, focused priority tests, relevant existing tests, protocol/legacy implementation, and the real saved fixture directly. No completion claim for subsequent device/GUI work is implied.

## Reviewed behavior

- `crates/niz-core/src/profile.rs:219-243`: reassignment links normal-layer Fn anchors or an explicitly new Fn assignment. Exact semantic no-op returns without raw normalization. Alternate-only Fn reassignment remains local. Linked patching occurs on a draft and validates final Fn availability.
- `crates/niz-core/src/profile.rs:248-296`: indices are bounded/editable and same-layer; source structure and Fn anchors are checked; all linked source records are checked before mutation. Complete records are exchanged and bytes 2/3 are rewritten in every continuation only. Final differential validation occurs before replacing the profile. Same-index validated no-op and atomic rejections are covered.
- `crates/niz-core/src/profile.rs:302-326`: both complete structures and normal-layer Fn anchors are checked, with exact identity, firmware, and record count. Raw-identical records may retain unknown actions at the same address. Every changed editable record requires valid known original and desired definitions. Extra-group changes are rejected. Counters/capture timestamp remain non-wire metadata, consistent with current requirement and tests.
- `crates/niz-core/src/profile.rs:426-452`: both Fn sides must appear on the normal layer and every normal anchor must be synchronized on both alternate layers. Legitimate extra alternate-only Fn remains accepted. Standalone validation remains strict over editable records, retaining the existing opaque extra-group treatment.
- `crates/niz-core/src/profile.rs:459-505`: native imports first merge onto the target; differential validation then rejects changed unknown source/target records and opaque extra-group deltas. Three-group native imports into nine-group targets retain target extra groups and metadata. Legacy source and final merged result remain strictly validated.
- `crates/niz-core/src/protocol.rs:143-212`: record decoding validates macro continuation addressing/header consistency and accepts native and short custom macro forms without discarding source raw bytes. The swap moves those records directly, avoiding re-encoding opaque terminal fields.

## Offline verification performed by reviewer

1. `cargo test --offline -p niz-core`: PASS, 31 existing core tests + 16 priority tests, zero failed; doc/unit targets have zero tests.
2. `cargo clippy --offline -p niz-core --all-targets -- -D warnings`: PASS.
3. `cargo fmt --all -- --check`: PASS.
4. Independent temporary Rust probe linked to the freshly tested niz-core artifact: PASS across all 6,633 unordered same-layer pairs including same-index cases from the live fixture. It observed 6,237 successful swaps and 396 expected opaque-source rejections, independently constructed exact raw destination records, verified the differential gate, and verified that a second swap restored all original raw records. It also checked 396 malformed-address validations in both desired/baseline directions and 342 opaque-tail byte changes, all rejected as required. A JSON-roundtripped external baseline was accepted only by the offline comparison API, confirming its intentionally non-authenticating nature.

Probe source: `/private/tmp/niz-implementation-20261010/priority-core-independent-probe.rs`.
Probe executable: `/private/tmp/niz-implementation-20261010/priority-core-independent-probe`.

The committed test suite also exercises continuation reports, 53-byte boundaries, maximum native custom macros, short custom forms, opaque terminal secondary fields, same-semantic differing raw records, cross-layer/invalid indices, linked six-record changes, normal Fn side preservation, native 3-to-9 imports, and strict legacy unknown rejection.

## Integration boundary requiring subsequent device review

`Profile::validate_changes_against` deliberately accepts any structurally compatible external Profile as an offline comparison baseline; its documentation at `profile.rs:299-301` explicitly disclaims live authenticity/qualification. Hardware authorization must remain in Session using the private LiveBaseline token, session/generation/current-read matching, exact target checks, and action-specific qualification. Current source has private token fields at `crates/niz-device/src/session.rs:10-14` and checks at `session.rs:269-283`; ordinary qualification is empty at `session.rs:39`.

At review time the device apply path still calls standalone validation at `session.rs:348-349`. Therefore the real opaque-code fixture is still blocked for device writes until planned Section B integration replaces this with the authenticated session-relative gate. This is expected deferred work and is not a defect in Section A. This review does not claim the stale/foreign-token, bounded trial-swap, calibration, lighting-backup, or GUI requirements have been completed or reviewed.

## Reviewed source fingerprints

- profile.rs SHA-256: `507fdd4c6c408407de21d4fda71785539210b23616ba6c3f8086924b1edeeab7`
- tests/priority.rs SHA-256: `47de44d9451fcb11964d724f97503a96d9c47db3872cccd6d5d1d38eb78e923b`
- live fixture SHA-256: `c0939e886b8f1d7b4243f9ef17b82605b094bb2c1f627b63a10fb07bb8efe9ef`

Fingerprints remained identical from start through final verification.
