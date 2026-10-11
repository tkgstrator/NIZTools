# Exact Atom66 V1.0.48: per-key threshold feasibility

2026-10-11. Static analysis on the verified native host `devonlys-MacBook-Pro.local`, workspace `/Users/devonly/Developer/personal/NIZ`. All new outputs are under `/private/tmp/niz-perkey-20261011/firmware/`. No repository edits, Git operations, vendor program execution, firmware execution, GUI, HID requests, unknown command probes, reset, update or flash operation occurred.

## Result

**This exact firmware has 66 independent stored analog calibration bytes, and a host-readable per-cell actuation table. Its normal HID dispatcher does not expose a host-addressed per-key numeric threshold setter or key mask.** Selecting a physical key in the macOS utility cannot, by itself, restrict any existing calibration/threshold command to that key. The zero-argument DD command can update only one qualifying analog cell in a favorable physical situation, but that is conditional on analog readings, not enforced by the selected key.

- DB recalibrates and replaces all 66 baseline cells and persists the full baseline block. It can alter other keys' effective actuation even if a subsequent DD updates only one calibrated cell.
- DD considers all 66 cells and writes every cell whose measured/compensated delta reaches its filter. It reads neither a host key number nor a host threshold. Its row compensation can make an unselected cell qualify. It also recalculates the full live actuation table from the stored global mode.
- D5 resets all 66 calibrated cells to the default sentinel and persists the whole table.
- D8 accepts a numeric scale in the range 10..40 but applies it to all 66 live actuation cells. It is not a selected-key setter, and no persistence of its numeric input is present in this path.
- DC is a newly established exact-target **static getter candidate**, with three modes returning baseline, live actuation and stored calibration tables. This establishes a way to design selected-key diagnostic readback. It has not been qualified through the actual keyboard; no DC command was sent.

A strict selected-key-only numeric control remains unsupported by the recovered normal host API. A post-calibration DC comparison can detect unintended changes after they occur; it cannot prevent them or restore arbitrary previous calibration bytes, because no corresponding setter was found. Do not ship a selected-key threshold slider or write button based on DB/DD/D8, nor label physical-holding calibration as host-addressed per-key control.

## Identity and method

Input private decoded image: `/private/tmp/niz-ui-reference-20261011/target-v1.0.48-private-image.bin`, 42,396 bytes, base 0, SHA-256 `b82e11891dfdc138780a81ac635697a7edb0ca1d4e123d6ad38c81a7ba55bf17`. The hash was independently checked in both evidence scripts. String at `0x9f94`: `66EC(XRGB)BLe;V1.0.48;V1.0;`. Initial stack pointer is `0x200027d0`, Thumb reset vector `0x000000f1`. Original extraction validation and package hash remain in the existing image manifest and validator.

Capstone 5.0.9 was installed only into the private analysis `tools/` directory, with no product dependency installation. A script disassembles Thumb code and annotates PC-relative literal loads. Apple LLVM's native `llvm-objdump --triple=thumbv6m-none-eabi` independently agrees on the significant instructions and branch targets. A private ELF analysis wrapper contains the identical image solely to let LLVM decode a raw binary; it is not a modified firmware or flash artifact. Literal pools and Keil byte jump tables are data even where the full linear disassembler renders them as apparent instructions. Conclusions below use branch-followed code and explicitly decoded tables, not arbitrary linear hits.

