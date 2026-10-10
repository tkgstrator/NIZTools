# Atom66 XRGB BLE calibration: static evidence and implementation proposal

Verified host: `devonlys-MacBook-Pro.local`. Target reported by the coordinator and independently read from `/private/tmp/niz-implementation-20261010/hardware-baseline.json`: VID `0483`, PID `502a`, usage page/usage `008c/1`, exact firmware `66EC(XRGB)BLe;V1.0.48;V1.0;`. The baseline contains 198 configuration reports; its `counters` array is EMPTY, and it has no per-key lighting field. No calibration, lock, configuration-write or firmware command was sent during this investigation. No source/Git changes occurred; this report is the only file written.

## What is proven

Calibration belongs to the separate official **CalibrationLite / KBDLL.dll**, not the supplied `66EC(XRGB)BleHWI.dll`. Independently inspected official artifacts were fetched into memory and passed to `objdump` through stdin; Windows binaries were never loaded/executed. RAR members were read with `tar -xOf -` to stdout, without filesystem extraction.

| Artifact | Bytes | Independently verified SHA256 |
|---|---:|---|
| Older KBDLL.dll | 17,408 | c40e164fdbc022ad96b786c78bfbc0bf8aede527a72a8991598c3807c9a71b5f |
| Older CalibrationLite.exe | 1,761,280 | 580049eadb6e331f44627214bbca136d540dede3f6e2367b6d6bb22de3d0b90d |
| NIZ-EC-CalibrationLite-2026.rar | 5,417,486 | 87d1f2ac8f77c99244fa8cffc23c7ef9987aa8b1f4a0a8c41c4e1c8d53dc63ce |
| 2026 KBDLL.dll | 17,408 | 4a73d74f378b5d6c0b731257eb6432482e8184efc27e7f611161db8a6e12fe32 |
| 2026 CalibrationLite.exe | 1,835,008 | 86d5a16164761ecf88817b5d9afa992f989741db9e784cdd7efa3ae5bf2a2a9a |
| 2026 使用说明Instructions.txt | 1,060 | 0a03d04e337e23d7b72ef3c75e3c7c5d245d289b0f6dbbd23966c9db67212bf6 |

Older and 2026 DLL `.text` raw sections are each 8,704 bytes with identical SHA256 `a26b7e701e4962f5e693f16627b05370ca3d6b7132f7839b9c7ea952565a7d90`. Preferred DLL image base is `0x10000000`, `.text` RVA `0x1000`, raw offset `0x400`; code raw offset = RVA - `0xc00`.

## Exact command and completion evidence

Below, **payload** means 64 bytes for macOS HID, with no extra Windows report-ID byte. Raw Windows I/O is 65 bytes: leading zero plus that payload. All outgoing bytes after the specified prefix are zero. Current live descriptor has 64-byte IN/OUT and no Report IDs; software's leading zero is framing for Windows/hidapi, not part of the actual payload.

| Action | Payload prefix | DLL export VA / raw offset | Machine evidence |
|---|---|---|---|
| Identify | `00 F9 00` | GetVersion `100012d0` / `6d0` | `100012e5` pushes 65; `100012f8: 66 c7 46 09 00 f9` |
| Lock typing | `00 D9 00` | Keylock `100013c0` / `7c0` | `100013ea: 66 c7 46 09 00 d9`; `100013f0: 88 46 0b` stores argument |
| Calibrate released state | `00 DB 00` | CalibrationInit `10001320` / `720` | `10001335` pushes 65; `10001348: 66 c7 46 09 00 db` |
| Calibrate physically held keys | `00 DD 00` | CalibrationPress `10001370` / `770` | `10001385` pushes 65; `10001398: 66 c7 46 09 00 dd` |
| Unlock typing | `00 D9 01` | Keylock `100013c0` / `7c0` | argument 1 goes to raw byte3 / payload byte2 |

Each command clears 65 bytes via memset, writes raw[0]=0, puts `00 opcode` at raw[1..2], and argument at raw[3]. Writer `10001aa0` sends 65 bytes: `10001ae3: 6a 41`; WriteFile call `10001ae7: ff 15 58 40 00 10`. The receive helper `10001b70` clears/request-reads 65 bytes and calls ReadFile at `10001bca`.

