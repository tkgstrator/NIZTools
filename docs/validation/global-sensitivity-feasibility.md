# Stock firmware batch threshold feasibility

Static-only review for exact 66EC(XRGB)BLe;V1.0.48;V1.0 private image, SHA-256 b82e11891dfdc138780a81ac635697a7edb0ca1d4e123d6ad38c81a7ba55bf17. No hardware command, firmware write, or code change was performed.

A global relative actuation adjustment has a concrete existing firmware candidate: D8, with coefficient 10..40. It recalculates all 66 RAM R entries from the current stored T calibration and current global mode. It does not pretend a key was pressed, sample ADC, or set arbitrary per-key values. A selected-key numeric setter/arbitrary 66-entry table write was not found in the normal dispatcher. OS key injection cannot influence DD's internal ADC measurements.

## Exact payload and branch

Dispatcher 0x5c84 increments the incoming payload pointer once at 0x5c88 and reads opcode at payload index 1. D8 is selected at 0x5cb4..0x5cb8. Branch 0x5e88 reads `[r0,#1]`, therefore payload index 2 is the unsigned coefficient, and calls 0x754c. Candidate application report is the usual existing 64-byte frame `[00,D8,c,00,...]`, coefficient c in 10..40. The HID API report-ID framing must match the already established transport; this static report is not a hardware-qualified write API.

## Computation and scope

Routine 0x754c first checks unsigned `(c-10) <= 30`, returning otherwise. It loops 11 rows × 6 columns. T base literal at 0x75a8 is 0x200010cc, and R is T-0x42=0x2000108a. For T != FF, it stores max(9, floor(T*c/40)), using byte narrowing after division. For T == FF, it instead stores `10 + [2,4,8][global_mode]`, giving 12/14/18 for modes 0/1/2. Thus default/sentinel cells do not respond to c, lower values hit the minimum 9, and integer rounding can mean a coefficient change produces no measured R change. This is a global formula, not all keys set to the same requested threshold.

The routine reads T and global mode but writes only R cells; no B/T stores, ADC calls, flash writer, or EEPROM persist call appears in this path. It calls 0x87d8 after every completed row; that helper 0x87d8..0x87f2 calls 0x63f0 and services peripheral state. No acknowledged durable coefficient setting is established. Dispatcher then follows 0x5ed2..0x5ede to toggle shared feedback bit 0x20 at 0x200009b8, invokes 0x5038, delays via 0x31bc, and finishes through 0x87d8. That feedback happens even for an invalid coefficient; an LED change is not evidence that R changed. Unlike DB/DD, this branch has no explicit 00 DA DA / 00 DE DE USB success reply.

## Lifetime/current mode

D8 does not store c or change current global mode 0x20000959. Mode regenerator 0x7624 calls 0x754c with fixed coefficients 20/24/28 for modes 0/1/2. Confirmed direct call sites are D5 0x30ce, DD 0x32fe, global mode change 0x4738, and startup 0x4876. Global-mode action 0x4718 persists mode at EEPROM address 6 then regenerates R. A custom D8 scale is therefore temporary RAM behavior and can be replaced by those operations/restart. Arbitrary restoration of a prior R table cannot be promised because no R-table setter was established; restoring only the stored mode coefficient may differ from the prior temporary scale.

## Verification via qualified DC1, without DC2

Existing qualified DC1 before/after can verify **observed R table changes** as full physical 66-value vectors on the same identity/firmware/owned connection. DC1 does not need DC2 to establish before/after R. Static getter 0x670c reads R without changing B/T/R or global coefficient, but pauses/resets scan state and restores compensation defaults, so preserve current app's released-key requirement, exclusive lock, exact target/F9 validation, fail-closed parser, timeout uncertainty, and no automatic retry. D8 itself must be separately qualified through an explicit hardware mutation plan before enabling the UI.

Without current T/global mode, DC1 alone cannot independently predict the exact expected formula for every cell; it also cannot empirically prove B/T preservation or persistence. Unchanged R is a valid outcome for sentinel/minimum/rounding cells and is not evidence of a failed write. A measured changed R is not proof of millimeter travel or improved response. Qualification should collect complete before/after R, command trace, and measured side effects, account for missing success acknowledgment, and avoid claiming individual-key isolation.

Primary sources: /private/tmp/niz-write-20261011/firmware/01-normal-dispatcher.txt (5c84,5cb4,5e88,5ed2), 13-D8-and-mode-regenerator.txt (754c..75b4), 14-mode-regenerator.txt (7624..764c), 24-config-mode.txt (4718..4750), 21-DC-getter.txt (670c..67c4), write-evidence.json direct call references; current crates/niz-device/src/session.rs raw_thresholds_inner confirms app-side qualified DC1 frame index and guarded parser/transport behavior.