The pinned public [protocol document](https://raw.githubusercontent.com/cbingb666/niz-firmware/8b92eaa889fc5b8d1aaede2f79af8a1d15f02352/recovered/PROTOCOL.md) was used only as a list of known commands to examine. Its newer-image function addresses were not reused. Every address in this report is from this exact V1.0.48 image.

## Exact HID dispatcher

USB OUT handler `0x347c` obtains the receive buffer from registers rooted at `0x40060500`/USB memory base `0x40060100`, and calls `0x5c84` at `0x348a`. At `0x5c88`, the dispatcher advances the buffer by one byte; at `0x5c8a`, it reads the command byte. Thus report payload byte 1 is the opcode; following `[r0,#1]` reads payload byte 2. The caller supplies a length, but the dispatcher does not use it as an address or mask. The prefix byte is not used to select a key.

The lower opcodes use explicit compares. E0..E5 and F0..FF use the compiler switch helper at `0x89d8`. That helper reads a count byte at the return address, selects a one-byte relative offset, multiplies it by two, and branches. The exact tables are:

- At `0x5ccc`: `06 b7 cd f0 ef 6b ee 6b`; branch base is `0x5ccd`.
- At `0x5ce0`: `10 09 f0 ef 61 61 e4 ee 61 61 ed ec eb ea e9 e8 e7 61`; branch base is `0x5ce1`.

| Opcode | Exact dispatch path | Payload parameters | Material semantics |
|---|---|---|---|
| 3A | `0x5d9e -> 0x5fcc -> 0x70f4` | Firmware record data | Existing update path; excluded from this task |
| D5 | `0x5da0 -> 0x5ece -> 0x3094` | None used for calibration reset | Resets all calibrated cells, then shared-state feedback |
| D8 | `0x5cb8 -> 0x5e88 -> 0x754c` | Byte 2 numeric scale | Recalculates every live threshold; then shared-state feedback |
| D9 | `0x5d9c -> 0x5e82` | Byte 2 | Stores scan enable at `0x2000095a` |
| DB | `0x5da4 -> 0x5e6a -> 0x31e4` | No key/value/mask read | Whole-matrix baseline calibration |
| DC | `0x5cc2 -> 0x5e76 -> 0x670c` | Byte 2 mode, 0..2 | Sends all 11 rows of one analog table |
| DD | `0x5d9a -> 0x5e70 -> 0x3268` | No key/value/mask read | Conditional analog-cell calibration across the matrix |
| E0 | `0x5e3a` | Count and lighting bytes | Lighting data written to serial EEPROM, not analog thresholds |
| E1 | `0x5e66` | None | Resets lighting receive position |
| E2/E3 | `0x5eac/0x5eaa -> 0x5fba/0x5fc2` | None | Lighting/count read states 2/3 |
| E4 | `0x5da2` | None | Default/ignored branch |
| E5 | `0x5ea8 -> 0x5fc8` | None | Clears state byte `0x20000063` |
| E6 | `0x5d98 -> 0x5fb0` | None | Lighting/count completion handlers |
| F0 | `0x5cf2` | Group, physical key, type, type-specific record | Keymap/macro records; addressed writes enter serial EEPROM through `0x3334` |
| F1 | `0x5ec0 -> 0x5f72` | None | Begins keymap transaction, clears 3 x 132-byte record index |
| F2 | `0x5ebe -> 0x5fa6` | None | Starts keymap read state 1 |
| F3/F4/F7/F8 | `0x5da2` | None | Default/ignored branches |
| F5 | `0x5ea8 -> 0x5fc8` | None | Clears the same state byte as E5 |
| F6 | `0x5ebc -> 0x5f56` | None | Persists keymap index and refreshes key record metadata |
| F9 | `0x5eba -> 0x5ee0 -> 0x68d4` | None | Sends `00 F9` plus string from `0x3840 -> 0x9f94` |
| FA/FB | `0x5eb8/0x5eb6` | None | Module command states 1/2 |
| FC | `0x5eb4 -> 0x5ef0` | Count capped at 20, bytes | Module command state 3 and temporary buffer `0x20000ad4` |
| FD/FE | `0x5eb2/0x5eb0 -> 0x5f22/0x5f3a` | Two bytes | Module command states 4/5 and same buffer |
| FF | `0x5eae -> 0x5f52` | None | Module command state 6 |

No D0 factory-reset branch from the public newer protocol is present in this dispatcher. For example, a D0 byte reaches the default exit after failing D5/D8 equality. This is one concrete reason not to transplant protocol capability by version/name.

The FA..FF states are consumed at `0x90c4..0x9144`. States 1..5 construct module commands F9/F8/FA/FB/FC and send via `0x4aa4`; state 6 follows a separate control path. These branches do not take a scan-cell address or write any of the analog tables. F0's host physical-key address is real, but its records, storage and event processing are keymap/macro configuration; it must not be relabeled a threshold address.

Representative raw instruction bytes:

```text
5c88 40 1c       adds r0,r0,#1
5c8a 02 78       ldrb r2,[r0]
5cba db 2a       cmp r2,#0xdb
5e6a fd f7 bb f9 bl 0x31e4
5e70 fd f7 fa f9 bl 0x3268
5e76 40 78       ldrb r0,[r0,#1]
5e78 03 28       cmp r0,#3
5e7a 01 d2       bhs 0x5e80
5e7c 00 f0 46 fc bl 0x670c
5e82 40 78       ldrb r0,[r0,#1]
5e84 88 71       strb r0,[r1,#6]
5e88 40 78       ldrb r0,[r0,#1]
5e8a 01 f0 5f fb bl 0x754c
5ece fd f7 e1 f8 bl 0x3094
```

## Analog layout and units

All relevant loops are 11 rows x 6 columns. These addresses are inferred from literal loads, loop limits and subsequent scanner use:

| RAM address | Cells | Meaning and evidence |
|---|---:|---|
| `0x20001006` | 66 | Released/baseline signal B, DB replacement and normal-scanner subtraction |
| `0x20001048` | 66 | At startup, floor(B x 7/10); alternate scanner uses it with 6/4 thresholds |
| `0x2000108a` | 66 | Live actuation byte R, regenerated from T and global mode/scale |
| `0x200010cc` | 66 | Calibrated byte T, DD conditional updates; FF means default sentinel |
| `0x20000959` | 1 | Stored global trigger mode, loaded from serial EEPROM address 6 |
| `0x2000095a` | 1 | Scan-enable byte, D9 and temporary pausing |
| `0x20000996` | 2 | Stored common/reference ADC sample loaded at startup |
| `0x20000998` | 2 | Startup-adjusted ADC reference for scan acquisition |

The average-matrix routine `0x5ab8` selects every row through `0x50e0`, takes 50 samples per cell from ADC helper `0x5a58`, transforms each sample to `uint8(0x303 - ADC)` when ADC < 0x303, otherwise 0, and stores floor(sum/50). The ADC helper masks a register to 10 bits at `0x5aa0..0x5aa2`. These table bytes are raw firmware signal/calibration units derived from ADC counts. No physical millimeter, force, gram or distance conversion is established. D8's coefficient is a numeric scale, not a measured distance.

Firmware event creation at `0x49cc..0x49d4` translates each flat matrix index through the 66-byte table at `0x9dc6`. Its values are a permutation of physical key numbers 1..66, verified by the evidence script. This maps a selected physical key number to a DC table cell. The complete mapping is retained in `physical-key-to-analog-cell.csv`. For example, physical key 1 is row 1/column 0, key 12 is row 5/column 0, key 13 is row 11/column 0. Mapping these numbers onto drawn geometry remains the app's already-established physical-layout responsibility.

## DB changes every baseline

Routine `0x31e4` saves scan enable, writes 0 to it, samples every matrix cell into a stack buffer via `0x5ab8`, and samples a common ADC reference via `0x5c14` (50-sample average). It invokes `0x88fc` with the full matrix and reference at `0x3206`.

The flash writer `0x88fc` erases storage rooted at absolute `0x10c00`, then writes matrix data in 4-byte words for offsets 0..64 inclusive, i.e. a 68-byte block holding 66 meaningful cells. It stores the separate reference word at `0x10c44`. Memory-mapped controller primitives `0x36c8`/`0x371c` carry erase/write operations; this is separate from the serial EEPROM keymap transport. The writer does not inspect a selected key. Return/error status is not checked by DB, so the DA message is not proof of successful persistence.

DB sends a 64-byte acknowledgment starting `00 DA DA` at `0x320a..0x3218`, then copies every sampled cell into B at `0x20001006`. Sample values >=128 are replaced by 16 in the RAM copy (`0x3232..0x323a`). It restores scan enable at `0x3254`.

The normal scanner at `0x4982..0x4992` explicitly subtracts B[cell] from that cell's current acquired signal, saturating at zero. Thus replacing B for the entire board changes the effective signal used for all keys. The alternate scanner also derives from baseline material. **Even if a later DD persists one T cell, a preceding DB cannot be claimed to preserve all other keys' thresholds/response.** DB's new common reference is written to flash but its live RAM reference is not updated by this routine; startup reload/normalization follows a separate path.

## DD is an analog filter, not a key selector

Routine `0x3268` saves and disables scan enable, samples all 66 cells, then processes each row:

```text
sample[j] = full-matrix average from 0x5ab8
original_delta[j] = max(sample[j] - B[j], 0)     // stored as a byte
row_sum = sum(original_delta[0..5])
for j = 0..5:
    x = original_delta[j]
    if x > 10:
        x = uint8(x + floor((row_sum - x) * 5 / 100))
    if x >= 16:
        T[j] = x
    else:
        leave T[j] unchanged
```

Evidence: subtraction/saturation at `0x329c..0x32aa`, row sum at `0x32ac..0x32b4`; x >10 filter at `0x32c0..0x32c2`; 5% compensation at `0x32c4..0x32d4`; x >=16 and T store at `0x32d6..0x32e4`. Bytes at `0x32d8` are `10 28` (`cmp r0,#0x10`), and `0x32e4` is `c8 55` (`strb r0,[r1,r7]`). The destination base is B+0xc6 = `0x200010cc`. There is no reference to host report data, a physical-key number or a mask anywhere in this routine.

A concrete arithmetic counterexample to guaranteed isolation is a row with released/pressed deltas `[120,11,0,0,0,0]`. The first cell may be the intended held key; the second may be an unselected residual/coupled/noisy cell. The function transforms this to `[120,17,0,0,0,0]` and stores **both** first and second T cells. This example is derived from the actual instructions and independently checked by the evidence script; it is not a claim that this physical board currently exhibits those readings. It proves that the algorithm does not enforce a selected-key invariant.

DD calls `0x8954` at `0x32fa`: it erases storage `0x10e00` and writes all 68 bytes covering the 66 meaningful T cells. Cells not qualifying the analog filter retain their prior RAM value before this full-block persistence. Then DD calls `0x7624` at `0x32fe`, which regenerates **all** live R cells from the stored global mode. If a prior D8 had applied a different temporary scale, this regeneration can alter other cells' live R even when their T did not change. DD sends `00 DE DE` (`0x3302..0x3312`) and restores scan enable. As with DB, acknowledgment does not independently verify erase/program success.

DD does not consult normal scanner held bits or HID keydown state. Describing it as 'held-key calibration' is a reasonable physical workflow interpretation of the analog filter, but describing it as guaranteed only the selected/held key is stronger than the code supports.

## D5 and D8 are global

D5's `0x3094` disables scanning, resets key-output/scan state through `0x506c`, loops 11 x 6 writing FF to every T cell, persists the full T block using `0x8954`, regenerates all live thresholds with `0x7624`, and restores scanning. It does not read any report parameter. The dispatcher additionally toggles bit 0x20 in a shared state/feedback byte `0x200009b8` via `0x5038`; the precise UI feedback meaning is not needed for the threshold conclusion.

D8's `0x754c` checks `(uint32(value)-10) <=30`, giving an accepted scale of 10..40. For each of 66 cells, with ordinary T:

```text
R[cell] = max(9, floor(T[cell] * value / 40))
```

With T=FF, it uses `10 + default_table[global_mode]`, where the exact bytes at `0x9b02` are `02 04 08`, giving mode fallbacks 12/14/18. The full board is always traversed, and no key selector is read. Byte 2 is the only host field used by this analog path. Values outside 10..40 return without changing R, but the dispatcher's subsequent shared-state toggle still happens; they are not a safe unknown-command probe.

`0x7624` chooses D8-style coefficients 20/24/28 for global modes 0/1/2 and calls `0x754c`. The key-action handler compares action `0x95` (149) at `0x4186` and calls `0x4718` at `0x41aa`. That routine cycles the global mode and persists its byte to EEPROM address 6. This demonstrates why an assignable trigger action does not create per-key analog control.

Normal scanner `0x4938`, called from main at `0x8d22`, computes positive signal-minus-B, applies row compensation through `0x4b94` (5%, or 18% under a shared state), compares it against R, and uses R-3 as release hysteresis, with two-scan confirmation. The alternate scanner `0x69a4`, reached at `0x91cc`, uses a baseline-derived array and fixed thresholds 6/4. Full scanner qualification in every operating mode is outside this bounded static pass, but both paths contradict interpreting DB as an isolated baseline write.

## DC diagnostic readback candidate

Input payload structure, statically proven but **not sent or device-qualified**:

```text
00 DC mode [zero padding to 64 bytes]     mode = 0, 1 or 2
```

`0x5e76..0x5e7c` reads mode only, rejects mode >=3, and calls `0x670c`. That routine zero-initializes a 64-byte response, saves/disables scan enable, temporarily resets scan/key-output state, and emits 11 frames via the same USB send routine `0x86d8` used by F9:

```text
00 DC row mode v0 v1 v2 v3 v4 v5 [zero padding]
      row = 1..11
      mode 0: B at 0x20001006
      mode 1: R at 0x2000108a
      mode 2: T at 0x200010cc
```

Mode 0 additionally sends the reference from `0x20000996`, but **DF does not have an all-zero tail**. The same response buffer is cleared only once at `0x6712..0x6716`. The last DC row stores its six cell values at response offsets 4..9 through `0x676e..0x6770`. DF construction at `0x6792..0x67a0` overwrites only offsets 0..3, leaving offsets 4..9 holding row 11 baseline values. The exact statically established DF layout is:

```text
00 DF reference_hi reference_lo B[row11,col0] ... B[row11,col5] [zeros at 10..63]
```

For reference parsing, consume offsets 2..3 as big-endian and ignore offsets 4..63; never require all bytes after offset 3 to be zero. The residual row bytes are reused buffer contents, not extra reference fields. This correction is independently supported by instruction bytes `6a 18 10 71` at `0x676e..0x6770` and the four stores `10 71`, `50 71`, `91 71`, `d0 71` at `0x6792`, `0x6796`, `0x679e`, `0x67a0`.

There is no E6-style terminator in this function. A parser should require all 11 unique row numbers and the expected mode to complete a 66-cell table; mode 0 additionally requires its DF frame when the caller requests the reference value. No row/key request parameter exists; selecting a key affects only which returned cell the application displays.

The getter performs no analog-table flash write. It does temporarily pause/reset scanner output and at the end restores the scan-enable byte and restores shared compensation state `0x200009b0` from `0x200009b1`. Therefore do not claim it is stateless, streams live key travel, or has zero runtime effects. B/R/T are calibration/actuation values, not live ADC samples. The host transport behavior, permissions, timeouts, reconnect handling and actual-device parsing remain unqualified.

## Minimal correct capability and remaining prerequisites

1. An exact-version gated diagnostic design could display the selected physical key's B/R/T raw values using the statically proven DC layouts and `physical-key-to-analog-cell.csv`. First obtain and validate an explicitly authorized read-only hardware qualification; it has not happened in this pass. Label these raw firmware values, not mm/grams or known current travel.
2. Existing DB/DD can remain clearly named whole-board-baseline / analog-filtered physical calibration operations where that scope is authorized. They cannot satisfy the present selected-key-only numeric request. Do not silently run DB before a purported isolated change.
3. DC snapshots before/after DD could prove which stored values actually changed in a particular completed operation, but fail to guarantee selected-key-only in advance or supply arbitrary rollback. Inspect B, R and T, not T alone. Even a successful one-cell example does not establish a host-addressed setter.
4. A working strict per-key numeric control needs a supported host command accepting a physical key/cell and a numeric threshold with units/readback and isolation semantics, or a separately authorized firmware implementation of that API. Neither is provided by the recovered normal dispatcher, and the present task forbids execution/update/flash work. Normal keyboard event injection cannot create the analog cell reading DD uses.

No unknown report padding was assigned speculative semantics. No calibration label, public newer-image address, action 149 name, DA/DE receipt, or worker completion claim was treated as proof of a selected-key setter.

## Retained evidence

- `per-key-firmware-report.md`: this report.
- `static-evidence.json`: pinned identity, complete bytes for significant ranges, exact switch targets and DD arithmetic counterexample.
- `physical-key-to-analog-cell.csv`: all 66 physical key numbers with exact B/R/T addresses.
- `analyze.py`, `make_evidence.py`: reproducible static decoding/extraction only.
- `dispatcher-disassembly.txt`, `DB-disassembly.txt`, `DD-disassembly.txt`, `D5-disassembly.txt`, `D8-disassembly.txt`, `DC-disassembly.txt`, `sampling-disassembly.txt`, `scan-disassembly.txt`, `flash-writers-disassembly.txt`: focused annotated evidence.
- `linear-disassembly.txt` and `llvm-disassembly.txt`: primary and independent whole-image decoding for reference; data pools/tables must still be interpreted as data.
- `static-analysis-wrapper.elf`: private identical-image wrapper for LLVM, never a firmware delivery or execution artifact.

All output is private temporary research material. No decoded firmware or analysis dependency was copied into the repository or distribution.
