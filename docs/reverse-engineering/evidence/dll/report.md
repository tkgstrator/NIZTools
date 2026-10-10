# Atom66 DLL static HID findings

Target: host `devonlys-MacBook-Pro.local`; `/private/tmp/niz-analysis-20261010/Software/66EC(XRGB)BleHWI.dll`.
Verified SHA256: `1ca6c2141ff2aac1d38c03533ac56a64c27beb9e4e82eae9a1f6c9abf5c108d3`; file size 66,048 bytes. No binary execution or device I/O occurred.

## Evidence generation

`objdump -p '/private/tmp/niz-analysis-20261010/Software/66EC(XRGB)BleHWI.dll' > pe-headers.txt`

`objdump -d '/private/tmp/niz-analysis-20261010/Software/66EC(XRGB)BleHWI.dll' > disassembly.txt`

`strings -a '/private/tmp/niz-analysis-20261010/Software/66EC(XRGB)BleHWI.dll' > strings.txt`

Those files and address-filtered evidence excerpts are in this report's directory. All virtual addresses assume preferred ImageBase `0x10000000`. For `.text`, raw file offset = VA - `0x10000000` - `0x0c00`; for `.rdata`, offset = VA - `0x10000000` - `0x1200`.

## PE and exports

Native PE32 / coff-i386, linker version 10.0, timestamp 2022-06-23, preferred base 0x10000000; no CLR header. It cannot be directly linked into a native macOS Rust application. `.text`: RVA 0x1000, raw 0x400, raw size 0x9a00; `.rdata`: RVA 0xb000, raw 0x9e00, raw size 0x3c00.

The 22 exported names are C++ methods on `CKB75HWI` plus Windows hook functions. Complete export table is in `pe-headers.txt`. Principal exported entry points:

| Name | RVA | Implementation VA | Raw implementation offset |
|---|---:|---:|---:|
| __createDev | 0x1cc0 | 0x10001cc0 -> 0x10002520 | 0x10c0 -> 0x1920 |
| __closeDev | 0x1d20 | 0x10001d20 | 0x1120 |
| DetectDevice | 0x23e0 | 0x100029a0 | 0x1da0 |
| DetectDeviceType | 0x2420 | 0x10002e10 | 0x2210 |
| ReadKeyboardVersion | 0x23d0 | virtual call -> 0x10002460 | 0x1860 |
| getHardwareVersion | 0x1d70 | virtual call -> 0x10002460 | 0x1860 |
| readKeyDefinationFromDev | 0x2390 | 0x100014d0 | 0x8d0 |
| writeKeyConfigurationToDev | 0x2380 | 0x10001860 | 0xc60 |
| readKeyLightFromDev | 0x23a0 | 0x10001a80 | 0xe80 |
| setKeyboardLightArea | 0x2040 | 0x10002040 | 0x1440 |
| getPressCounterFromDev | 0x23f0 | calls 0x10001b70 | 0xf70 |
| WriteFirmwareToKeyboard | 0x23b0 | calls 0x10003350 | 0x2750 |

`setKeyConfiguration` and `getKeyConfiguration` act on in-memory structures. The explicit write function transmits the complete three-layer key configuration.

## Device matching

The DLL enumerates Windows HID paths using HidD_GetHidGuid / SetupDiGetClassDevsW / SetupDiEnumDeviceInfo / SetupDiEnumDeviceInterfaces / SetupDiGetDeviceInterfaceDetailW. It searches for `vid_`, `pid_`, and `mi_01`, and compares four-character values extracted from those paths.

| UTF16 value | VA | Raw file offset |
|---|---:|---:|
| 0483 | 0x1000d3f8 | 0xc1f8 |
| 502a | 0x1000d3ec | 0xc1ec |
| 512a | 0x1000d434 | 0xc234 |
| 522a | 0x1000d440 | 0xc240 |
| mi_01 | 0x1000d428 | 0xc228 |

Confirmed software matching: VID `0x0483`, PIDs `0x502a`, `0x512a`, `0x522a`, interface 1. `DetectDeviceType` returns 1 for 502a (VA 0x1000329c), 2 for 512a (0x100032f9), and 3 for 522a (0x10003343); missing/unsupported returns 0. Static code alone does not prove which of these corresponds to a particular connected Atom66 revision, RGB option, or BLE mode.

## Transport

Vtable at VA 0x1000d47c / raw 0xc27c points to `0x10003bf0` (write), `0x10003c20` (read), and `0x10002460` (version).

CreateFileW at 0x10002430 opens the matched HID path with GENERIC_READ | GENERIC_WRITE (0xc0000000), FILE_SHARE_READ (1), OPEN_EXISTING (3), no overlapped flags. The writer invokes WriteFile via IAT `0x1000b020` at VA 0x10003c05. The reader invokes ReadFile via IAT `0x1000b024` at VA 0x10003c48.

