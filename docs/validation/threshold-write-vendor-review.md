# Exact Atom66 V1.0.48 vendor write-path feasibility review

2026-10-11. Independent, read-only review on verified native host `devonlys-MacBook-Pro.local`, intended workspace `/Users/devonly/Developer/personal/NIZ`. Preferred LocalGPT/Claude MCP tools were not present in this call's actual inventory; authorized native Codex fallback was used. No new worker conclusions were read. Existing repository evidence supplied by the coordinator was used as source context, then relevant binary evidence was checked freshly. No repository edit, Git command, HID request, vendor program/firmware execution, GUI launch, reset, or update occurred. All new output belongs to `/private/tmp/niz-write-20261011/vendor/`.

## Result and feasible options

No overlooked selected-key numeric setter or full analog-table setter was established in the inspected configuration utility, its HWI DLL, or the current first-party CalibrationLite package. The recovered firmware updater is a different write domain: it stages a sequential decrypted image in serial EEPROM and finishes an update transaction. It is not an ordinary endpoint for arbitrary live RAM or on-chip analog-calibration flash writes.

The strongest assignment-preserving reset candidate is exact-target D5, but its accurate scope is **clear all 66 stored per-key calibration overrides to the FF sentinel, then regenerate live thresholds using the retained global mode**. It does not restore the released baseline B, reference sample, or global mode. Thus this can satisfy an explicitly defined "all per-key calibration overrides to default" operation after qualification; it cannot honestly promise all baseline/threshold/global settings return to factory defaults.

A real physical-board-selected key-only slider still needs a new qualified setter or separately authorized firmware implementation. There is no established host-only implementation plan that both changes A numerically and guarantees the other 65 remain unchanged.

## Material findings for the plan

### P1: keymap and lighting table writes cannot be repurposed as analog table writes

The current model utility `66EC(XRGB)Ble.exe` imports the normal HWI API for configuration, lighting, counters, version and update. Fresh HWI export inventory has 22 entries, no calibration/threshold/table setter. Its normal `setKeyConfiguration` stores an in-memory key-record object; `writeKeyConfigurationToDev` subsequently sends three layers of 66 F0 records. Fresh disassembly of type 0 serializer at `0x100043d5..0x100043f7` writes `00 F0 layer key 00 count values...`. The host-addressed key exists, but the data are output/function actions.

Fresh lighting path at `0x10002040..0x10002148` uses E1/E0/E6, 198 bytes total, three RGB bytes per key. The precise firmware dispatcher/storage evidence in repository `docs/validation/per-key-firmware-evidence.md` routes these to serial EEPROM rather than B/R/T analog tables. A "full table" in a vendor programming UI is not proof of a full analog-table setter. F0 key identity and assignable action 149 do not provide an analog-cell write API.

Retained evidence: `normal-record-disassembly.txt`, `keymap-write-disassembly.txt`, `lighting-write-disassembly.txt`, both model PE inventories. Do not emit F0/E0/F1/E1/F6/E6 as part of a threshold-only change/reset. A full analog setter, if discovered later, could preserve other 65 stored values only by a pinned complete read-modify-write under exclusive ownership, with validated writes/readback, race control and defined live-table/persistence semantics. No such analog setter was found here, so that design remains hypothetical.

### P1: the 3A updater path is not a shortcut to per-key calibration writes

The HWI update constructor at `0x1000352f` prefixes `00 3A`; `0x10003466` copies 64 normalized bytes; `0x10003472..0x10003478` sends 65 Windows bytes including report ID zero. It carries vendor wrapped/encrypted records, not a generic host RAM address/value packet.

Fresh exact-image analysis at `0x70f4` decrypts the padded record into a stack buffer via `0x634a`. Type 0 checks checksum (`0x718a..0x718c`) and the two-byte record address against a sequential cursor (`0x719c..0x719e`). It writes record body through `0x3334` at `0x71b8` with destination cursor+4 and verifies a byte sum by serial EEPROM reads through `0x3444` at `0x71c8`. Type 4 initialization zeros update state/cursor and disables scanner/peripheral activity (`0x7134..0x7160`). EOF type 1 validates fixed fields, calls image validator `0x5640` at `0x7252`, writes a four-byte `CC CC length` marker via `0x33d4` at `0x7272`, invokes control/reset path `0x4fb4`, and loops at `0x727e`. Error replies include `00 3A A0` and `00 3A A1`.

The analog calibrated block is separately persisted by `0x8954` to on-chip flash rooted at `0x10e00`, as the supplied exact-image report establishes. 3A has no identified branch to that table writer or to RAM R/B/T. Even when a staged record's numeric address resembles an analog flash address, the write helper's address space is serial EEPROM and does not make it a calibration setter. Finishing the update stages a restart/firmware operation with different risks and scope. Do not silently generate/update firmware or wrap a table change as an updater action.

Retained evidence: `firmware-records-disassembly.txt`, `updater-target-disassembly.txt`, `serial-eeprom-disassembly.txt`. Every firmware address above comes from SHA-256 `b82e11891dfdc138780a81ac635697a7edb0ca1d4e123d6ad38c81a7ba55bf17`, not a newer public image.

### P1: D5 clears calibration overrides; it is not full factory calibration/threshold recovery

Fresh inspection of the supplied exact D5 bytes agrees with `0x3094..0x30d8`: disable scanning; reset transient scanner/key-output state via `0x506c`; loop 11 x 6 and write FF to T at `0x200010cc`; persist the full T block with `0x8954` at `0x30ca`; regenerate R with `0x7624` at `0x30ce`; restore prior scan-enable. The dispatch path also performs shared feedback/state handling. No B, reference or global-mode reset appears in this branch, and it does not use the keymap transaction/index or RGB write paths.

