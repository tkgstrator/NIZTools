# Exact V1.0.48 firmware container feasibility

Date: 2026-10-11. Native verified host/workspace devonlys-MacBook-Pro.local, /Users/devonly/Developer/personal/NIZ. Bounded offline feasibility only. No vendor executable, firmware execution, flashing, HID, GUI, OS setting, repository source or Git mutation. One existing publicly evidenced decoder key was tested; no brute force or unknown command enumeration. The user subsequently clarified the goal as selected-key calibration to improve response, so dispatcher/ADC research was stopped after this useful extraction result.

**Result: the matching V1.0.48 package can be strictly validated and extracted into a contiguous analysis image using the existing DES-ECB wrapper decoder.** There is no missing decoder artifact at this boundary. This does not identify a new hardware command, per-key numeric threshold value/readback, or qualify firmware behavior on a device.

## Exact input / independently observed result

Input: `/private/tmp/niz-analysis-20261010/Atom66 EC-BLE RGB/2020.11 update/66EC(XRGB)BLe_V1.0.48_20200810[1].bin`.

- Original140,529 bytes, SHA-256878d2f6193d4da310da2762c35a1e1c1f25eeb5eb648f9159e917f9266e2f989.
- 2,653 colon-prefixed uppercase-hex records with final CRLF. Line lengths:2,650 x51,2 x19,1 x35. The first clear wrapper byte is decoded-record length; remaining ciphertext size is that length rounded up to an8-byte block boundary.
- Using the single evidenced DES-ECB key7600000000000000 produced valid zero padding and Intel HEX count/checksum for every record. Types:one4 extended-linear-address record,2,650 type0 data records,one5 entry record,one1 EOF.
- No overlapping addresses, no missing bytes, EOF last. The full re-encrypted record stream reproduced the original file byte-for-byte. This checks exact formatting and ciphertext, not merely a plausible first block.
- Recovered image base0x0, exclusive end0xa59c,42,396 bytes, SHA-256b82e11891dfdc138780a81ac635697a7edb0ca1d4e123d6ad38c81a7ba55bf17.
- Initial SP0x200027d0, reset vector0x000000f1, Intel HEX start address0xf1. Exact target version string66EC(XRGB)BLe;V1.0.48;V1.0; appears at0x9f94. These provide target identity and ARM/Thumb analysis plausibility; VID0483 alone does not prove a specific STM32 MCU. No chip family or instruction-set implementation was independently identified in this pass.
- ASCII-wrapper entropy4.1701 bits/byte; decoded-image entropy6.8559. Entropy is recorded as descriptive metadata, not used as a cryptographic/code identity proof.

## Evidence chain

Fresh target HWI DLL disassembly at VA10003436..100035d7 agrees with `docs/reverse-engineering/evidence/dll/firmware.txt`: helperVA10003510 converts the line after its colon from hex pairs into a buffer; VA1000352f puts00 3A at the prefix, while VA10003466 copies64 normalized bytes and VA10003472..10003478 sends65 Windows bytes including report-ID0. This host path transmits the wrapped length/ciphertext; it does not itself expose plaintext code or a threshold setting.

The pinned niz-web revision395a8f8e46ab677ecf30536c3e756ab7e68c2683 [gitmodules](https://raw.githubusercontent.com/cbingb666/niz-web/395a8f8e46ab677ecf30536c3e756ab7e68c2683/.gitmodules) points to `cbingb666/niz-firmware`; its pinned tree selects submodule commit8b92eaa889fc5b8d1aaede2f79af8a1d15f02352. The [reviewed decoder](https://raw.githubusercontent.com/cbingb666/niz-firmware/8b92eaa889fc5b8d1aaede2f79af8a1d15f02352/scripts/decode_firmware.py) supplies the observed one-byte length / DES-ECB / zero-padding / Intel HEX validation method and evidenced key. This checker independently applies that method through native LibreSSL3.3.6, without installing a Python crypto package or executing downloaded code.

The [recovered protocol document](https://raw.githubusercontent.com/cbingb666/niz-firmware/8b92eaa889fc5b8d1aaede2f79af8a1d15f02352/recovered/PROTOCOL.md) documents device-side decryption and known host-report format for the newer analyzed image. Its function addresses and analog/control branches are not transplanted onto V1.0.48. The [project README](https://raw.githubusercontent.com/cbingb666/niz-firmware/8b92eaa889fc5b8d1aaede2f79af8a1d15f02352/README.md) identifies its baseline as V1.5.1; extraction success here demonstrates format compatibility, not device-level behavioral equivalence.

## Reproduction / retained private evidence

Original helper: `/private/tmp/niz-ui-reference-20261011/validate_target_firmware.py`. It pins this input SHA, validates each wrapper/record, checks address completeness/EOF, re-encrypts to compare the complete original wrapper, and verifies the embedded exact target string. It emits only metadata by default; optional image output remains a private research artifact.

Run:

```sh
python3 /private/tmp/niz-ui-reference-20261011/validate_target_firmware.py '/private/tmp/niz-analysis-20261010/Atom66 EC-BLE RGB/2020.11 update/66EC(XRGB)BLe_V1.0.48_20200810[1].bin' --image /private/tmp/niz-ui-reference-20261011/target-v1.0.48-private-image.bin --manifest /private/tmp/niz-ui-reference-20261011/target-v1.0.48-manifest.json
```

The saved manifest contains all validation counts/hashes; the image remains under private temporary research storage and should not be included in the app/source distribution. It is an input for future static analysis, not a build, modified firmware or flashing artifact created by this task.

No bounded target-dispatcher/ADC decompilation was completed after the user clarified selected-key calibration. Existing vendor DB/DD evidence already supports the generic physical-holding guide, so more firmware research is unnecessary for that GUI change. If numeric threshold control becomes a separate requested goal, the image now removes the decoding obstacle; exact-target command/scan/calibration/storage semantics and units/readback still need independent analysis and qualification before implementing a setter.
