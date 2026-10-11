# Persistent numeric threshold follow-up

The required outcome is a host-chosen threshold written to the keyboard and retained like official calibration, while preserving the stock executable. The physical gesture or choice of implementation is not a requirement. D8 temporary adjustment does not satisfy this outcome. **A reliable persistent numeric setter is not implemented or qualified.**

The exact `66EC(XRGB)BLe;V1.0.48;V1.0;` image was independently reread by two Codex researchers. Its 42,396-byte SHA256 is `b82e11891dfdc138780a81ac635697a7edb0ca1d4e123d6ad38c81a7ba55bf17`. No hardware write, exploit packet, firmware record, or vendor executable was generated/executed in this follow-up. The coordinator's actual app action was settings read for the separately completed font correction.

## Storage exists; numerical ingress is the gap

Official DD calibration saves the per-cell calibration table T to on-chip flash `0x10e00` through `0x32fa -> 0x8954`. DB saves released baseline B/reference at `0x10c00`/`0x10c44`. Startup reloads them. Therefore the keyboard demonstrably supports independently stored per-key calibration; storage does not inherently require a physical press.

The inspected DD host packet carries no supplied key/table/value. The firmware samples analog input, updates qualifying T cells, saves the whole T block, regenerates normal thresholds R, then replies DE. DD is not a pure save of host-provided bytes. Its flash callers do not check erase/program status, so an acknowledgment alone is not independent restart-persistence verification.

D8 changes R in RAM rather than saved T. DD after D8 measures T again and regenerates R from T and the saved mode; it does not preserve the D8 R table. Under saved coefficients 20/24/28, R is derived from valid T 16..254 or a default sentinel, with rounding and minimum handling. Thus even a future T uploader needs a precise value-conversion contract.

The ordinary host interfaces, alternative USB control/feature ingress and inspected official DLLs did not establish a numerical T uploader or pure analog save command. The internal IIC writer accesses a separate serial EEPROM used for keymap/RGB/update staging; its addresses do not alias the on-chip calibration flash.

## Additional candidate routes checked

An unchecked long-record RAM append path can reach T only after overwriting 3,903 preceding bytes, including device control state, keymap metadata, B and R. The inspected cursor/index handling did not establish a bounded T-only seek or preservation of those bytes. This is a corruption candidate, not a qualified implementation. No such packet was sent to hardware.

The existing updater was also considered as an unchanged stock image plus calibration data, rather than automatically equating every update route with modified executable code. Its application-side checksum validator can accept appended staging bytes, but that does not establish where the bootloader will program them. The cursor is 32-bit, supplied record addresses are 16-bit, and completion length is 16-bit; nonwrapping EEPROM staging holds at most 65,532 image bytes. A simple base-zero linear copy cannot reach calibration flash at `0x10c00`/`0x10e00`. Exact bootloader/LDROM mapping, preservation and recovery behavior are missing, so a calibration trailer is unproved.

These results bound the inspected paths; they do not prove that every hidden factory/module/debug/programmer path is impossible. Further qualification needs a positively identified numerical ingress or exact bootloader/storage evidence before a write can be presented as successful and safe for the other stored data.

## Current official package verification and evidence

The coordinator opened the current [manufacturer distribution page](https://www.nizkeyboard.com/pages/firmware-upgrade), followed its CalibrationLite distribution, and freshly read the 2026 archive in memory. Archive SHA256 `87d1f2ac8f77c99244fa8cffc23c7ef9987aa8b1f4a0a8c41c4e1c8d53dc63ce` and KBDLL SHA256 `4a73d74f378b5d6c0b731257eb6432482e8184efc27e7f611161db8a6e12fe32` match the previously disassembled exact binaries. [Refresh metadata](../validation/persistent-threshold-official-package-refresh.json) records the read and comparison. No Windows code was loaded or launched.

Detailed independent address/control-flow reports remain local in `/private/tmp/niz-persistent-threshold-20261011/device-storage-review.md` and `independent-feasibility-review.md`; original private firmware/vendor artifacts were not added to the repository. LocalGPT and Claude generation tools were unavailable, so authorized independent Codex research was used. This document records unfinished feasibility work; it is not a persistent-write implementation or hardware qualification result.
