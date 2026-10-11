# Independent proposal review

Read-only review of `/Users/devonly/Developer/personal/NIZ/docs/reverse-engineering/2026-10-11-threshold-write-feasibility.md`, observed SHA-256 `b62a8d8095429a6db14354c73703c339143f57ebcd04c7ef7d17b2846f87ee63`, 2026-10-11. Host/workspace had already been independently verified as `devonlys-MacBook-Pro.local` and `/Users/devonly/Developer/personal/NIZ`.

This reviews a next-step proposal, not an implementation, execution approval, flashing plan approval, or actual hardware qualification. No other new reviewer's feedback was read. No repository edits, Git commands, product/HID/ARM/firmware execution, reset, calibration, update or flash operation occurred. Native read-only commands and bounded arithmetic checks were used; this report is the only new initial-review output.

## Verdict

The current capability findings and D5 wording are accurate. The proposal correctly keeps the standard-firmware per-key write gate closed and treats firmware changes as future work. It is suitable for discussing whether **firmware feasibility/design research** should enter scope after the following three P2 clarifications. None of these findings implies permission to implement or execute firmware changes now. No P0/P1 defect is asserted in a functioning implementation, because no such implementation is being reviewed.

## Material findings

### P2 — Separate offline fault injection from physical persistence/recovery qualification

**Path:** `docs/reverse-engineering/2026-10-11-threshold-write-feasibility.md:29`, related `:33`.

Line29 groups power loss and restoration to standard code into the offline validation step. Offline fault injection can check a designed state machine or flash model; it cannot establish actual controller erase/program behavior, physical power-loss behavior, bootloader fallback, or USB recovery. The existing analog writer `0x8954` erases the T storage at `0x10e00` (`0x8966..0x896c`), then writes the entire 68-byte block one word at a time (`0x8970..0x8986`). It has no transaction generation, CRC, duplicate-copy commit or checked persistence success in the traced path. Interrupted erase/program can therefore invalidate or reset unselected calibration values even if a successful new API intended to change one cell. A pre-write host snapshot does not make that storage transaction atomic.

Before claiming the other65 cells survive failure, define what preservation means on success, uncertain delivery, restart and interrupted persistence; establish an appropriate storage failure/recovery strategy and verified memory layout. Do not assume an unused spare flash page or reuse the B page at `0x10c00`, because B preservation is an acceptance criterion and physical flash capacity/erase granularity remain unverified. If a safe failure strategy cannot be established, the strict preservation guarantee must remain unavailable.

Line33 already correctly gates actual firmware writing on recovery/bootloader evidence. Keep that gate and split line29 into offline fault injection first, then separately authorized hardware persistence/recovery qualification after the recovery prerequisites. Suggested scope text:

> オフラインでは途中失敗・再送・電源断を模した検証を行う。これは実機の保存・起動・復旧の確認には数えない。保存中の電源断で他65キーの校正値を失わない設計とメモリ構成を確認し、実機の保存・復旧検証は復旧方法を確立した後の別段階とする。

### P2 — Make preservation of live R explicit; T-only preservation plus global regeneration is insufficient

**Path:** `docs/reverse-engineering/2026-10-11-threshold-write-feasibility.md:28-29`, requirement `:5`.

The plan says it will define the T/R transform and compare the other65 keys, but the implementation contract should explicitly forbid assuming that a one-cell T update followed by existing global regeneration preserves all other live R values. `0x7624` reads the stored/current global mode and calls `0x754c` with coefficient20/24/28. `0x754c` regenerates all66 R cells. A prior D8 can have applied a different temporary scale without changing that stored mode. Concrete instruction-derived counterexample: mode1, unselected T=40, current temporary D8 scale28 gives R28; reusing `0x7624` gives R24 for that unselected cell. This violates the main requirement even though its T byte did not change.

The new API must define and verify preservation of both T and the currently effective normal-scanner R for all65 neighbors, including pre-existing temporary D8 state. It must also define whether “preserved” refers to immediate post-write values, values after restart under the stored mode, or both. Any eventual use of an all-cell helper must explicitly restore/prove the neighbor invariant while scanning is controlled; do not infer the invariant merely from unchanged T.

The existing representation has concrete numeric limits and quantization: startup `0x4812..0x4818` converts T<16 to the FF default sentinel; T=255 is reserved; ordinary T is16..254. R is `max(9, floor(T*coefficient/40))`, giving mode0 R9..127, mode1 R9..152, and mode2 R11..177. These ranges were independently checked over every ordinary T value. The same displayed R can map to multiple T values, and the FF default path gives12/14/18 separately. A numeric slider must either expose clearly scoped raw units and legal representable values, or introduce a new explicit persistent representation/override. It cannot assume an unrestricted raw R value will survive restart through the old T loader.

