# Exact V1.0.48 write-path reanalysis

Static-only independent follow-up, 2026-10-11. No HID report, calibration, reset, global-mode action, flash operation, update, vendor executable, or device probe was executed. No repository file or Git state was changed. New outputs are confined to `/private/tmp/niz-write-20261011/firmware`.

## Decision

**A supported selected-physical-key numeric threshold setter, or supported full analog-table writer that can preserve the other 65 cells, is still not established in this exact image.** The normal interrupt OUT dispatcher, the USB class-control ingress registered in all three descriptor configurations, the serial EEPROM block writer, and the updater staging path do not provide that supported API. The evidence does not justify shipping a writable per-key slider or attempting unknown padding/address fields.

**D5 does have a statically demonstrated analog-calibration-only reset scope that preserves saved keymaps.** Its accurate behavior is “clear custom per-key calibration for all keys and use the current trigger mode's defaults.” It sets all 66 meaningful T cells to FF, writes the T storage rooted at MCU flash `0x10e00`, regenerates all R values using the current global mode, and leaves baseline B, stored common reference, the global mode byte, and serial EEPROM keymaps untouched in its reachable call chain. This is a promising exact-version reset capability, not a completed hardware qualification. It does not restore every analog setting to factory defaults and cannot restore a prior arbitrary T table afterward through any proved supported setter.

The task's confirmed DC mode-1 readback is useful for R, but R alone does not identify the prior T sentinel/calibration or baseline state. A readback snapshot can detect changes; it cannot make an unsafe write isolated or supply the missing rollback command.

## Input and verification

- Verified native host: `devonlys-MacBook-Pro.local`.
- Verified workspace: `/Users/devonly/Developer/personal/NIZ`.
- Private binary: `/private/tmp/niz-ui-reference-20261011/target-v1.0.48-private-image.bin`, exactly 42,396 bytes.
- SHA-256: `b82e11891dfdc138780a81ac635697a7edb0ca1d4e123d6ad38c81a7ba55bf17`.
- Version bytes at `0x9f94`: `66EC(XRGB)BLe;V1.0.48;V1.0;`.
- `extract_write_evidence.py` independently checks size/hash, extracts 25 focused ranges, decodes both command switch tables, and enumerates candidate direct BL references at every aligned halfword. Candidate references were checked against focused code; this scan is not represented as a full proof of all reachable ARM control flow.
- Apple LLVM's native Thumb disassembler independently agrees on the D5, analog flash-writer and USB class-control instructions. Its existing private ELF wrapper was checked to contain the exact pinned image at file offset `0x100`.

The earlier per-key report was used to choose areas to reexamine, not as proof of the answer. Instructions and switch data were read from the pinned image again. Device image identity and hardware persistence were not independently qualified in this subtask.

## HID ingress and alternate routes

### Interrupt OUT

USB IRQ `0x7980` reaches `0x347c` at `0x7a24`. `0x347c` takes the buffer/count from controller registers, then calls `0x5c84` at `0x348a`. The aligned direct-reference scan finds only this caller to `0x5c84`.

`0x5c88` advances the original payload pointer by one; `0x5c8a` loads the command. Therefore payload byte 1 is the opcode. The report count arrives in r1, but the dispatcher replaces r1 with a RAM base at `0x5c86`; it does not enforce a report-length check before interpreting individual command fields.

The E0..E5 table at `0x5ccc` and F0..FF table at `0x5ce0` were decoded through the actual `0x89d8` helper semantics. There are no hidden threshold handlers behind ignored cases:

| Route | Destination | Analog significance |
|---|---|---|
| D5 | `0x5ece -> 0x3094` | All T reset; full T flash persistence; all R regeneration |
| D8 | `0x5e88 -> 0x754c` | One scale byte applied across all R cells |
| DB | `0x5e6a -> 0x31e4` | Full baseline sampling/persistence |
| DC | `0x5e76 -> 0x670c` | Readback of one full B/R/T table, selected by mode 0/1/2 |
| DD | `0x5e70 -> 0x3268` | Analog-filter-selected T changes, then whole-table persistence and all R regeneration |
| F0 | `0x5cf2` | Addressed keymap/macro records and serial EEPROM storage |
| F1/F6 | `0x5f72` / `0x5f56` | Keymap transaction/index reset and persistence |
| E0 | `0x5e3a` | Lighting block staging and serial EEPROM address `0x22` |
| FA..FF | `0x5ee6..0x5f52`, consumed at `0x90c4..0x9144` | Peripheral/module command forwarding through `0x4aa4` |
| 3A | `0x5fcc -> 0x70f4` | Firmware-update record staging, excluded from threshold feature |

