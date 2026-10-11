# Arbitrary selected-key threshold capability: independent bounded review

Date: 2026-10-11. Native host/workspace verified: devonlys-MacBook-Pro.local, /Users/devonly/Developer/personal/NIZ. This fresh review read the supplied target-specific manual, exact EXE/HWI bytes, saved target baseline and pinned public source, and fetched only the official older KBDLL into memory for independent disassembly. Vendor binaries were never loaded/executed; no GUI, HID, firmware, OS setting, Git or repository source mutation. This report is the only output file for the capability task. Preferred LocalGPT/Claude tools are unavailable; this is the explicitly authorized independent Codex route.

## Result and boundary

**No proven USB numerical/per-key actuation-threshold setter or getter is available for 66EC(XRGB)BLe;V1.0.48;V1.0; from the inspected paths.** The known Windows utility feature is an assignable trigger-switch action149, not a numerical threshold attached to the selected physical key. The known calibration protocol can act on physically held keys, but it sends no software-selected key ID/mask and no threshold value. Those are separate operations.

This is a bounded “not established” result. It does not prove that the firmware has no hidden field/command or that some different/newer vendor tool cannot expose one. No runtime Windows GUI or exact V1.0.48 firmware command dispatcher was inspected. The current family-level client/manual evidence cannot be promoted into exact firmware analog qualification.

## Fresh exact artifact and byte evidence

### Target-specific manual and actual saved baseline

Fresh native `textutil -convert txt -stdout` of `/private/tmp/niz-analysis-20261010/User Manual/66EC(XRGB)Ble User Manual.doc` agrees with the existing numbered extraction. Original 751,104 bytes, SHA-256 e07495d4a738e1b26d6b5fb6e4194b39594d3cc17d2b946875e1d6ae4fbffbea.

`manual-agent/66EC-XRGB-Ble.txt:39-62` places trigger switching in the Right Fn function table at physical7/& and describes high/low stages, with reset to the default after shutdown. It gives no per-key selector/number/USB query. :152-164 and :213-220 describe clicking a position, assigning an output/function and writing key configurations; :181-184 numerical timing is macro delay, not sensor actuation. Treat RightFn + 7 as the manual's general trigger-stage shortcut, without claiming the selected GUI key constrains it or inferring mm values/current stage.

Fresh decode of `/private/tmp/niz-implementation-20261010/hardware-baseline.json` identifies the saved target as firmware66EC(XRGB)BLe;V1.0.48;V1.0;, VID0483/PID502a/usage008c/1. Record index7 is group1/key8/type0/count1/value21; index73 is group2/key8/type0/count1/value149; index139 is group3/key8/type0/count1/value8. Thus RightFn + physical7 already maps action149 in this captured snapshot. This is saved read-only evidence, not a new current-device read or a threshold-stage value.

### Actual 66EC Windows configuration utility

Read `/private/tmp/niz-analysis-20261010/Software/66EC(XRGB)Ble.exe`: 2,447,872 bytes, SHA-2563189ce0a95911b7cbeffd6a15092f38af490aa8bf2d90115453d11f401974555. Fresh UTF-16 scanning finds exactly the relevant trigger captions at raw0x1b1a20 “按键触发行程切换” and raw0x1b1a34 “Adjust Trigger Point”. Fresh bounded disassembly at VA00425173..00425237 shows:

- VA0042517f / raw0x2457f: `c7 83 b8 3a 00 00 95 00 00 00` stores action0x95/149 into the entry.
- VA00425194 / raw0x24594 references Chinese caption VA005b3420.
- VA004251d5 / raw0x245d5 references English caption VA005b3434 in the same entry.
- VA00425210..00425231 stores one rectangular entry's bounds at object+0x3a9c..0x3aa8 (223,112,259,148). This supports a selectable action entry, not a per-key numeric parameter form.

Fresh resource-directory parsing enumerated nine RT_DIALOG templates: IDs102,129,134,135,137,138,139,30721,30734 at raw0x2258dc..0x225ce0. All application templates have empty static captions/controls; only the MFC New dialog contains normal New/OK/Cancel/Help labels. Therefore controls are largely constructed dynamically. Absence of a sensitivity label in static dialogs alone is not proof that a UI can never exist. The positive action-table proof plus imported writer shape is stronger than resource absence.

The EXE imports key get/set/read/write, lighting get/set, counters/version, connection/hook/firmware methods from HWI. No threshold-named API is imported. The manual describes Function Key selection rather than separate sensitivity levels for the selected key. **No per-key sensitivity selector is demonstrated by these inspected Windows utility paths; the observed trigger item assigns the switch action to a position.** Installing/running the GUI was outside this review.

### HWI sender/getter boundary

Read `66EC(XRGB)BleHWI.dll`: 66,048 bytes, SHA-2561ca6c2141ff2aac1d38c03533ac56a64c27beb9e4e82eae9a1f6c9abf5c108d3. Native PE export inspection confirms22 exports. Key get/set exports are RVA0x2160/0x1d90, version0x23d0, counters0x23f0, key read/write0x2390/0x2380 and lighting0x23a0/0x2040/0x22e0. None is an identified per-key threshold getter/setter.

Fresh type0 serializer disassembly VA10004390..10004405, .text RVA0x1000/raw0x400, proves:

- VA100043cd / raw0x37cd writes normalized prefix00 F0.
- VA100043d5 / raw0x37d5 writes layer into payload[2]; VA100043db / raw0x37db writes key into[3]; VA100043de writes type0 into[4].
- VA100043ea / raw0x37ea writes count into[5]. VA100043f1..100043f7 / raw0x37f1..0x37f7 copies action values into[6+].

