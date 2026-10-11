# Atom66 selected-key threshold: vendor UI, manual and host-client evidence

Date: 2026-10-11. Verified native host: `devonlys-MacBook-Pro.local`. Verified readable target workspace: `/Users/devonly/Developer/personal/NIZ`. Assignment scope is independent, read-only vendor/manual/host-client analysis for `66EC(XRGB)BLe;V1.0.48;V1.0;`, VID `0483`, PID `502a`, usage `008c/1`; the coordinator has a separate exact-firmware analyst. LocalGPT and Claude model/ask tools were absent from the current tool inventory. Native file/command and primary Web tools were used. No executable was loaded, no native GUI was launched, no HID/firmware/unknown command was sent, and no repository/Git file was changed.

## Decision

**The inspected vendor path does not establish a software-selected-key input-threshold setter or getter.** It establishes (1) selecting a physical key to assign an output or function, including action 149, and (2) a separate calibration operation whose pressed-key scope is determined by what the human physically holds. Neither host operation sends the clicked key together with a numerical actuation value.

The user's requirement is stronger than choosing a holding guide: click a lower-board position, adjust **only that key's** input threshold, and remove the global threshold explanation. The prior guide-only proposal does not satisfy it. No functioning slider, numeric field, or claim that software selection targets `DD` is justified by this evidence.

This is a bounded negative result for the inspected clients/manuals. It is not proof that the exact firmware contains no independently mutable per-sensor threshold or no unused command. Exact firmware scan/calibration/storage semantics remain the separate analyst's task.

## New primary UI evidence from the original model manual

The original document is `/private/tmp/niz-analysis-20261010/User Manual/66EC(XRGB)Ble User Manual.doc`, 751,104 bytes, SHA-256 `e07495d4a738e1b26d6b5fb6e4194b39594d3cc17d2b946875e1d6ae4fbffbea`. Fresh `textutil -convert txt -stdout` agreed with the existing numbered extraction at `manual-agent/66EC-XRGB-Ble.txt`.

- Lines 39–62 place the trigger action under Right Fn plus physical 7. Lines 59–62 describe a high/low trigger stage which returns to default after shutdown. There is no chosen-key or numerical value procedure.
- Lines 152–164 describe layers, reading/writing key configurations and single/combo output assignments. Lines 216–221 tell the user to select a key, open Function, choose a function, and confirm. The selected physical key is the location to which the function is assigned.

**Fresh visual inspection:** the `.doc` compound file was parsed statically, following its FAT/stream chains. Eleven PNG resources in its `Data` stream were extracted to this report's temporary directory, each with PNG chunk CRC verification. No Office/vendor runtime was used. Important screenshots:

| Manual resource | Stream-relative offset | Size | Observed UI |
|---|---:|---:|---|
| [Main programming screen](manual-images/Data-image-02-offset-3a995.png) | `Data+0x3a995` | 116,599 bytes, 1017×611 | A physical layout, layer selectors, WriteKeys/LightSet/ReadAll/ReadCounter controls, and selected Esc's single/function assignment summary. No selected-key sensitivity or actuation control. |
| [Key Settings / Function](manual-images/Data-image-08-offset-84e4c.png) | `Data+0x84e4c` | 45,694 bytes, 579×370 | Tabs Single/Com, Macro, Emulate, Multimedia, Mouse, Function, Power; Function is a grid of assignable keyboard/backlight actions with OK/Cancel. No threshold-value field. |

The resource index is [manual-images/ole-index.json](manual-images/ole-index.json). These are original manual illustrations, not a newly executed Windows application or an exact-firmware runtime result. They are stronger UI evidence than merely finding no sensitivity string in a resource table.

## Fresh exact host utility evidence

`/private/tmp/niz-analysis-20261010/Software/66EC(XRGB)Ble.exe`: 2,447,872 bytes, SHA-256 `3189ce0a95911b7cbeffd6a15092f38af490aa8bf2d90115453d11f401974555`.