Every protocol request analyzed supplies 65 bytes to WriteFile, with first byte zero. ReadFile requests 65 bytes and the wrapper copies from received buffer +1 into the caller's logical 64-byte buffer. This is consistent with Windows HID's report-ID prefix convention (software passes report ID 0). Actual report descriptor/report ID/usage page must still be verified from hardware; descriptor bytes are absent here. USB endpoint addresses, bInterval, usage page/usage, and BLE/GATT services are not present in the analyzed transport. The BLE filename does not prove BLE configuration support: observed host transport is Windows HID, without BLE API imports.

No HidD_SetFeature or HidD_GetFeature imports. No custom kernel driver API is used in this DLL. This is evidence for a user-space HID port, not proof that macOS can access the intended collection without descriptor validation.

The read method's third argument is ignored. Callers pass 100000, but this is not an implemented timeout: ReadFile is synchronous. It also copies the caller-requested count before checking actual returned count, then reports max(requested, actual-1). The write wrapper checks WriteFile's boolean result only, not full byte count. A port should use bounded reads, actual lengths, valid record limits, and reliable full-write checks instead of reproducing those defects.

## Report notation

`raw[0..64]` means the 65-byte Windows I/O buffer, including its first zero byte. `payload[0..63]` is raw bytes 1..64 after the DLL removes that byte. Serialized key record offsets below are payload offsets, never raw offsets. Distinguishing them is essential: command bytes commonly appear at payload[1] / raw[2].

## Confirmed request workflows

| Operation | Evidence implementation VA | Raw request |
|---|---:|---|
| Read version | 0x10002460 | 00 00 F9 followed by 62 zeros |
| Read complete key configuration | 0x100014d0 | 00 00 F2 followed by 62 zeros |
| Begin complete key configuration write | 0x10001860 | 00 00 F1 followed by 62 zeros |
| Read lighting areas | 0x10001a80 | 00 00 E2 followed by 62 zeros |
| Read press counters | 0x10001b70 | 00 00 E3 followed by 62 zeros |
| Begin lighting-area write | 0x10002040 | 00 00 E1 followed by 62 zeros |

Version reader converts payload starting at offset 2 to Windows wide characters via MultiByteToWideChar(CP_ACP, flags 0), for `receivedLength - 2` bytes. It does not validate response command bytes. Actual response encoding/format requires captures.

Configuration read loops on 64-byte payloads. It finishes when payload[0] == F6 (0x10001570 / 0x10001833). For record payload[1] == F0: layer = payload[2], key = payload[3], type = payload[4]. Types 0..4 select serializers/parsers; higher kinds appear to lead to a null-object call rather than robust rejection. Internal indexing is `(layer - 1) * 66 + (key - 1)`. Multi-report kinds derive body length from payload[9] << 8 | payload[10], allocate `ceil(bodyLength / 53) * 64`, and read additional reports before parsing. This establishes 11-byte header / 53-byte body chunks for these kinds.

Configuration write iterates three layers (1..3), each containing 66 keys (1..66). For present keys it invokes the record serializer, copies successive chunks of at most 64 serialized bytes into raw buffer +1, and writes 65 bytes. An unset slot produces raw prefix `00 00 F0 layer key 00 00`, with remaining bytes zero. End record is raw `00 00` followed by 63 bytes of F6 (0x10001a20..0x10001a46). The method performs no response read or persistent-state readback; the meaning of commit/end and persistence behavior is inferred from the exported operation, not proven on hardware. Last short serialized chunks can retain earlier bytes in the report buffer; a new implementation should zero padding explicitly.

Lighting read accepts payload[1] == E0, payload[2] byte count, payload[3..] data; accumulates into 198 bytes, with payload[1] == E6 ending the stream (0x10001b04..0x10001b57). Lighting write starts E1, sends raw `00 00 E0 count data...`, payload count at raw[3], body at raw[4], chunks at most 61 data bytes; completes 198 bytes. End report is raw[0] = 00 and raw[1..64] = E6 (0x10002122..0x10002148). These end markers differ from configuration-write end markers.

Press-counter read accepts payload[1] == E3, payload[2] count, payload[3..] raw data; accumulates until payload[1] == E6. Export then copies 66 native Windows 32-bit ints (264 bytes). This strongly implies 66 little-endian 32-bit counters; precise counter semantics, wraparound, and reset behavior are unknown. Export ignores the reader's return flag and returns true even on failure.

## Key serializers

Serializer type 0, VA 0x10004390 / raw 0x3790:

`payload = [00, F0, layer, key, 00, count, values...]`

Layer comes from object +8, key from +4, count from +0x20, values from int vector at +0x10 truncated to bytes. Parser 0x10004410 reconstructs the same count and values. Meaning of the values (standard HID codes vs firmware-specific codes) must be reconciled with main-EXE tables; DLL alone does not label them.

Serializer type 1, VA 0x10001040 / raw 0x440:

`payload = [00, F0, layer, key, 01, fieldHi, fieldLo, count, values...]`

16-bit field from object +0x10 big endian; count from +0x24; values from vector +0x14. Parser at 0x100010d0 reverses this mapping. Field semantics are unknown from this DLL.

