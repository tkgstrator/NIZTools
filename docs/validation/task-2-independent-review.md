# Task 2 independent source review

Reviewed by independent Codex fallback reviewer on `devonlys-MacBook-Pro.local`, workspace `/Users/devonly/Developer/personal/NIZ`, 2026-10-10. LocalGPT and Claude MCP were unavailable; neither review is claimed. This is an actual-source review of newly added Rust code against task-2-brief.md and task-2-report.md. No Git repository was initialized, no production source was edited, and no hardware I/O, Simulator use, commits, or external messages occurred. Temporary offline repro artifacts are retained in `/private/tmp/niz-implementation-20261010/task-2-review-offline/`.

## Findings

### [P2] Descriptor gate ignores other collections sharing Report ID 0

File: `crates/niz-device/src/transport.rs:183-195` (especially line 184); final width check at 202.

The parser adds input/output bits only while inside the selected usage008c/1 collection. HID reports are sized by report type and report ID across the descriptor, so another collection using ID0 contributes additional bytes to the same unnumbered report. A descriptor with the expected512bits in the selected collection plus8input/outputbits in another collection passes this gate even though Report ID0 is65bytes. Native open therefore accepts a descriptor incompatible with the asserted64-byte logical/65-byte outbound framing. The runtime catches an oversized read, but that does not make the pre-I/O descriptor safety gate correct, and a trial could send commands before any such read exposed the mismatch.

Reproduced by running the exact current validate_descriptor function in an isolated Rust executable with two independent HID collection byte fixtures. Result: `descriptor with total 520 input/output bits on Report ID 0: Ok(())`. Evidence: task-2-review-offline/descriptor-probe.rs and descriptor-probe-evidence.txt.

Requested change: account for all fields contributing to each input/output report ID and require the complete ID0 input/output report widths to be512bits. Keep the separate requirement that the configuration collection itself provides the expected fields. Add a negative fixture with additional ID0 fields outside the selected collection. Reject descriptor layouts whose numbered/unnumbered framing cannot be established safely.

### [P2] Concrete CLI diffs omit repeat and macro parameters

File: `crates/niz-cli/src/main.rs:169-177`.

The common diff renderer prints only kind and key names. Changes to repeat interval, macro interval, cycle count, custom-delay mode, or individual delays are counted as changed records but cannot be seen in the displayed before/after action. Thus the required concrete apply/restore preview can show identical actions for materially different behavior, and the offline diff command cannot explain a timing-only change.

Reproduced using independent raw JSON fixtures derived offline from the parent's saved capture, with a repeat record differing only in interval100 versus200. `niz diff repeat-100.json repeat-200.json` exits0 and prints `kind 1 ["Tab"] -> kind 1 ["Tab"]`; neither interval appears. Evidence: task-2-review-offline/cli-diff-evidence.txt and repeat-100.json/repeat-200.json. This repro performs only offline file parsing.

Requested change: include changed semantic fields in the preview, including interval/cycles/delay mode/delay values. Explicitly identify opaque-byte-only changes if the decoded definitions are equal. Extend CLI integration tests with timing-only and opaque-only differences and assert meaningful before/after output.

### [P2] Readback mismatch discards the bytes needed for diagnostics

File: `crates/niz-device/src/session.rs:422-429`; CLI capture consumer `crates/niz-cli/src/main.rs:139-140`.

After a complete syntactically valid F2 readback, a difference in any opaque or unmodified byte takes the `Error::new` branch. That fresh error has an empty raw_reports vector, and the verified Profile is dropped. The subsequent partial-write error retains backup_path and began_write, but --capture records no received reports. This is precisely the failure where the application's actual hardware outcome is uncertain and its bytes should be preserved for investigation; rereading later is a new observation rather than evidence of the failed transaction.

Verified from the actual error path: Error::new initializes raw_reports to an empty vector (`crates/niz-device/src/lib.rs:15-20`), and transaction error wrapping at session.rs441-445 never adds readback bytes. Existing `readback_unknown_byte_change_detected_no_retry` exercises the mismatch but does not assert raw diagnostics.

Requested change: attach the actual received readback reports to this error before discarding the Profile (and retain record-level mismatch context if useful). Extend the existing opaque-corruption test to assert exact raw_reports content and a saved --capture payload, with no retry or rollback.

## Verification

Executed `cargo test -p niz-device -p niz-cli` in the intended workspace:36device tests and6offline CLI integration tests passed; doc tests passed. The CLI tests invoked by this suite use only malformed/offline profiles, help output, and refusal before native device opening. Device tests use private fake transports, temporary files, and actual OS advisory locks.

Executed the isolated descriptor probe compiled with rustc and an offline CLI diff repro. All reproductions are offline. No real-device commands were sent by this reviewer. The existing owner's hardware baseline was read only as a JSON file.

The source does implement the principal protections: normal qualification table remains empty; unsupported changed actions fail before additional HID traffic; outbound reports are65bytes with ID0 and incoming64bytes retain their leading zero; failed reads/write transactions poison sessions; old/foreign baseline tokens are refused; external firmware/configuration changes are rechecked before F1; backups use no-clobber publication, file/directory sync, and reparsing before F1; partial writes have no automatic retry/rollback; desired A is applied over current baseline B; all record bytes are compared during readback. Most session fixtures are generated through Profile::demo/core encoding, so independent literal native packet fixtures would strengthen the claimed protocol evidence even though the tests exercise actual session behavior.

## XRGB/lighting and scope

The parent's current capture is66EC(XRGB)BLe firmware. The public bounded trial is deliberately S BLE-only and refuses that firmware; the normal qualification table is empty. No current public write path is therefore enabled for that XRGB device. Local reverse-engineering evidence describes configuration F1/F0/F6 separately from lighting E1/E0/E6, and this source sends no lighting command. That is not hardware evidence that a configuration commit preserves independent lighting state. Any future XRGB qualification should capture and compare that state before enabling writes. Calibration and key-swap work are future scope and their absence is not a finding in this review.

## Assessment

No currently enabled ordinary-write bypass was found. The three P2 findings should be resolved before presenting the protocol/diagnostic implementation as complete or expanding hardware qualification. Existing passing tests do not cover these cases.