A useful concise addition is:

> 他65キーは保存値Tだけでなく、現在のしきい値Rも保持する。既存の全体再計算をそのまま使わず、一時的な全体スケール・FF既定扱い・丸め・表示範囲・再起動後の値を仕様に含める。

The existing “other scan modes/Bluetooth must not be assumed” wording at line28 is correct. Main state4 uses `0x69a4` with baseline-derived values and fixed thresholds6/4, rather than R. Changing only the normal R path cannot be described as adjusting every operating mode or proving the A symptom is fixed.

### P2 — Start firmware work with a feasibility gate for build/patch and recovery, not an assumed rebuildable baseline

**Path:** `docs/reverse-engineering/2026-10-11-threshold-write-feasibility.md:25-27`, related `:33`.

“Use exact V1.0.48 as the baseline” is correct, but a reader choosing firmware development should know that a validated decoded image is not a verified exact-version source rebuild. The original package investigation found binaries and no source (`docs/reverse-engineering/atom66-findings.md:7`). This repository's native file inventory found no C/assembly/linker/Keil project files for a matching firmware rebuild. The existing decoder verification proves byte-exact package round-trip and Thumb image plausibility, while explicitly leaving the MCU family/implementation unidentified (`docs/validation/ui-firmware-feasibility.md:13-16`). None of that proves toolchain/linker placement, flash capacity, interrupt/vector behavior after modification, package acceptance by the actual bootloader, or restoration after a failed update.

Make the first firmware phase identify the real MCU and memory/erase layout and choose a verified exact-source rebuild route or an explicitly bounded binary-patch route. A patch route needs reviewed hook/code placement, preserved vectors/literal/data addresses, image/package integrity checks and compatibility identification; replacing the complete firmware needs substantially broader driver and keyboard-behavior qualification. Neither route should be silently substituted for the other after the user chooses this option.

The application image and its `3A` updater staging code do not expose all unseen bootloader/LDROM behavior or prove an external hardware recovery path. The original-code restoration test cannot be completed only by re-encoding a package. Keep execution gated until recovery is independently established on the intended hardware, or state that firmware implementation is blocked at this feasibility boundary.

Suggested first-phase text:

> まずMCU・メモリ構成・起動／復旧経路を確認し、正確な元版の再ビルドまたは限定したバイナリ改修が成立するか調査する。現時点では元版の再ビルド環境と、更新失敗時の復旧を確認できていない。成立しなければ実装・実機書き換えへ進まない。

## Correct scope statements retained

- D5 at proposal lines15–17 is correctly described as clearing all66 custom T values to FF and regenerating R from the current valid global mode. It does not write the saved keymap EEPROM, B, stored common reference or global mode, and it is not a full factory reset.
- Mode-dependent defaults12/14/18 are supported by `0x759a..0x75a4` and bytes02/04/08 at `0x9b02`. This statement concerns normal-scanner R and valid modes0/1/2.
- D5 completion/persistence remains unqualified; current R-only readback is insufficient. There is no dedicated checked D5 persistence acknowledgment in the traced routine. Before/after T/R/B/reference and saved keymap comparisons are appropriate future evidence, with actual persistence verified separately.
- The proposal correctly refuses to use DD as a selected-key slider surrogate. DD has no host-selected cell/value field and can update multiple cells.
- New-firmware identification, apply-once host behavior, complete readback before confirming UI values, and explicit uncertain-outcome handling are appropriate requirements. Eventual API design should make command identity/idempotence/concurrency and failure status concrete; the current proposal need not prematurely choose an opcode.
- Line33 correctly avoids treating a host settings backup as a firmware/USB recovery guarantee. Preserve this limitation.

## Evidence checked

Only the proposal, earlier source/extraction provenance, and this reviewer's independently extracted firmware evidence were consulted. Relevant exact instruction ranges remain under `/private/tmp/niz-write-20261011/firmware`: `09-D5.txt`, `10-analog-flash-writers.txt`, `12-analog-startup.txt`, `13-D8-and-mode-regenerator.txt`, `14-mode-regenerator.txt`, `15-firmware-update-staging.txt`, plus `write-path-report.md` and `write-evidence.json`.

The arithmetic sweep confirmed:

```text
coefficient20: ordinary T16..254 -> every R integer9..127
coefficient24: ordinary T16..254 -> every R integer9..152
coefficient28: ordinary T16..254 -> every R integer11..177
unselected T40, mode1, temporary scale28: R28 -> global regeneration R24
```

These are arithmetic checks derived from the exact instructions, not firmware emulation, actual-device tests, or proof of flash failure safety.