Exact byte search, plus PE section mapping, confirms Chinese caption at raw `0x1b1a20`/VA `0x005b3420`, and English **Adjust Trigger Point** at raw `0x1b1a34`/VA `0x005b3434`. Bounded fresh native `objdump` inspection at `0x00425173..0x00425240` confirms:

- `0x0042517f`: `c7 83 b8 3a 00 00 95 00 00 00`, stores action `0x95`/149 at object offset `0x3ab8`.
- `0x00425194` and `0x004251d5`: reference those captions for the same action entry.
- `0x00425210..0x00425231`: write entry rectangle bounds (223,112)-(259,148) at object offsets `0x3a9c..0x3aa8`.

This is the action-grid entry consistent with the manual. It does not show a dialog parameter such as millimeters, capacitance, percentage, or a per-key stage. Absence of other labels is not relied on as a proof of firmware capability absence.

`66EC(XRGB)BleHWI.dll`: 66,048 bytes, SHA-256 `1ca6c2141ff2aac1d38c03533ac56a64c27beb9e4e82eae9a1f6c9abf5c108d3`. Fresh export inspection finds 22 exports for key configuration, lighting, counters, version, firmware, discovery, and hook support; no identified threshold or calibration export.

Fresh serializer disassembly `0x10004390..0x10004405` shows normal assignment payload `00 F0 layer key 00 count values...`: the layer and key index are written at `0x100043d5` and `0x100043db`; type 0 at `0x100043de`; count at `0x100043ea`; assigned action bytes at `0x100043f1..0x100043f7`. **The presence of a key index here is for the output/function assignment. Sending action149 as that key's assignment does not make the action's later trigger-switch effect key-specific, nor is149 an independently established command opcode.**

## Calibration instructions and actual sender parameters