D0 is not a supported factory reset in this dispatcher. D9 stores only the scan-enable byte. Unknown opcode/padding probing is not justified by a missing setter.

### USB class-control and descriptor alternatives

The USB setup dispatcher at `0x7b3c` resolves its class-request callback from RAM offset `0x2c`, calling it at `0x7b80`. Setup routine `0x08d0` selects one of three descriptor records (`0x9734`, `0x974c`, `0x9764`) from the configured USB option, but every branch loads the same class callback `0x3849` (Thumb entry `0x3848`) and zero protocol callback before `0x091c -> 0x7a90`. The registration stores the class callback at `0x2000015c` and zero at `0x20000160`. No alternate threshold callback is registered by those choices.

`0x3848` copies the fixed eight-byte setup packet and implements HID GET_REPORT/GET_IDLE/GET_PROTOCOL and SET_REPORT/SET_IDLE/SET_PROTOCOL handling. SET_REPORT at `0x38c2` checks report type at setup byte 3:

- Type 2 configures the control OUT length from setup byte 6 at `0x38d6..0x38d8` and increments the pending LED-output counter at `0x38da..0x38de`.
- Type 3 uses the zero-length path at `0x38ea..0x38f0`.
- Other report types return through the rejected/default paths.

USB OUT completion `0x7a12..0x7a16` calls `0x7880` (generic configured receive-buffer copy) and `0x84a0`. The application handler `0x84a0` reads the controller's length and processes data only when it equals 1 (`0x84ae..0x84b0`), then calls the keyboard LED handler `0x4bc8` at `0x84ba`. Neither this class path nor its registered alternatives supplies a host address/value to B, R, T, or analog flash. Generic USB copy helpers are transfer infrastructure, not an exposed arbitrary RAM-address command.

The setup dispatcher also has a vendor callback slot at RAM offset `0x28`, but the observed application registration does not install a separate vendor handler there. A null/unregistered callback or corruption-dependent path is not evidence for a supported setter.

The other command parser at `0x5574`, reached from `0x527e`, handles PS/2 EC..FF protocol commands such as ED LED control and F3 typematic configuration. Its state/response flow does not introduce a selected-key analog write.

## Block writers and host fields

### Serial EEPROM writer is a separate storage domain

`0x3334` divides writes around 128-byte EEPROM pages and calls `0x33d4`. The latter emits serial device opcode A0 at `0x33e8..0x33ea`, then the destination's high and low address bytes at `0x33ee..0x33f8`, followed by the source bytes at `0x33fe..0x3404`; GPIO helpers `0x9400`, `0x9308`, `0x9420` implement the serial transfer. Only the low 16 address bits are transmitted. This path does not program MCU flash addresses `0x10c00` or `0x10e00`. Passing an equal-looking numeric EEPROM address would not access that MCU storage.

F0 payload fields are proved key-record parameters:

| Original payload offset | Meaning |
|---|---|
| 1 | F0 opcode |
| 2 | Keymap group, admitted 1..3 |
| 3 | Physical key number, checked nonzero |
| 4 | Record type, admitted 0..4 |
| 5 | Type-0 length |
| 7 | Type-1 length |
| 9..10 | Type-2/3/4 big-endian length |
| 11..63 | Long-record chunk data, copied in up to 53 bytes |

The direct index stores at `0x5dc2..0x5dc6` / `0x5e10..0x5e14` write two-byte EEPROM record pointers into the group/physical index rooted at `0x20000bd0`. Valid groups 1..3 and physical keys 1..66 address RAM `0x20000bd0..0x20000d5b`. Even extending the nonzero physical byte to 255 addresses no farther than `0x20000ed5`, below B at `0x20001006` and T at `0x200010cc`. These are EEPROM keymap pointers consumed as key-record metadata, not analog cell values.