Serializer types 2,3,4 use VA 0x10003d50 / raw 0x3150. Each 64-byte serialized chunk repeats an 11-byte header:

| Payload offset | Confirmed value/source |
|---:|---|
| 0 | 00 |
| 1 | F0 |
| 2 | layer from object +8 |
| 3 | key from object +4 |
| 4 | kind from object +0xc (2,3,4) |
| 5 | low byte of object +0x14 |
| 6 | low byte of object +0x10 (encoding mode) |
| 7..8 | object +0x3c big endian |
| 9..10 | total body byte length big endian |
| 11..63 | body chunk of up to 53 bytes |

For mode 0, body contains vector +0x28 values as bytes; body length equals element count (object +0x38). The native DLL writer in nonzero mode advertises 4 * element count and emits each entry as `[valueByte, C8, secondaryValueHi, secondaryValueLo]`, with secondary vector +0x18. There is no last-element exception in the writer. However the native parser explicitly skips reading the secondary field for the last element; this makes the shorter 4*n-3 format used by the public implementation compatible with this parser, without matching the native writer byte-for-byte. Delay/macro labels and units are plausible but not proven from the DLL alone. `setKeyConfiguration` maps its external group subtype 0/1/2 into wire types 2/3/4; UI meaning needs main application inspection.

## Firmware boundary

WriteFirmwareToKeyboard at export 0x100023b0 delegates to 0x10003350. It requires a string starting ':' and splits CR/LF lines. Hex parser 0x10003510 creates a buffer starting `00 3A`, followed by decoded hex pairs. It copies 64 bytes to the HID payload and transmits a 65-byte report. This resembles an ASCII-hex firmware-record transport but is not enough to establish Intel HEX address interpretation, checksum verification, bootloader transition, compatibility, rollback, or recovery. No firmware write should be enabled based only on this inspection.

## Port feasibility and unresolved evidence

Static results support implementing the framing/record codec in portable Rust and replacing Windows enumeration/read/write with native macOS HID access. Apple Silicon and Intel builds can share pure Rust codec and model logic. This is an architectural inference; a functioning macOS transport and full device compatibility have not been demonstrated.

Before claiming real device support, acquire and review: live USB/HID descriptors; exact Atom66 revision's VID/PID, interface and usage collection; report lengths and macOS report-ID behavior; successful read-only version/configuration captures; key-position/code tables from main EXE/manual; write+readback behavior on an authorized device; exact action/macro/delay semantics; distinction among the three PIDs; whether wired USB is required when configuring BLE-capable boards; disconnect and partial-transfer recovery. Firmware flashing is a separate higher-risk reverse-engineering effort.

## Independent macro-length recheck

The public reference `/private/tmp/niz-analysis-20261010/public/niz-web/protocol.ts` lines 132-139 only emits C8/secondary bytes between keys, producing `4*n-3` for n >= 1 in custom mode. This is a real native-writer/public-writer difference, not a disassembly field-inference correction. Native writer evidence:

- VA 0x10003d5e (`8b 5e 38`) loads element count n; 0x10003d69 (`8d 04 9d 00 00 00 00`) computes 4*n; 0x10003d70..73 saves it as total length, subsequently written to header offsets9/10 at 0x10003ddc / 0x10003e5d. No subtraction by3 occurs.
- VA 0x10003e17 (`88 50 ff`) writes key; 0x10003e1a (`c6 00 c8`) writes C8; 0x10003e26 (`88 50 01`) and 0x10003e30 (`88 50 02`) write secondary bytes.
- VA 0x10003e33 (`41`) increments index; 0x10003e34 (`83 c0 04`) advances body pointer by4; 0x10003e37 (`3b cb`) compares index to n; 0x10003e39 (`7c d5`) loops while index<n. All four writes occur before the only last-element test.
- Native parser VA 0x10004136..43 computes `floor((bodyLength-1)/4)+1`, i.e. ceil(bodyLength/4) for positive lengths. Both 4*n and 4*n-3 produce n.
- Parser VA 0x1000433c (`8d 4b ff`) computes n-1; 0x1000433f (`3b c1`) compares index; 0x10004344 (`74 20`) skips to 0x10004366 on the final key, bypassing secondary reads at 0x10004349..0x1000435d.

This asymmetry is supported by `macro-builder.txt` and newly saved `macro-parser.txt`. Keep it as an explicit compatibility discrepancy until hardware captures establish which trailing form the device stores/accepts. No device commands were sent.

Supplementary descriptor evidence arrived after the static analysis: `/private/tmp/niz-analysis-20261010/atom66-hid-descriptor.hex`. I independently read its bytes: usage page item `05 8c`, application usage `09 01`, input `75 40 95 08 81 02`, output `75 40 95 08 91 02`; 64-bit fields times8 = 512bits = 64bytes for IN and OUT. There is no Report ID (`85`) item. This resolves report size/ID and usage-page/usage uncertainty for that supplied descriptor. Parent coordination reports matching live IORegistry device VID:PID 0483:502a; I did not independently repeat IORegistry enumeration. It does not establish command compatibility or eliminate the need for controlled read-only captures.
