# Exact-target live press depth review

Static-only independent review on 2026-10-11. Workspace `/Users/devonly/Developer/personal/NIZ`; no HID/device command, firmware execution, updater, or app launch occurred.

## Conclusion

The exact firmware acquires live per-cell analog signals internally. The inspected normal USB command dispatcher exposes B/R/T snapshots through DC modes 0/1/2, **not the current sampled signal S**. Thus the existing DC1 read cannot implement a continuously moving press-depth indicator. No supported live analog sample read/stream command was established in the recovered normal dispatcher or the inspected vendor calibration exports. This is a bounded static conclusion, not proof that every alternate interface, module, or undocumented factory path is impossible.

Even a future verified S/ADC transport would initially supply relative sensor counts. Converting those to millimeters requires per-key travel calibration and an established sensor-to-distance relationship; neither follows from threshold bytes or D8's numeric coefficient.

## Primary private image verification

Fresh read of `/private/tmp/niz-ui-reference-20261011/target-v1.0.48-private-image.bin`: 42,396 bytes; SHA-256 `b82e11891dfdc138780a81ac635697a7edb0ca1d4e123d6ad38c81a7ba55bf17`; literal at 0x9f94 `66EC(XRGB)BLe;V1.0.48;V1.0;`. Fresh Capstone disassembly confirmed sampling paths and direct BL references; focused earlier extracts were checked against this exact image identity.

## Observable analog acquisition

- ADC helper 0x5a58 selects one of six channels; at 0x5a9c it loads the ADC register and at 0x5aa0..0x5aa2 masks the return value to ten bits.
- Ordinary acquisition 0x3ae4 calls ADC at 0x3af6. At 0x3afa..0x3b08 it computes positive `reference(0x20000998) - ADC`, narrows to a byte, and replaces values exceeding 128 by zero. At 0x3b0a..0x3b14 it stores six sampled bytes into seven-byte row records rooted at 0x2000190e (one row index plus six values).
- Alternative acquisition 0x3b50 calls ADC at 0x3b56 and uses the same reference transform, accumulating channel samples before populating row records.
- Normal scanner 0x4938 consumes those queued samples. At 0x497e it loads the current sample, subtracts B[cell] at 0x4982..0x4990, applies row compensation via 0x4b94 at 0x4994, and compares with R[cell] at 0x4998..0x49a2. This is independent evidence that S and R have different meanings.
- Calibration acquisition 0x5ab8 averages 50 samples for each of 11×6 cells. Transform is `uint8(0x303 - ADC)` for ADC < 0x303, else zero; output is floor(sum/50). Direct calls found at 0x31fa (DB) and 0x3280 (DD). Those paths perform calibration/persistence, not a read-only live measurement service.

## DC is a snapshot getter of configuration/calibration tables

Normal opcode dispatcher 0x5c84, DC branch 0x5e76..0x5e7c, accepts mode <3 only. Getter 0x670c constructs eleven 64-byte replies; row number 1..11 and mode occupy header bytes. At 0x6744..0x6754, pointers derive from B=0x20001006, B+0x84=R=0x2000108a, and R+0x42=T=0x200010cc. At 0x6764/0x6768/0x676c it copies B/R/T respectively. No current row-record/sample source 0x2000190e is used. Getter saves/disables scanning and resets scan/output state at 0x671a..0x672a, then restores scan enable/compensation at 0x67aa..0x67b4. Repeated DC reads are not a harmless analog stream.

Primary extracts: `/private/tmp/niz-write-20261011/firmware/01-normal-dispatcher.txt`, `21-DC-getter.txt`, `22-DD.txt`, `23-DB.txt`; `/private/tmp/niz-perkey-20261011/firmware/sampling-disassembly.txt`, `scan-disassembly.txt`.

## Dispatcher and send-path coverage

The normal dispatch comparison chain and both compiler switch tables were inspected across 0x5c84..0x5fd8, including DB/DD/D8/D9/DC and keymap, lighting, metadata and firmware-update branches. DC is the discovered analog table transport. No selector beyond modes 0..2 or sample-buffer read branch was found. A fresh halfword-aligned direct-call search found ADC references 0x3af6, 0x3b56, 0x5b0e, 0x69bc, and USB send references 0x3218, 0x3312, 0x588e, 0x58b0, 0x5a10, 0x5bf6, 0x677e, 0x67a6, 0x6908, 0x7230, 0x7298, 0x770a. The potentially relevant non-DC configuration/lighting send ranges 0x5860..0x58b4 and 0x59bc..0x5c04 were inspected; they carry EEPROM/configuration tables and completion packets. 0x770a wraps FA module data, rather than a demonstrated S transport. This aligned direct-call search is not a complete indirect-call/control-flow proof.

## Vendor evidence

`/private/tmp/niz-perkey-20261011/vendor/calibration-request-disassembly.txt` shows KBDLL CalibrationInit at 0x10001320 constructs 00 DB 00; CalibrationPress at 0x10001370 constructs 00 DD 00. These are zero-argument release/held calibration operations. `/private/tmp/niz-write-20261011/vendor/calibration-instructions.txt` directs release-all calibration followed by holding abnormal key(s) for press calibration. Neither inspected evidence supplies live numeric press depth or millimeter conversion.

## UI plan findings delivered before implementation

Bounded simplification belongs mainly in app.rs. Keep raw_input_hook, modifier/key-up/reference clearing, live/focus/busy/modal guards. Remove transcript/codes/clear/layer controls and render normal layer 0. FROM/TO must accept complete physical 66-value vectors only, owned by the current device/baseline/operation; existing unconditional held.after fallback is insufficient provenance. Preserve primary mouse activation for prepare/execute and cancellation/unlock handling. Warn about neighbors only when measured other-key differences >0. Keep device/demo provenance distinguishable. Retain the meaningful existing input tests and add bounded current-source/full-vector presentation tests.