The current [manufacturer firmware/download page](https://www.nizkeyboard.com/pages/firmware-upgrade) independently lists Atom66 tools and a separate CalibrationLite download. The current [trigger-problem article](https://www.nizkeyboard.com/blogs/news/some-keys-doesn-t-work-look-at-this) recommends calibration as a repair procedure; its rendered/raw article content now omits useful detailed steps after the download link. The [manufacturer FAQ](https://www.nizkeyboard.com/blogs/news/keyboard-faq-solution) also links calibration without documenting selected-key actuation values.

The first-party 2026 distribution was freshly read into memory from [NIZ-EC-CalibrationLite-2026.rar](https://drive.google.com/file/d/1HlD3fvHx5obZDIfgJ2PJogi16xE0I7Bo/view), direct URL `https://drive.usercontent.google.com/download?id=1HlD3fvHx5obZDIfgJ2PJogi16xE0I7Bo&export=download&confirm=t`. Archive size 5,417,486, SHA-256 `87d1f2ac8f77c99244fa8cffc23c7ef9987aa8b1f4a0a8c41c4e1c8d53dc63ce`. `tar -tf -` and `tar -xOf - MEMBER` inspected only the instructions and KBDLL in memory; no vendor binary was extracted to disk.

`NIZ-EC-CalibrationLite-2026/使用说明Instructions.txt`: 1,060 bytes, SHA-256 `0a03d04e337e23d7b72ef3c75e3c7c5d245d289b0f6dbbd23966c9db67212bf6`. Its bilingual procedure says to connect, confirm connected status, perform release calibration once with **all keys released** (this calibrates all keys), then physically hold one or several abnormal keys, invoke press calibration, and keep them held until completion. It gives no software-selected position, depth to hold, relative sensitivity control, millimeter/percentage/capacitance unit, per-key value readback, calibration recovery, persistence guarantee or compatibility matrix.

The extracted-in-memory KBDLL is 17,408 bytes, SHA-256 `4a73d74f378b5d6c0b731257eb6432482e8184efc27e7f611161db8a6e12fe32`. Fresh disassembly was saved as [calibration-request-disassembly.txt](calibration-request-disassembly.txt):

| Export | Positive byte evidence | Software key/depth input |
|---|---|---|
| `CalibrationInit`, `0x10001320` | Clears65 bytes at `0x10001335..0x1000133d`; writes `00 DB` at `0x10001348`, zero argument at `0x1000134e`; sends buffer. | None. |
| `CalibrationPress`, `0x10001370` | Clears65 bytes at `0x10001385..0x1000138d`; writes `00 DD` at `0x10001398`, zero argument at `0x1000139e`; sends buffer. | None. |

Windows framing includes a leading report ID0. Normalized payloads are zero-padded `00 DB 00...` and `00 DD 00...`. Both exports take no arguments other than their member-object pointer; they neither insert a key index/mask nor a threshold value. The existing research documents DA/DE completion and native Mac client differences, but completion alone does not supply a value/readback.

## Can holding a key at a desired trigger depth set its threshold?

**Not established by the vendor procedure or host client.** Physically holding an affected key is documented; holding it at a *chosen actuation depth* is not. The instructions do not say whether the press step samples a fully depressed endpoint, stores a threshold directly, derives an endpoint range, adjusts only keys it currently classifies as pressed, or requires an existing threshold to detect those keys. The client's zero-argument DD call cannot answer those firmware questions.

It would be incorrect to turn “hold the selected key” into “hold it at 40%/1.5mm and save its threshold,” or to call DD a key-only threshold setter just because only one key is held. At minimum the exact firmware's update-selection rule, value transformation and persistence behavior must be established first. Even if firmware analysis proves a per-held-sensor numerical change, the app would still need a mechanism to guarantee the affected physical set matches the clicked key before claiming software selection protects all other keys.

## Exact-target versus family qualification

The supplied manual/utility explicitly target the Atom66 XRGB BLE model, but they do not certify calibration semantics on exact `V1.0.48` or identify an API for this user's `502a` interface. CalibrationLite is a separate family-level distribution. The existing native Mac calibration report identifies its historical client's PID as `502b`, so native Mac UI evidence cannot qualify `502a` hardware behavior.

Current [Atom68 product documentation](https://www.nizkeyboard.com/collections/atom/products/niz-2019-new-atom-68-ec-bluetooth-keyboard-rgb-or-non-rgb) describes 2/3mm trigger switching for that different model; other current models advertise different trigger distances. Those family marketing values cannot be assigned to the inspected Atom66's high/low stages. Newer model documentation/rapid-trigger features cannot supply a chosen-key setter or units for exact `V1.0.48`.

## Minimal truthful UI and remaining prerequisites

No enabled key-only threshold adjustment can yet be supported from this evidence. The lower physical-layout selection may identify a position, and the global shortcut explanation may be removed as requested. If an implementation is made before protocol qualification, selected-key threshold controls must be explicitly unavailable, with a short accurate reason. That is an honest interim state, **not completion of the user's desired adjustment feature**. A holding-guide/calibration panel would remain a different capability and is expressly rejected as the requested solution.

To enable a real selected-key threshold UI, obtain either an existing vendor operation/capture for exact `V1.0.48` that sends key identity plus threshold, or exact-firmware evidence of an accessible stored per-key field/command with the following acceptance facts:

1. The selected physical position maps to the correct sensor independently of remapped output labels.
2. Its write affects only that sensor; all other sensor thresholds/baselines are proven unchanged.
3. Values and bounds have established meaning; do not invent mm or percentage mappings.
4. Readback confirms the actual value or a separately qualified physical procedure confirms the intended trigger point.
5. Persistence, rollback/recovery, command completion and exact-interface eligibility are known sufficiently to implement the authorized workflow.

A read-only screenshot/manual walkthrough or passive capture of a genuine existing vendor operation is an appropriate source of such evidence. Sending unknown opcodes, assuming unused bytes are fields, running/flash-upgrading the vendor tool, or presenting a guide as selected-key hardware targeting does not close the gap.

## Verification boundary

Verified the intended host and source paths, original artifact hashes, model manual text, actual manual screenshots, EXE action-entry bytes, HWI export/assignment serializer bytes, current primary vendor links and 2026 instruction/DLL hashes and DB/DD constructors. This review intentionally contains no hardware trial, vendor runtime claim, new analog result or test/GUI completion claim. All created outputs are under `/private/tmp/niz-perkey-20261011/vendor/`.