The EEPROM record destination comes from the transaction cursor at `0x200000a0`, not a host MCU address. F1 initializes this cursor to `0x33a` and clears the three 132-byte index groups; F6 writes the index to EEPROM `0x1ae` with length `0x18c`. Lighting E0 persists length `0xc6` at EEPROM `0x22`. None is a supported analog-table write.

### Unchecked lengths limit any blanket safety claim

F0's long-record loops use the accumulated 16-bit position at `0x20000078` to store chunk bytes at `0x2000018d + position` (`0x5d48..0x5d62` and `0x5d76..0x5d92`). The examined code does not show a staging-buffer-capacity bound before these writes. The simple-record branch also uses host lengths without checking available payload bytes. E0 has a host byte count and an accumulated position before its threshold-triggered persistence/reset.

Consequently, this analysis does **not** prove that arbitrary malformed report sequences cannot corrupt analog RAM, other configuration, or control state. Such an unchecked-buffer path is not a confirmed safe API: it has no selected-cell contract, no safe bounds, no atomic neighbor preservation, no proved rollback or persistence contract, and can overwrite earlier adjacent memory before reaching an analog cell. It must not be turned into a slider implementation or explored by unknown hardware probes. The conclusion is absence of an established supported threshold setter in the traced command interfaces, not mathematical impossibility of any exploit or corruption path.

### Firmware update staging is not a table setter

3A's `0x70f4` decodes a record into a stack buffer through `0x634a`. For decoded type 0 it checks checksum (`0x7168..0x718c`), requires a sequential record address matching `0x200000e4` (`0x718e..0x719e`), and sends record data to serial EEPROM at cursor+4 via `0x71b8 -> 0x3334`. It reads back that EEPROM range and compares sums before advancing the cursor (`0x71bc..0x71ec`). The completion branch can write an update header at EEPROM address 0 and enter reset/update flow (`0x725a..0x727e`). It does not select a B/R/T cell or call the analog table flash writers. Using an update/bootloader operation would violate this task's scope and is not a direct host threshold capability.

Other flash primitive callers `0x2b18`, `0x2b40`, `0x2b5e` operate on configuration addresses `0x00300000` / `0x00300004`; they do not give the normal host dispatcher an arbitrary analog flash destination.

## Analog persistence and D5 reset proof

The meaningful analog tables contain 66 bytes:

| RAM | Table | Persistence/regeneration |
|---|---|---|
| `0x20001006` | B baseline | Flash `0x10c00`, DB writer `0x88fc`; common reference separately at `0x10c44` |
| `0x20001048` | Baseline-derived alternate-scan table | Startup computes floor(B*7/10); no direct diagnostic mode for this table |
| `0x2000108a` | R normal-scanner actuation | Regenerated from T and mode/scale; DC mode 1 |
| `0x200010cc` | T calibration | Flash `0x10e00`, writer `0x8954`; DC mode 2 |

The independent direct-call search found exactly `0x3206 -> 0x88fc` for B persistence, and `0x30ca` (D5) plus `0x32fa` (DD) for T persistence in the decoded application image. Both writers accept a RAM pointer internally, but no traced supported host command passes a supplied full analog table to them.

`0x8954` performs erase at `0x10e00` (`0x8966..0x896c`) and writes 4-byte words at offsets 0,4,...,64 (`0x8970..0x8986`). That is 68 programmed bytes holding 66 meaningful T cells plus two adjacent RAM bytes; D5 does not promise FF for the two padding bytes. `0x88fc` similarly writes the B block at `0x10c00` and the reference word at `0x10c44`. Erase-sector size is controller semantics, not proven from these instructions alone. The saved keymaps are on the separate serial EEPROM path.

D5's exact chain is:

1. `0x309a` saves scan-enable at `0x2000095a`; `0x309e` disables it. `0x30a2` clears active compensation `0x200009b0`.
2. `0x30a4 -> 0x506c -> 0x88d0` puts the scan-drive GPIO sequence in its inactive pattern. This helper does not write keymaps or analog storage.
3. `0x30b6` stores FF at T[row*6+column], bounded by column<6 at `0x30bc` and row<11 at `0x30c4`: exactly all 66 meaningful T cells.
4. `0x30ca -> 0x8954` persists the full T block at `0x10e00`.
5. `0x30ce -> 0x7624 -> 0x754c` regenerates R from the current mode at `0x20000959`. Modes 0/1/2 choose scales 20/24/28. When T=FF, `0x759a..0x75a4` ignores that scale and uses `10 + default_table[mode]`; table bytes at `0x9b02` are 02/04/08, so all R cells become respectively 12/14/18.
6. `0x30d2` restores saved scan-enable. `0x30d4..0x30d6` restores configured compensation `0x200009b1` into active compensation, not the previously active value.
7. Dispatcher `0x5ed2..0x5ede` toggles feedback bit `0x20` in `0x200009b8`, calls the GPIO feedback helper `0x5038` (also before/after a delay), and exits through `0x87d8`.

There is no B/reference write, mode change, EEPROM write, F1/F6 keymap transaction or keymap-default-reset `0x4bf0` call in that D5 chain. `0x4bf0` is a distinct defaults routine that writes EEPROM and reconstructs keymaps; it is not reached by D5.

D5 does not produce a dedicated DA/DE-style persistence acknowledgment in the examined routine. Erase failure status is ignored by `0x8954`; the program primitive does not supply a checked success result at each caller. GPIO feedback or command delivery therefore cannot prove successful flash persistence. Both flash controller wait loops (`0x36d8..0x36dc`, `0x372e..0x3732`) lack a secondary software timeout. A host timeout is not proof that reset completed or was canceled.

Startup reloads B/reference through `0x47e4 -> 0x575c`, and T through `0x47ea -> 0x57b0`. It normalizes B>=128 to16, T<16 toFF, computes the alternate table, loads global mode from EEPROM6 (invalid mode defaults to1), then regenerates R at `0x4876`. T=FF from a successful D5 remains a default sentinel after restart. The mode and baseline remain those already stored, so a persistently difficult A key caused by baseline/reference, sensor behavior, or alternate-scan mode is not guaranteed to be fixed by D5.

## Preservation, restoration and qualification limits

- **Selected key only, 65 others unchanged:** not implementable with the supported write paths established here. DD has no host-selected key or value; it examines all 66 sampled cells and can update multiple T cells. DB changes the whole baseline. D8 applies across the board. F0 is keymap data, not a threshold address.
- **All-table write with preserved neighbors:** the internal flash routine could persist any RAM table only if a supported host route first populated that table. That route has not been found. The mere existence of a pointer-taking internal routine is insufficient.
- **Restoring all 65 arbitrary prior cells:** DC can reveal B/R/T, but no proved supported setter can restore those arbitrary bytes. DD cannot be treated as an exact restoration primitive; its analog filter and R regeneration also depend on physical readings/global mode.
- **All-key custom-calibration reset while preserving saved keymaps:** D5 has the demonstrated static scope. A correct user-facing label should preserve its “current mode defaults” meaning and not imply baseline/reference/global-mode factory reset.
- **Reset readback:** a future explicitly scoped qualification can verify all T=FF and mode-appropriate uniform R, compare B/reference and saved keymaps, and verify persistence after reconnect/restart. DC mode1 alone verifies the R part; other table modes and reset/restart are separate commands from the already-qualified mode1 getter. This subtask sent none of them.
- **Every operating mode:** normal scanner `0x4938` consumes R, but main state4 reaches alternate scanner `0x69a4` at `0x91cc`; the latter uses the baseline-derived table with fixed press/release thresholds6/4. Neither DC mode1 nor D5 proves the active operating mode or adjusts that alternate scanner's thresholds.

Confidence is high for the concrete command mappings, data stores and D5 scope on the pinned image; limited for universal absence of hidden/indirect/corruption paths and for actual device identity, flash timing/success, sensor response, or hardware rollback. This is enough to keep the per-key write gate closed and to specify a narrow D5 reset candidate honestly. It is not sufficient to claim a selected-key numeric feature or a successful reset.

## Retained files

`extract_write_evidence.py` and `write-evidence.json` contain reproducible extraction, direct references, exact switch tables, instruction bytes, table/default arithmetic and source identity. `01-*.txt` through `25-*.txt` provide focused annotated disassembly. `llvm-D5.txt`, `llvm-analog-writers.txt`, and `llvm-class-control.txt` retain independent decoding of the significant new conclusions. These private outputs contain no executed device traffic and are not distribution artifacts.
