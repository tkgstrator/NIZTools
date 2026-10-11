# Exact V1.0.48 DC mode 1 raw threshold read: feasibility review

2026-10-11. Independent read-only firmware audit on `devonlys-MacBook-Pro.local`, native workspace `/Users/devonly/Developer/personal/NIZ`. No HID, keyboard probe, app UI, vendor program execution, firmware execution, source mutation or Git operation. Only this temporary report was written for this assignment. The proposed host qualification procedure was supplied by the coordinator; actual probe source review is recorded separately below when available.

## Assessment

**The exact firmware provides a concrete DC mode 1 getter for all 66 raw normal-scanner actuation bytes R.** A tightly pinned, explicitly requested qualification read is feasible. The getter does not write B, T or R tables, the global trigger mode, a global scale, keymap data, EEPROM or flash. It does have scanner/compensation/GPIO and USB runtime effects, so it must not be described as having no side effects or as an atomic sensor-travel measurement.

The firmware source is the private image `/private/tmp/niz-ui-reference-20261011/target-v1.0.48-private-image.bin`, size 42396, SHA-256 `b82e11891dfdc138780a81ac635697a7edb0ca1d4e123d6ad38c81a7ba55bf17`. Its exact version at `0x9f94` is `66EC(XRGB)BLe;V1.0.48;V1.0;`. All addresses below are from that image, not the newer public firmware. Existing `analyze.py` checks this image hash before disassembly.

## Command and response bytes

Normalized 64-byte output payload: `00 DC 01` followed by 61 zero bytes. With hidapi's unnumbered output convention, the application writes 65 bytes: leading Report ID 0 plus this 64-byte payload. It must check that the write reports all 65 bytes.

Dispatcher `0x5c84` advances the input pointer by one at `0x5c88`, reads the opcode at `0x5c8a`, selects DC at `0x5cc2 -> 0x5e76`, reads payload byte 2, rejects values >=3, then calls `0x670c`. For mode 1, the data pointer is B base `0x20001006` + `0x84` = R base `0x2000108a` (`0x6744..0x674e`). The mode-1 path at `0x675a..0x676a` uses `ldrb r0,[r6,r1]`; it never stores to the table.

Eleven 64-byte input payloads are emitted in ascending row order:

```text
00 DC row 01 R0 R1 R2 R3 R4 R5 [zeros at offsets 10..63]
row = 1..11
flat_index = (row - 1) * 6 + column       column = 0..5
```

The row header is written at `0x6736..0x6742`; six values are written at `0x676e..0x6770`; the count bound is 6 at `0x6776`; the row bound is 11 at `0x6786`. Mode 1 branches directly to restoration at `0x678c`, so **no DF frame or E6 terminator** belongs to this mode. DF's retained row-11 tail detail applies only to mode 0 and must not be imported into the mode-1 schema.

Require exactly 64 input bytes, prefix `00 DC`, mode byte 1, valid row numbers, each row exactly once, all 66 cells present, and a bounded transaction deadline. The firmware emits ascending rows; a strict first qualification parser can enforce ascending 1..11 to reject duplicates/reordering/stale packets. Do not accept a 65-byte input by guessing/removing a report ID. Keep complete raw input reports for any failed qualification. A valid byte is a raw u8; its interpretation must not be guessed from remapped keyboard outputs.

## Runtime effects and restoration

The following reachable writes are concrete; none is an analog calibration or persistent configuration write:

| Step | Address/path | Effect |
|---|---|---|
| Save enable | `0x671e..0x6720` | Reads `0x2000095a` and saves its exact byte on stack |
| Pause scan | `0x6722` | Writes 0 to `0x2000095a` |
| Pause compensation | `0x6724..0x6728` | Writes 0 to active compensation byte `0x200009b0` |
| Inactivate scan drive | `0x672a -> 0x506c -> 0x88d0` | Shifts three 11-bit all-one patterns and one 6-bit all-one pattern through GPIO rooted at `0x50004280`, with latch/control writes |
| Emit vendor replies | `0x677e -> 0x86d8` | Writes vendor USB IN buffer and endpoint count; changes endpoint-ready byte `0x20000057` and SysTick registers |
| Restore enable | `0x67aa..0x67ae` | Restores the exact saved `0x2000095a` byte |
| Restore configured compensation | `0x67b0..0x67b4` | Copies configured/default byte `0x200009b1` to active byte `0x200009b0` |

**Compensation restoration is not restoration of the prior active byte.** That previous active byte is never saved. If it differed from the configured/default byte before DC, the post-read active compensation state may differ from the pre-read state. R remains unchanged, but normal scan signal compensation could change. Thus “all thresholds/calibration settings are unchanged” is defensible for the B/T/R/global-mode/persistent fields; “all effective scanning state is unchanged” is not.

The `0x506c` helper does not directly send an ordinary USB keyboard key-up/report, clear the held-key bit array, or call the keyboard output functions. The earlier broad phrase “reset key-output state” should be read as scan-drive hardware inactivation here, not proof of a full key-release report. Scanning and main-loop input delivery can be delayed while the vendor read executes. Perform qualification with keys released and no typing, and verify ordinary typing resumes afterward through an observable local check; do not claim a hardware check was performed in this static audit.