For sentinel T, the exact D8 routine regenerates R as `10 + [2,4,8][global_mode]`, i.e. 12/14/18 in raw firmware signal units. D5 retains global mode, so it need not give the same live R on two keyboards with different stored mode. B remains the existing per-cell released signal table. D5 does not repair a bad B established by prior release calibration. Adding DB would replace every B and sample reference; it would be a new measurement procedure requiring all keys released, not restoration of a known factory baseline.

Static assignment preservation is strong because neither D5 nor its identified persistence path enters keymap/RGB storage. Actual hardware completion, persistence, reset feedback and unchanged assignments still require evidence. No vendor D5 sender or completion acknowledgement was found; do not invent a DA/DE response for it. Required postcondition reads should cover T=FF all 66, R matching retained mode sentinel behavior, B/reference unchanged for D5-only, and full keymap/RGB unchanged. Current app only reads DC mode 1; `crates/niz-device/src/session.rs:218` and `crates/niz-core/src/raw_thresholds.rs` are diagnostic R-only APIs. They cannot currently verify those wider postconditions or back up/restore arbitrary T.

### P1: factory reset and software Default labels do not justify preserving assignments

Original Atom66 XRGB BLE manual lines 132-133 explicitly document factory settings restoration using `ESC + L-ctrl + Backspace + Right` held five seconds and state that the keyboard restarts. This broad operation is distinct from D5 calibration override reset. The live [manufacturer FAQ](https://www.nizkeyboard.com/blogs/news/keyboard-faq-solution) also describes factory restoration as a four-corner physical shortcut for 66/68/75/84 keyboards. Neither source promises preservation of assignments. Do not substitute this shortcut or a guessed D0 command for the requested assignment-preserving operation; exact V1.0.48 normal dispatcher has no D0 branch according to the supplied exact-image evidence.

The model EXE has an English `Default` string at raw `0x1b0244` / VA `0x5b1c44`; there are six code references to it, in UI initialization with rectangle/control IDs and Read/Write labels. Its lower-case profile string `default` at raw `0x1b3a8c` / VA `0x5b548c` is referenced at `0x448523` in an in-memory list-object construction. None of those string references is a demonstrated D5 or analog writer. This review does not claim complete click-handler recovery of every Default control; bounded interpretation is that a generic label cannot establish hardware calibration/reset semantics.

### P2: current calibration package supplies physical measurement commands, no rollback/table/default API

Fresh in-memory download of [manufacturer-linked CalibrationLite distribution](https://www.nizkeyboard.com/pages/firmware-upgrade) repeated the existing package hash `87d1f2ac8f77c99244fa8cffc23c7ef9987aa8b1f4a0a8c41c4e1c8d53dc63ce`. KBDLL has exactly 12 exports: construction/destruction, pipe/open/close, version/discovery, Keylock, CalibrationInit and CalibrationPress. Both calibration methods take no host key/value/table argument. CalibrationLite EXE imports those methods, with no separately identified numeric setter/reset API. Its user-facing strings and instructions explain release-all calibration once, then physically hold one or several abnormal keys for press calibration. They give no depth-to-raw-value mapping, default recovery or rollback.

Windows HWI and calibration DLL imports include only HidD_GetHidGuid for HID plus normal ReadFile/WriteFile transport, no imported HidD_SetFeature/GetFeature, USB vendor/control transfer or custom driver API. Supplied actual descriptor is 39 bytes, Input/Output 64 each, no Feature or Report ID item. Thus no alternate documented Feature-report write domain was established. Absence of imports/descriptors alone does not prove no hidden firmware USB control endpoint exists; no such endpoint was positively recovered in this bounded client review and no probes occurred.

Retained evidence: `KBDLL.dll-pe.txt`, `CalibrationLite.exe-pe.txt`, `calibration-static-metadata.json`, `calibration-instructions.txt`. Original binaries were parsed only in memory and not saved, launched or loaded.

## Practical implementation boundary and tests

- For a D5-only product control, define scope accurately as all per-key calibration overrides reset; bind it to the exact tuple/version, use exclusive device ownership, durable full keymap/RGB evidence and B/R/T snapshots, send once, and report any uncertain-send/readback outcome. Do not offer numeric rollback until an actual analog setter exists.
- A blanket "all threshold and calibration defaults" button should remain blocked unless default meaning is narrowed to D5 scope or a full-baseline/mode recovery path is established. Factory reset plus reapplying assignments creates a broad two-operation failure window and does not meet a preserving-assignment guarantee.
- For a future strict slider, prove selected physical slot mapping, byte/value bounds, unit meaning, exact targeted mutation of T/R/B and persistence, every operating scanner mode, and all other 65 invariants. DD-style full table regeneration can change other R even if stored T differs only at one cell, particularly after a transient D8 scale.
- Add offline transaction tests for exact payload, one-shot send, version/interface/connection generation, no-send cancel, stale/late/wrong reports, exclusive ownership, unresolved-write cleanup, identity change/disconnect, backup preservation and no unrelated opcodes. Synthetic tests validate sequencing only. A hardware qualification must retain complete packets/times, B/R/T before/after, full configuration/RGB equality and controlled reconnect/power-cycle persistence. No such write qualification occurred in this review.

## Verification record

Freshly verified all original artifact hashes, PE imports/exports and bounded host serializers/updater, exact V1.0.48 updater instructions, D5 source bytes, original manual lines and current official sources. `input-hashes.json` records image/manual/model identity. This is a feasibility/plan review, not a working setter, A hardware repair, completed reset or hardware persistence claim.