Receiver thread reads **raw byte2** (`1000116e: 8a 44 24 12`), recognizing `F9` at `10001172`, `DA` at `1000118a: 3c da`, and `DE` at `100011a2: 3c de`. DA forwards Windows message `0x7eb`; DE forwards `0x7ec`. No other completion, progress, lock/unlock acknowledgement, calibration data readback, calibration backup/restore, save, reset or rollback export exists in these 12 DLL exports. No command E8 is established for calibration. Do not invent a save/reset or use configuration commit F6 to finish calibration.

**Important receive limit:** the vendor receiver checks only raw[2], not raw[1]==0, actual packet length, or response padding. Thus expected DA/DE opcodes are directly proven; strict normalized response predicates `len==64 && payload[0]==0 && payload[1]==DA/DE` are a defensive candidate matching the current port, not yet a real calibration-capture result. Preserve complete unknown response fields; do not require all remaining bytes zero. No millimeter, force, capacitance or scan-time units are exposed by these commands.

2026 EXE base `0x400000` confirms call sites and polarity:

- Connection handler calls GetVersion at `0040230f` through IAT `005261e0`, then pushes 0 at `00402315` and calls Keylock at `0040231d` through `005261e4`.
- On `SC_CLOSE` (`0040219f` compares `0xf060`), it pushes 1 at `004021a6` and calls Keylock at `004021ae`. It sleeps 50 ms afterward; that is not an unlock acknowledgement.
- Release button calls CalibrationInit at `004025be`, IAT `005261f0`, after user confirmation. Press button calls CalibrationPress at `0040268e`, IAT `005261f4`, after confirmation.
- Message map `005507c8`: `[7eb,0,0,0,e,004024b0]`; `005507e0`: `[7ec,0,0,0,e,004024f0]`.
- DA handler `004024b0` uses completion string at `00550918`: “Key release calibration completed!”; DE handler `004024f0` uses `00550978`: “Key press calibration completed!”. Neither decodes the forwarded payload. Handlers display messages without disabling the calibration buttons or imposing a one-shot count.

Do not copy the vendor client's connection-triggered lock into the macOS app. Lock only inside the explicit experiment the user begins. Status is event-driven DA/DE; no polling opcode is established.

## Vendor user steps and eligibility

Official 2026 instructions (GBK-compatible text, independently read) require: connect and confirm connected status; release **every key** and run ReleaseCalib once for all keys; physically hold one or several affected keys and run PressCalib; keep them held until completion, then release. A screen-selected key is not a hardware target: DD carries no key index/mask. Bounded exact instructional excerpt: “wait until the prompt is completed before release the key.”

The instructions give **no model/firmware compatibility matrix**, storage timing, backup, recovery or reset procedure. The older DLL's device discovery searches `vid_` and `mi_01` and compares vendor 0483; it does not use an exact Atom66 firmware/PID allowlist. The matching `.text` establishes client protocol stability, not that V1.0.48 firmware implements/persists these requests correctly.

The supplied target firmware package `/private/tmp/niz-analysis-20261010/Atom66 EC-BLE RGB/2020.11 update/66EC(XRGB)BLe_V1.0.48_20200810[1].bin` is 140,529 bytes, SHA256 `878d2f6193d4da310da2762c35a1e1c1f25eeb5eb648f9159e917f9266e2f989`; its colon-prefixed records are encoded vendor records, not a readily identified raw instruction image. This investigation did not decode that firmware's command handler. Exact V1.0.48 eligibility remains experimental.

Trigger-point selection is distinct: extracted vendor manual `/private/tmp/niz-analysis-20261010/manual-agent/66EC-XRGB-Ble.txt:59-65` describes Fn + 7/& selecting high/low trigger points and Fn + Esc toggling keyboard lock. It does not equate that with DB/DD calibration or provide calibration millimeter settings. Treat hardware hotkey recovery as untested, not guaranteed cleanup.

## Concrete initial workflow for this user's identified target

Use an explicit **experimental calibration** flow bound to exact VID0483/PID502a, usage008c/1, firmware `66EC(XRGB)BLe;V1.0.48;V1.0;`, current device handle and connection generation. Other tuples remain unavailable until investigated. Recheck identity before lock; no automatic commands on connecting or opening the dialog.

Before any changing command, save a durable full key-settings backup and capture/preserve the **198-byte RGB/per-key-lighting state**. The existing baseline currently lacks lighting. This backup can restore key settings, not analog calibration; no analog rollback exists. Preserve drafts/undo locally. Calibration has no F0/E0 keymap/lighting writes in the inspected clients, but unchanged hardware settings must be verified, not promised.