After getter return, dispatcher common exit `0x5fd2 -> 0x87d8` services shared protected/watchdog hardware registers. The reachable helper has no threshold/EEPROM/flash write. No D9, calibration/reset helper, global-scale helper or firmware writer is needed in the host read procedure.

## If host reads fail or time out

The command runs inside the USB IRQ path: vector offset `0x9c` points to Thumb handler `0x7981`; handler at `0x7a24` calls `0x347c`, which calls the dispatcher/getter. Host parsing is outside that routine. A host timeout, wrong header or missing packet does **not** cancel or unwind the firmware getter.

`0x86d8` polls USB ready byte `0x20000057`. If it is 0, it invokes delay helper `0x2ab8` with argument 1 and increments a counter. Counter bound `0x7530` = 30000 is loaded at `0x86e2`; reaching it exits the polling loop at `0x86f0`, even if the ready flag never became 1. It then clears that flag, copies all 64 bytes to USB memory via `0x7a70`, writes endpoint count at `0x870c`, and returns. The delay programs and polls SysTick COUNTFLAG (`0x2abe..0x2ad2`). Clock updater `0x6e88` stores computed core-clock Hz H at `0x20000000`, then `0x6e8a..0x6e92` stores C = floor((H + 500000) / 1000000) at `0x20000004`; the literal divisor at `0x6eb0` is `0x000f4240`. Delay(1) loads C at `0x2ab8..0x2abc` and writes that value as SysTick RELOAD at `0x2ac0`, clears CURRENT at `0x2ac4`, and enables core-clock counting at `0x2ac8..0x2acc`. Thus this is an intended one-microsecond delay, with rounding, a reload cycle, polling and call overhead. A maximally blocked report performs exactly 30000 delay calls and still transmits; mode 1 makes eleven sends, at most 330000 delay calls. The programmed waiting time is roughly 30 ms per send, 0.33 s for all eleven, plus loop/call/copy/interrupt overhead. For example, if H = 48 MHz, C = 48 and the counter period C+1 alone contributes approximately 30.625 ms per send and 336.875 ms for eleven maximally blocked sends. This example does not establish the live device clock or measure real elapsed time. **There is no proven hard wall-clock worst-case bound:** the SysTick COUNTFLAG wait has no secondary timeout, and interrupt/hardware behavior can add time. The exact 30000 / 330000 iteration bound is the strongest firmware-only bound under normal working CPU/SysTick.

Therefore ordinary host disappearance is not an explicit infinite wait in the USB-ready polling code: under normally advancing SysTick, each send eventually falls through when its counter expires, and after all eleven send calls the getter restores enable/configured compensation. The USB completion handler writes ready=1 only after the dispatcher call returns (`0x7a28..0x7a34`); the send loop does not synchronously run that handler. A host should budget a generous bounded transaction deadline and preserve timing evidence; it cannot derive a guaranteed cleanup time from receiving the first or last packet alone. However a halted/misconfigured SysTick, processor fault/reset or disconnected hardware state is outside that normal execution proof; the SysTick wait itself has no secondary timeout. Also overwriting/arming an endpoint after the ready poll expires can lose replies. Receiving only some rows does not prove the snapshot is complete or restoration already happened.

**Host cleanup must not invent an unlock.** No host D9 lock was sent, and the original scan-enable byte is unknown to the host. Sending D9=1 as recovery would alter that state and could overlap an in-flight getter. On short write, error, wrong/partial responses or timeout: stop new output commands, preserve captured bytes and delivery uncertainty, retain the existing process lock while bounded passive reads/settling finish if the device remains connected, then close the owned handle. Do not automatically resend DC or issue calibration/reset/flash/global-threshold commands. If typing restoration cannot be observed, report that uncertainty and leave recovery to an explicit follow-up. Closing the host handle is not proof of firmware cancellation or scan restoration.

## What R means and where it is used

R is a firmware comparison byte, not press depth, sensor travel, millimeters, force or grams. ADC helper `0x5a58` returns the low 10 bits (`0x5aa0..0x5aa2`). Normal acquisition `0x3af6..0x3b0a` / `0x3b56..0x3b6c` subtracts the ADC result from live common reference `0x20000998` only if reference > ADC, narrows that difference to u8, then rejects narrowed values >128 as 0. The reference is established at startup (`0x483c..0x4860`), using saved reference and a startup sample; it is not returned by DC mode 1. The normal scanner then subtracts per-cell baseline B with saturation at 0, and may add row compensation from other cells. Thus the exact raw comparison path depends on ADC reference and narrowing/filtering as well as R. Routine `0x4b94` uses 5% or 18% according to active state `0x200009b0`; compensation is added only for a cell delta >10. The adjusted byte is compared to R at `0x4998..0x49a2`. A new pressed state requires two qualifying scans (`0x49c8`). A held state is released below R-3 at `0x499c..0x499e`, `0x49f2..0x49f4`, again with two-scan confirmation (`0x4a1a`). Thus displaying R alone must not imply an independent physical travel calibration or a full response model.