That establishes how149 is transported as an assignment value. It does not authorize sending0x95 as a command, and it carries no demonstrated analog threshold parameter. Existing unused bytes cannot be assumed to hold one. Current Rust `editor.rs:237-253` also restricts real hardware trials to basic output codes1..74 and excludes Fn-bearing columns: introducing action149 to the current bounded apply UI would require a separately reviewed expansion, not merely another dropdown label.

### Calibration: physically held target, no selected key parameter

Fresh in-memory download from the official older file `https://drive.usercontent.google.com/download?id=17WC9k3V1y9xSvDaBNjdLQc4mr36xaWeY&export=download&confirm=t` yielded KBDLL.dll17,408 bytes with expected SHA-256c40e164fdbc022ad96b786c78bfbc0bf8aede527a72a8991598c3807c9a71b5f. `objdump -p -` independently confirmed12 exports; CalibrationInit RVA0x1320 and CalibrationPress RVA0x1370 are zero-argument member methods.

Fresh disassembly proves both clear65 bytes with memset (VA10001335..1000133d and10001385..1000138d). VA10001348 / raw0x748 stores00 DB and VA10001398 / raw0x798 stores00 DD after the Windows report-ID byte; raw0x74e/0x79e writes zero argument. Remaining bytes stay zero. Windows raw reports include an extra leading report-ID0; normalized64-byte payloads are00 DB00… and00 DD00…. No key ID/mask or number is inserted. The client uses physically held keys for the press calibration step; software selection can only guide the human.

This corroborates the existing exact calibration analysis rather than repurposing it into sensitivity setting. It does not prove how V1.0.48 derives/stores per-sensor calibration, a chosen actuation depth, or analog recovery. Do not instruct holding at an invented depth/percentage or present a millimeter slider based on DB/DD.

## Primary web/source cross-checks

Freshly opened [manufacturer firmware/download page](https://www.nizkeyboard.com/pages/firmware-upgrade) still separates Atom66 tools and CalibrationLite downloads. Its [key-trigger support article](https://www.nizkeyboard.com/blogs/news/some-keys-doesn-t-work-look-at-this) points to calibration as a repair avenue, without a selected-key numeric threshold API. The [manufacturer FAQ](https://www.nizkeyboard.com/blogs/news/keyboard-faq-solution) likewise links capacitance calibration; it does not specify per-key actuation levels.

The pinned [niz-web actuation-readback investigation](https://raw.githubusercontent.com/cbingb666/niz-web/395a8f8e46ab677ecf30536c3e756ab7e68c2683/docs/actuation-readback-research.md) explicitly distinguishes trigger-action assignments, calibration data and travel, and finds no identified query/decoder. It targets different EC-S/PID522a/V1.4.4 hardware, so it is corroboration of the client boundary only. Native pinned `public/niz-web/protocol.ts:113-180` encodes assignment/macro values and delays; `key-names.ts:101` and `legacy.ts:56` label149 as trigger travel/switch. Naming149 does not create per-key threshold readback.

## Supported generic GUI plan addendum

The proposed “反応の調整” view is appropriately bounded if it explicitly preserves these capability distinctions:

- Remove the A-only typing card because the separate readonly Input Test now exists. Keep a compact RightFn + 7 general shortcut guide with current stage unknown; changing the selected calibration guide key does not change this shortcut's scope.
- Show a generic experimental calibration step card and a66-position physical-layout picker BELOW it. Initial selectionA can reflect the reported symptom, but allow any physical position. Labels must come from immutable official position/legend evidence, not mutable draft outputs or remapped OS input. The selected position is a holding instruction only; DB still handles all released keys and DD still handles the keys actually held.
- Disable guide selection while any operation, confirmation or active calibration owns the procedure. Freeze the chosen guide at start. **P2 plan condition:** after terminal completion, selecting the next key must not relabel the previous result as if the new key had been calibrated. Snapshot the guide for the run, or keep terminal results explicitly generic/device-level; allow the next selection separately.
- State clearly that per-key numeric threshold setting/current values are not supported by the established protocol. Do not add functioning-looking numeric controls, send a position mask, expand the worker API, or claim an exact analog result solely from DA/DE.
- Preserve existing target/backup/consent/pointer-only stages, cancel/unlock evidence and no automatic retries. Selection alone sends nothing and leaves Editor baseline/draft/history/selection unchanged. Both themes and default960x820/min700x580 reachability must remain. A real arbitrary-key calibration is still an exact-target experiment, not something performed by this read-only review.

Plan acceptance: different selected slots change only physical holding instructions; remapped/duplicate outputs do not change physical slot; busy/modal/active state freezes selection; terminal result provenance remains accurate; guide opening/selection sends no worker/HID request; DB/DD bytes and existing calibration gates remain unchanged. No new hardware feature is needed for this truthful generic guidance.

## Meaningful next step for genuine per-key threshold control

First obtain evidence that the exact target's official utility offers a per-key actuation/sensitivity operation distinct from ordinary action149 assignment and CalibrationLite. A read-only screenshot/manual walkthrough or passive capture of the user performing that existing operation can establish the trigger/control shape. Capture full directions/timestamps/report bytes before app filtering, and compare repeated unchanged readings with a controlled change of exactly one key; do not scan unknown opcodes, reset or flash hardware to search.

If no such vendor UI exists, the next substantive protocol task is static reconstruction of the matching V1.0.48 command/scan/calibration handlers and relevant stored data, using the matching packaged firmware as evidence. Its encoded container must be decoded and validated before interpreting code; newer EC-S/RGB firmware conclusions do not qualify this target. A candidate per-key value requires reproducible exact-firmware evidence before a GUI setter/readback is implemented. Generic physical-key calibration guidance can improve the current UI now, but it does not satisfy the stronger claim “set this key's numeric threshold.”