1. Explain release-all-keys and mouse/trackpad operation. If Enter/Space confirmed the dialog, require release before proceeding. Set possibly-locked state **before** dispatching `00 D9 00`.
2. Send `00 DB 00` once; await the expected DA notification with a bounded, configurable deadline. The 10-second public-port deadline is experimental, not vendor-measured.
3. Await explicit user action while holding exclusive device ownership. User holds affected keys, clicks press calibration, send `00 DD 00`, await DE, then tell user to release.
4. Finish with `00 D9 01`; report “unlock command sent” separately from device completion/typing recovery. Test affected keys. Read back key configuration and RGB state under the existing explicit read procedure; verify unchanged settings before allowing a key-swap write.

Repeated user-requested held batches have client-level evidence: stateless repeated CalibrationPress export, reusable vendor press-button handler, and the pinned public port's action loop. No vendor text mandates repetitions and no V1.0.48 real trace proves their firmware behavior. Initial qualification should do one held batch; allow subsequent explicit batches only after DE and successful target qualification. Never automatically repeat DB/DD, retry on timeout, or attribute a late response to a new run. Requests have no transaction ID.

## Cleanup and fault tests

Keep one device worker/operation owner across all stages and user waits; reject key reads/writes, lighting writes and device changes during the run. Invalidate the write baseline after a calibration-changing send is attempted; do not discard its backup/editor.

- If lock/release/press physical writes completed and the error is an input timeout/wrong response, stop calibration; when the original handle/generation is still valid and **no write is outstanding**, permit one bounded D9 01 cleanup attempt through a dedicated cleanup path even if the ordinary protocol state is failed. Never clear the failure to continue calibration. Preserve the original error and separately record cleanup outcome.
- If a physical output write is unresolved/stalled at timeout, do not enqueue competing unlock, close/reopen, or continue. Await its completion or transport-level termination before any cleanup decision. A timeout alone does not prove a packet was not sent.
- Disconnect/changed identity: do not unlock another/reopened handle and never resume/replay automatically. Report unknown lock/calibration outcome. Successful unlock transmission has no proven acknowledgement; successful typing test is additional evidence, not persistence proof.
- Cleanup failure stays reviewable; no guaranteed recovery on crash, sleep, cable removal, or process kill. Qualify a vendor-supported recovery procedure before intentionally testing interruptions.

Required offline tests: exact 64-byte payload/padding/lock polarity; cancel/no-send; DB→DA and DD→DE sequencing; DD prohibited before DA; UI activation-key release; wrong/duplicate/stale/reordered/truncated replies; nonzero unknown reply tail preserved; input deadline without retry; exclusive ownership during user wait; double-clicks and one-time start; exact identity/generation recheck; confirmed-send failure versus unresolved-send cleanup; failed/unknown unlock status; no writes after disconnect; preservation of 198 key records, RGB bytes, drafts and backup; no F0/E0/F1/E1/F6/E6/firmware/reset command emitted by calibration; no percent or sensor-value fabrication. Simulated replies verify app sequencing only.

Qualification trace required before claiming this firmware works: report IDs/full bytes/timestamps and annotated released/held actions using applicable official tool; DA/DE framing and duration; typing unlock; unchanged configuration + RGB; persistence after controlled reconnect/power cycle; cleanup/recovery. Do not use unknown opcodes or flash firmware to fill gaps.

## Sources and reproducibility

First-party distribution and instructions:

- https://www.nizkeyboard.com/pages/firmware-upgrade (live page links separate CalibrationLite Windows/Mac downloads).
- https://www.nizkeyboard.com/blogs/news/some-keys-doesn-t-work-look-at-this (live article recommends CalibrationLite; detailed old illustrated steps are missing).
- Older DLL https://drive.google.com/file/d/17WC9k3V1y9xSvDaBNjdLQc4mr36xaWeY/view ; direct read https://drive.google.com/uc?export=download&id=17WC9k3V1y9xSvDaBNjdLQc4mr36xaWeY .
- Older EXE https://drive.google.com/file/d/1Y4ADVyphBBKAMaqh_u1JI913lVWnj7mt/view ; direct read https://drive.usercontent.google.com/download?id=1Y4ADVyphBBKAMaqh_u1JI913lVWnj7mt&export=download&confirm=t .
- 2026 archive https://drive.google.com/file/d/1HlD3fvHx5obZDIfgJ2PJogi16xE0I7Bo/view ; direct read https://drive.usercontent.google.com/download?id=1HlD3fvHx5obZDIfgJ2PJogi16xE0I7Bo&export=download&confirm=t . Members: `NIZ-EC-CalibrationLite-2026/KBDLL.dll`, `CalibrationLite.exe`, `使用说明Instructions.txt`.

