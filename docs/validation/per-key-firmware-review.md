# Independent exact-target firmware review

2026-10-11. Reviewer scope: read-only verification of `/private/tmp/niz-perkey-20261011/firmware/per-key-firmware-report.md`, focusing on normal USB HID dispatch, DB/DD calibration scope and persistence calls, D8 scale scope, DC readback semantics, and truthful selected-key UI claims. Native host is `devonlys-MacBook-Pro.local`; workspace `/Users/devonly/Developer/personal/NIZ` remains readable. No repository/Git changes, HID requests, firmware/native GUI execution, reset/update/flash, or speculative probes occurred. This review file is the only new output of the review assignment.

## Verdict

**Core conclusions verified. One P2 diagnostic packet-schema finding was raised and is now resolved in the author report. No unresolved material finding remains for the proposed explicitly unsupported selected-key UI.** The firmware report supports saying that no host-addressed per-key numeric setter or key mask is exposed by the recovered normal HID dispatcher. It does not support an enabled key-only threshold control, and it does not prove absolute impossibility through every alternate interface, malformed request or firmware modification.

The hardware goal remains unfulfilled. A selected-key drawing plus DB/DD holding guidance, D8 scale control, or DC raw-value display would not satisfy the user's strict selected-key-only adjustment requirement.

## Independent method and identity

The 42,396-byte input image `/private/tmp/niz-ui-reference-20261011/target-v1.0.48-private-image.bin` was independently hashed as `b82e11891dfdc138780a81ac635697a7edb0ca1d4e123d6ad38c81a7ba55bf17` before decoding. The report's exact version is consistent with this image.

The author's `analyze.py` and `make_evidence.py` were read before deciding how to use their analysis environment. Neither helper was executed during this review: `make_evidence.py` writes outputs, while this assignment required read-only verification. Independent inline Python commands used only file reads, Capstone instruction decoding, literal/table extraction, and arithmetic calculations, with `PYTHONDONTWRITEBYTECODE=1`. No ARM instruction was executed.

The existing private ELF wrapper was independently parsed: `.text` is at address0, is 42,396 bytes, is byte-for-byte identical to the input image, and has the same SHA-256. Native Apple LLVM21 `objdump --triple=thumbv6m-none-eabi` independently confirmed critical DD and DC branches on that verified wrapper. The report's disassembly/evidence files were read as references, but findings were checked against fresh decoding of the actual image bytes.

## P2 finding: DF retains the last DC row rather than zero-padding it

**Original affected reference:** `per-key-firmware-report.md:173` described the additional mode0 frame as `00 DF reference_hi reference_lo [zero padding]`. That would allow an incorrect future parser/schema to reject a normal reference frame or misidentify its tail.

The buffer is cleared once at `0x6712..0x6716`, before the row loop. For each row, `0x676e..0x6770` stores six values at response offsets4..9. After row11, the DF construction at `0x6792..0x67a0` replaces only response offsets0..3; it does not clear the six row values. Both the last DC row and DF send use the same buffer at `sp+4`, via `0x677c` and `0x67a4` respectively.

Exact source-image bytes independently inspected:

| Address | Bytes | Relevant effect |
|---:|---|---|
| `0x6712` | `40 21` | Response clear length64. |
| `0x6714` | `01 a8` | Response pointer `sp+4`. |
| `0x6716` | `f9 f7 54 fd` | Clear response via `0x1c2`. |
| `0x676e` | `6a 18` | `r2 = response + column`. |
| `0x6770` | `10 71` | Write value to `response[column+4]`. |
| `0x6792` | `10 71` | Write zero to response0. |
| `0x6796` | `50 71` | Write DF to response1. |
| `0x679e` | `91 71` | Write reference high byte to response2. |
| `0x67a0` | `d0 71` | Write reference low byte to response3. |
| `0x67a4` | `01 a8` | Reuse response `sp+4`. |
| `0x67a6` | `01 f0 97 ff` | Send the reused64-byte response via `0x86d8`. |

The send routine itself does not clear or modify this source buffer. At `0x8706` it reloads the saved source pointer, `0x8708` calls copy helper `0x7a70`, and that helper loads bytes from source `r1` and stores to destination `r0` (`0x7a72..0x7a76`). Thus the retained bytes are not merely an artifact of overlooking sender behavior.

Correct static mode0 DF frame:

```text
00 DF reference_hi reference_lo B[row11,col0] ... B[row11,col5] zeros[10..63]
```

**Resolution verified:** after the coordinator relayed the independent finding, the author corrected the report at lines173–181. It now explains the reused buffer, documents row11 values in offsets4..9, and requires DF when mode0 reference data is requested. Fresh rereading verified this correction against the same actual bytes. No DC parser or HID code is being shipped in this turn, so there was no product parser to repair.

## Verified calibration and scale claims

**DB replaces the entire baseline.** At `0x31fa`, DB calls full-matrix averaging `0x5ab8`; the sampler loops6 columns and11 rows (`0x5b54..0x5b5e`) with50 samples each. DB calls baseline flash writer `0x88fc` at `0x3206`. Its subsequent RAM copy also loops6 x11 (`0x3240..0x324a`), storing each sample at B=`0x20001006`, substituting16 for samples>=128. No host key identity/value is read by this routine. Therefore DB cannot be presented as preserving other keys' baseline or response.