Normal scanner `0x4938` is called from main state 0 (`0x20000062`) at `0x8d22`; it consumes pending scan data. The exact main switch data at `0x8d1a` is `05 04 fd fc fb fa f9`: state 0 -> `0x8d22`; state 4 -> `0x8f0e -> 0x91c4`. In state 4, with scan-enable nonzero, main calls alternate scanner `0x69a4` at `0x91cc`. That scanner uses the baseline-derived array at `0x20001048` with fixed press/release thresholds 6/4, not R. The getter does not expose the current main state in its replies, so it does not establish that R controls every currently active operating mode. UI naming should be “raw normal-scanner threshold R” or a similarly scoped label.

## Exact physical-slot mapping

The firmware table at `0x9dc6..0x9e07` is a verified permutation of physical numbers 1..66 used in normal/alternate scanner event creation. A selected app slot uses immutable physical number `slot + 1`; invert the firmware permutation to select its DC matrix index. Do not treat the 11 firmware rows as the app's five drawn keyboard rows, and do not map from mutable output assignments.

Current app `crates/niz-gui/src/keyboard.rs` builds stable slots 0..65 across visual row counts 15/14/13/13/11 and separates fixed legends from mutable output labels. Source SHA-256 at review: `994f0ce165ae17f927c53fdb19d905d4c15a3b45dcbbbf9661db027259a3a9e5`. The existing static host-software mapping proof remains the source for physical geometry/legends; this read adds the exact firmware scan-cell-to-physical-number mapping. No live physical-key mapping was performed by this worker.

Examples:

| Physical number / app slot | DC row, column | Flat index | R RAM |
|---|---|---:|---|
| 1 / 0 | 1, 0 | 0 | `0x2000108a` |
| 31 / 30 (A in fixed template) | 3, 1 | 13 | `0x20001097` |
| 61 / 60 | 9, 5 | 53 | `0x200010bf` |
| 62 / 61 | 10, 3 | 57 | `0x200010c3` |
| 66 / 65 | 10, 5 | 59 | `0x200010c5` |

Full map: `/private/tmp/niz-perkey-20261011/firmware/physical-key-to-analog-cell.csv`, SHA-256 `699d6aa67d520fb1db406128b02039b5ce46e69aac6c2ac43008cd81bd0361ba`. Its entire permutation is checked by the existing evidence generator.

## Host qualification plan review

The supplied plan is appropriate with the qualifications above:

1. Use the existing per-path process lock, validate the exact opened descriptor/identity, and keep that same handle and connection generation. Current native transport accepts a wider family; DC qualification must explicitly restrict VID0483/PID502a, usage008c/1 and this exact F9 version string. Confirm 64-byte unnumbered input/output descriptor semantics. The lock is cooperative; unrelated vendor clients may not honor it, so a contending client/unsolicited stream is a stop condition.
2. Perform a bounded initial stale-queue drain using the existing native preparation mechanism, logging discarded packets. Then send a fresh F9 and require the exact version, not family substring recognition or a cached baseline. A drain is appropriate before fresh read transactions; never drain away a partially received in-flight DC response to manufacture a clean retry.
3. Construct only F9 and `00 DC 01` output reports. Review the probe's command allowlist and final trace. Do not call calibration helpers or inherit automatic D9 cleanup from the calibration flow. Check full output-write length before parsing replies; a short/error write has uncertain delivery and precludes retry.
4. Collect the complete eleven mode-1 rows under a bounded host deadline, with strict framing/mode/row checks. Preserve exact raw bytes and elapsed times. Wrong, partial, duplicate, malformed, delayed beyond deadline or unexpected replies fail the qualification and prevent a second DC request. Stop new writes; only passive settling/cleanup may follow.
5. Only after a complete successful first read with no extra/queued error may a separate fresh second transaction be issued for stability. Recheck the same descriptor identity/connection and fresh F9, and repeat DC mode 1 under the same lock. Compare all 66 R bytes, not only selected A. Two stable reads qualify reproducible values for this target at those times; they do not prove future immutability, physical units, all-mode usage or zero compensation side effects.
6. Retain useful binary/text captures and hashes. Drop the owned handle and process lock after completion/passive cleanup. Verify keyboard typing resumes separately; report any unresolved restoration uncertainty. No GUI/app implementation or setter is authorized by this static audit.

The existing transport source supports descriptor validation, per-path SHA-256 file lock, bounded queue drain and 65-byte output/64-byte input conventions; `Session::version` provides a fresh F9 path. Its generic recognized-family check is insufficient for this exact-version getter and its generic drain does not retain discarded frames itself, so a qualification probe should log that evidence explicitly.

## Actual probe source review

Pending: the coordinator is preparing a formal read-only Session mode-1 API, CLI raw-threshold read and fixture tests after independent plan review. The existing GUI keeps its native session/path lock after baseline; the CLI must wait for a clean close and acquire the same existing lock normally. This report presently reviews the proposed procedure and current native transport/session code, not an unseen final implementation, its command allowlist, or any hardware result. No claim of actual probe code review or DC hardware qualification is made yet.