Pinned source revision `395a8f8e46ab677ecf30536c3e756ab7e68c2683`:

- https://github.com/cbingb666/niz-web/blob/395a8f8e46ab677ecf30536c3e756ab7e68c2683/src/calibration.ts#L42-L50 (packet constructor and proposed strict reply predicate), #L139-L165 (explicit release/press/action sequencing), #L174-L186 (cleanup). This is corroborating client code, not firmware proof.
- Local pinned `/private/tmp/niz-analysis-20261010/public/niz-web/model.ts:24-27` declares firmware-family matching and calibration candidate, not exact hardware validation. `/private/tmp/niz-analysis-20261010/public/niz-web/hid.ts:479-483` makes current public availability capability/descriptor based, without a separate exact-firmware whitelist. That policy is not this target's qualification result.
- https://github.com/cbingb666/niz-web/blob/395a8f8e46ab677ecf30536c3e756ab7e68c2683/docs/calibration-research.md documents its prior investigation; this report independently verified the vendor binaries/instructions rather than relying on its addresses alone.

Reproduction without saving/executing vendor binaries: Python urllib reads URL bytes, verifies hashlib.sha256; `subprocess.run(['objdump','-p','-'], input=bytes)` and `objdump -d --start-address=0x100012d0 --stop-address=0x10001410 -` inspect DLL requests; `objdump -d --start-address=0x100010f0 --stop-address=0x10001207 -` inspects notifications. For RAR, `tar -tf -` inventories and `tar -xOf - MEMBER` reads each member to stdout. EXE ranges `0x402190..0x402327` and `0x4024b0..0x4026c5` establish callsites. PE IAT/map entries above were independently decoded with Python struct.

## Additional first-party native Mac evidence (static, no launch or mount)

The live official firmware page's CalibrationLite link leads to folder `10brHTpjXS-cU2HYCsKpm9cFY5skUeEDQ`, then its Mac subfolder `16MtzGdINrANBTKhv2xSNrI4aOMlJtG4S`. That folder lists **MACv1.0.2.dmg**, Drive file ID `1s7pXSJKZEPN6cIq1eYs5VYSHDvyXS024`:

- https://drive.google.com/drive/folders/16MtzGdINrANBTKhv2xSNrI4aOMlJtG4S
- https://drive.google.com/file/d/1s7pXSJKZEPN6cIq1eYs5VYSHDvyXS024/view
- Direct byte read: https://drive.usercontent.google.com/download?id=1s7pXSJKZEPN6cIq1eYs5VYSHDvyXS024&export=download&confirm=t
- DMG: 449,045 bytes, SHA256 `c0614de6ef28d7b5dc10cf55df32946ef47663318cc319aa512b702462848853`.
- Real app member: `键盘校验工具V1.0/键盘校验工具v1.02.app/Contents/MacOS/USBDeviceProject`, 62,256 bytes, SHA256 `cea0a8d2bd8e1ed57c34a6e5c7a19fdffe2d1cce352fbc42696513b365ab0ef3`.
- Mach-O 64-bit x86_64, app version 1.02/build2, macOS minimum10.12, SDK10.14. This historical binary is not an Apple Silicon native app. Do not mistake the hidden `.Trashes/501` older 62,048-byte app for the actual v1.02 bundle.

Read-only reconstruction used the DMG's `koly` trailer XML offset443119/length5414, blkx zlib chunks and HFS+ catalog, entirely in memory. HFS partition starts at sector64 (byte32768), allocation block4096, catalog extent684/47; the actual executable is catalog fileID100 with a single data extent1206/16. No image was mounted and no executable/bundle was saved or opened. `objdump --macho --objc-meta-data --indirect-symbols -` and `objdump --macho -d -` accepted the reconstructed bytes on stdin. Mach-O mode ignores the general start/stop address options in this tool version; output was filtered numerically in Python.