**DB persistence is an attempted full-block flash write, not an acknowledgment guarantee.** Writer `0x88fc` computes base `0x43<<10 = 0x10c00`, calls erase primitive `0x36c8`, then writes4-byte words at offsets0,4,…,64 (`0x892a..0x8930`, limit66), covering68 bytes with66 meaningful cell bytes. It writes the separate reference word at `0x10c44`. The caller does not use an erase/program status to decide whether to send DA. The report appropriately separates this path from proof that the keyboard's flash persisted successfully.

**DD updates all qualifying cells, not an addressed key.** The decoded loop at `0x3286..0x32f4` obtains B per cell, computes positive sample-minus-B, and records the row sum. At `0x32c0..0x32d4`, x>10 receives5% of the other row deltas; at `0x32d8..0x32e4`, a resulting byte>=16 overwrites T. The destination is B+`0xc6`=`0x200010cc`. All6 columns and11 rows are visited. No report pointer, physical-key number, mask, threshold parameter or ordinary keydown state enters this decision.

Independent arithmetic agrees with the report's counterexample: `[120,11,0,0,0,0]` becomes `[120,17,0,0,0,0]`, so two cells qualify. A second example `[100,12,11,0,0,0]` becomes `[101,17,16,0,0,0]`, qualifying three. These are code-level possible inputs, not claims of observed device coupling. The favorable `[120,0,0,0,0,0]` case updates one cell, but it does not provide a software-selected-key guarantee.

DD calls T flash writer `0x8954` at `0x32fa`; that writer computes `0x87<<9 = 0x10e00`, erases and writes the68-byte block. DD then calls `0x7624` at `0x32fe`, regenerating all R cells from the global mode. Therefore even unchanged T cells can acquire different live R after a prior temporary D8 scale. DE is completion-path feedback, not independent flash-success evidence.

**D8 is a full-board numeric scale.** Dispatch `0x5e88` reads payload byte2 and calls `0x754c`. Range check `value-10 <=30` uses an unsigned higher branch, accepting10..40. The routine loops6 x11, taking T at `0x200010cc` and storing R at T-`0x42`=`0x2000108a`. Its ordinary-cell formula is `max(9, floor(T*value/40))`; sentinelFF uses10+the mode table `02 04 08` at `0x9b02`. There is no cell/key selector or scale-input persistence store in this path. The row helper `0x87d8` writes hardware control state, not a saved per-key numeric scale.

`0x7624` selects20/24/28 for global modes0/1/2. Function action149's target `0x4718` cycles the shared mode at `0x20000959`, persists it to serial EEPROM address6, and regenerates all R. This confirms the vendor action's global scope despite assignment to a selectable physical position. D5 independently clears all66 T cells toFF and persists the whole block.

## Verified DC and dispatcher boundary

DC dispatch reads mode only at `0x5e76`, rejects mode>=3, then calls `0x670c`. The function selects B=`0x20001006` for0, R=B+`0x84`=`0x2000108a` for1, and T=B+`0xc6`=`0x200010cc` for2. Its response layout is `00 DC row mode values[6]`; row ranges1..11. This is static support for full-table diagnostic readback, not a live travel stream, exact physical units, or a hardware-qualified implementation. It pauses scanning and restores scan enable and shared compensation state. DC returns RAM B/R/T; it does not directly verify flash contents or flash-write success.

The recovered physical-number table at `0x9dc6` has66 bytes, independently checked as a permutation1..66. The scanner uses that table at `0x49cc..0x49d4`. A selected physical-key diagnostic can select the returned cell after full-table readback; the key selection does not alter what DC requests.

Fresh decoding of `0x5c84` confirms the byte1 opcode convention and explicit lower-command compare tree. The switch helper `0x89d8` obtains its count and one-byte offset table from LR, uses default index for out-of-range values, doubles the offset and branches. Independently decoded E0..E5 and F0..FF targets agree with the report's tables. D0 falls through the default exit; no transplanted newer-image D0 capability is present here.

The known analog-changing branches are D5, D8, DB, and DD. F0's addressed record writes are keymap/macro configuration; E0's data path is lighting storage; FA..FF feed module-command/control states at `0x90c4..0x9144`. None of these normal paths establishes a legitimate parameterized key-threshold setter. No malformed-request exploit or alternate-interface completeness claim is needed to support the report's bounded normal-dispatcher result.

## UI claim and acceptance boundary

- Supported claim: the exact-image normal host API presently provides no proven software-addressed selected-key threshold setter; adjustment is unavailable.
- Supported future research claim: raw per-cell diagnostic values have a statically recovered getter candidate, requiring actual device qualification and an accurate nonzero-tail parser before implementation.
- Unsupported claim: selecting one drawn key guarantees DD changes that key only, that DB leaves all other response values untouched, that D8 is key-specific, or that DC/DA/DE proves calibration persistence or a millimeter travel point.
- A truthful unsupported UI is an interim limitation, not achievement of the requested per-key hardware adjustment. The report and coordinator preserve this distinction.

No formal proof excluding all imaginable hidden interfaces or firmware modifications is asserted. Within the requested normal dispatcher, loops, RAM tables and persistence paths, the byte evidence supports the report's central decision. The one concrete packet-schema correction is resolved.