**Native receive framing is stronger evidence than the Windows raw-buffer comparison.** IOHID's input callback at `0x10000275c` stores the report pointer from r9; at `0x1000027d7` it forwards that exact pointer as rdx, and at `0x1000027da` the original callback length as rcx to `processInput:withLength:`. At `0x100002c2d`, that method constructs NSData from the same pointer/length without adding or removing a byte. `didReceiveNewInputData:` is implemented at `0x100001995`; it hex-encodes these native report bytes and tests:

| Test | Native code VA | Native report prefix |
|---|---:|---|
| Firmware response | `100001a23` (`hasPrefix:`, call `100001a34`) | `00 F9` |
| Release completion | `100001b2c` (`hasPrefix:`, call `100001b3a`) | `00 DA DA` |
| Press completion | `100001b80` (`hasPrefix:`, call `100001b8e`) | `00 DE DE` |

This directly corroborates native payload[0]==0 and payload[1]==DA/DE, and reveals duplicated completion opcode at payload[2] in the Mac client's expected form. Its prefix tests do not require 64-byte length or validate remaining padding. Therefore the proposed 0/DA and 0/DE predicate now has first-party native-client support; actual target timing/full response length/tail and whether payload[2] repeats still need a real V1.0.48 trace. Do not strengthen the parser to require the third byte solely from this different client's target.

**Native output also corroborates opcodes, with a length difference.** `lockKeyBoard` starts at `100001d18`; its async block references hex `00d90000` at `100001d8f`. `unlockKeyBoard` starts `100001e26`, references `00d90100` at `100001e3a`. Release UI method starts `100001ebc`, block references `00db00` at `100002033`; press method starts `100002114`, block references `00dd00` at `10000228b`. These invoke `sendOutputReport:error:` (`100002938`), not its separate feature-report method.

Output wrapper `100002989` reads the NSData's first byte. `100002992: 41 80 ff 01` / `100002996: 48 83 d8 00` subtract one from length when that first byte is zero; it uses that byte as report ID at `10000299a`, report type1(Output) at `1000029ae`, and passes the original NSData to `sendMessageToHostWithData:reportPage:reportType:length:error:` (`100002a78`). That helper obtains the original bytes at `100002ac1` and passes that pointer unchanged in rcx at `100002b25` to **IOHIDDeviceSetReport** at `100002b2c` (stub100003e54). Consequently this native client submits report ID0 and **2-byte `00 DB`/`00 DD`**, or **3-byte `00 D9 arg`**, rather than the Windows client's zero-padded64 bytes. It does not strip the leading payload zero; it omits the final zero from the short NSData. This is a client implementation discrepancy, not permission to vary packet length on this user's device. The current identified descriptor/Windows protocol supports retaining the proposed exact64-byte payloads; qualify actual native send/readback rather than copying the old wrapper's length behavior.

**Eligibility caveat strengthened:** real v1.02 `viewDidLoad` explicitly requests VID0483 at `1000015f5: ba 83 04 00 00` and **PID502b** at `1000015fa: b9 2b 50 00 00` before `startMonitoringForVendorID:andProductID:error:`. This is not the user's PID502a. Whether 502b corresponds to another operating mode/revision was not established here. The Mac implementation strengthens protocol/framing evidence but does not qualify calibration on `66EC(XRGB)BLe;V1.0.48;V1.0;`/502a, nor establish Apple Silicon execution compatibility.

Completion handlers call `restorationButton` at `100001b4e` / `100001ba2`, which re-enables the UI buttons at `100002322` rather than transmitting a reset command. This supplies additional reusable-client evidence for explicit later batches. Attach handler automatically invokes lock and version at `1000018f7..10000190e`; deallocation invokes unlock at `100001759`, and viewDidLoad registers unlock for application termination. Preserve the proposed explicit-session locking and honest crash/disconnect cleanup limits instead of copying these automatic behaviors.

The top-level item named `CalibrationLite（mac）.rar` (DriveID `16oUFUqxWPpw70_eirDeG4naZrPsU816-`) is misleading: its members are `CalibrationLite.exe`, `KBDLL.dll`, `mfc100ud.dll`, `msvcr100d.dll`. It is a Windows archive, 3,243,522 bytes, SHA256 `3d9f3a9d1f8fef94eab96628339d328e68cab708065bb1f95c25f830983cc589`; it was not used as native Mac proof.
