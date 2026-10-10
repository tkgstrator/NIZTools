# Atom66 trigger-point switch: bounded read-only findings

Host verified: `devonlys-MacBook-Pro.local`. Actual identified target is `66EC(XRGB)BLe;V1.0.48;V1.0;`, VID0483/PID502a, usage008c/1. Only the supplied manual, static vendor binaries, pinned public source, current Rust source, and an existing read-only hardware baseline were read. No HID requests, source edits, firmware work, or Git operations occurred. This report is the only file written.

## Immediate result for the visible UI

**Right Fn + physical 7/& is already configured to invoke the vendor's trigger-point-switch action on this actual keyboard.** No keymap write is needed to use it. The fastest evidence-supported UI is a visible hardware-shortcut guide with an A-key typing check, showing **current setting unknown**. Do not present a software High/Low setter, millimeter slider, current-stage badge, or a USB command button as working: no direct setter/readback has been established.

There is also an **assignable vendor action code149 (`0x95`)**, named “Adjust Trigger Point” by the Windows client. A GUI can stage that action on another key through the existing key-configuration transaction; physically pressing that mapped key then invokes the firmware action. This is a key assignment, not an immediate USB threshold setter. It is unnecessary for this user's already mapped Right Fn +7 and changes persistent key settings, while the selected trigger stage is documented as volatile.

## Primary manual proof

Source: `/private/tmp/niz-analysis-20261010/User Manual/66EC(XRGB)Ble User Manual.doc`; its existing extraction `/private/tmp/niz-analysis-20261010/manual-agent/66EC-XRGB-Ble.txt:1-5` identifies the XRGB BLE manual and the **Right FN+** function table. Lines59-62 list physical `7&`, “Adjust trigger point,” high/low stages, lack of shutdown memory, and a default described as a suitable trigger point. Lines63-65 separately describe Esc keyboard locking.

Supported instruction: hold **Right Fn** and press **7/&** to switch the trigger setting; release, test A with normal light presses, and toggle again if the result is unsuitable. This is a global firmware function, rather than a per-A numeric adjustment. The manual does not prescribe a long hold, a specific duration, indicator-flash count, a key force/travel distance, or a USB protocol. Do not borrow another shortcut's 3-second hold or flash count.

The manual's wording supports high/low choices and non-persistence after shutdown; it does not identify this exact firmware version, numeric millimeter values, which polarity is shallower/more sensitive, or a dependable indicator of the selected state. Describe the action as a switch and let the user compare response. Do not infer a state from how many times the app's guide was opened/clicked. Stage behavior on V1.0.48 is still an actual-device check.

## Vendor action149 proof and actual baseline

Inspected EXE: `/private/tmp/niz-analysis-20261010/Software/66EC(XRGB)Ble.exe`, SHA256 `3189ce0a95911b7cbeffd6a15092f38af490aa8bf2d90115453d11f401974555`. Preferred image base00400000; `.text` RVA1000/raw400; `.rdata` RVA187000/raw185600.

- At **VA0042517f / raw0002457f**, bytes `c7 83 b8 3a 00 00 95 00 00 00` store action149 into the action-table entry.
- At **VA00425194**, `68 20 34 5b 00` references Chinese label **按键触发行程切换**, VA005b3420/raw001b1a20.
- At **VA004251d5**, `68 34 34 5b 00` references **Adjust Trigger Point**, VA005b3434/raw001b1a34, in the same entry's label initialization.
- Static evidence is already available in `/private/tmp/niz-analysis-20261010/manual-agent/utility-disassembly.txt`; exact strings in `utility-utf16-strings.txt:731`.

Pinned public revision `395a8f8e46ab677ecf30536c3e756ab7e68c2683`: `/private/tmp/niz-analysis-20261010/public/niz-web/key-names.ts:101` labels149 “Actuation travel”; `legacy.ts:56` maps “adjust trigger point” to149. Current Rust `/Users/devonly/Developer/personal/NIZ/crates/niz-core/src/keys.rs:153` contains the same name, and line244 maps `adjusttriggerpoint` to149. These source names corroborate the vendor table; “Actuation travel” is an action label, not measured travel.

**Actual-device assignment evidence:** the saved `/private/tmp/niz-implementation-20261010/hardware-baseline.json` contains hex-encoded64-byte key records. Physical7 is key position8 (Esc=1, number7=8). Decoding its records gives:

| Layer | Physical position8 record type/count | Value |
|---|---|---|
| Normal/group1 | type0/count1 |21 (ordinary7) |
| Right Fn/group2 | type0/count1 |149 (trigger-point switch) |
| Left Fn/group3 | type0/count1 |8 (F7) |

This confirms the shortcut is in the keyboard's captured key configuration and identifies **Right Fn**, rather than treating either Fn as equivalent. It does not expose the currently selected threshold. The existing baseline has198 reports, counters EMPTY[], and no lighting field.

## USB boundary and calibration distinction

The supplied HWI DLL's22 exports and traced reports cover versionF9, key definitionsF2/F0/F1/F6, lightingE2/E0/E1/E6, countersE3, and firmware transfer. None is an identified trigger-setting setter/getter. Type0 serializer at **VA10004390** writes `[00,F0,layer,key,00,count,values...]`; code149 can therefore be transported as a mapping value, not as a standalone opcode. Do not send byte95 as a command.

Calibration is separate: official KBDLL CalibrationInit sends **DB** for all released keys, and CalibrationPress sends **DD** with no key ID/mask. Holding A physically during DD is pressed-sensor calibration, not a numeric threshold request or a software-selected A target. Neither DB nor E8 is proven as a trigger-depth setter. See `/private/tmp/niz-implementation-20261010/calibration-research.md` for the exact calibration sequence and limits.

No analyzed version/keymap/light/counter receive path decodes a current global trigger stage. Unknown/ignored response fields do not prove a hidden readback method. The pinned actuation investigation agrees, but targets different EC-S/V1.4.4 hardware and cannot qualify this user's firmware:

https://github.com/cbingb666/niz-web/blob/395a8f8e46ab677ecf30536c3e756ab7e68c2683/docs/actuation-readback-research.md

## Small UI recommendation and acceptance checks

Show an obvious **Trigger-point switch / Sensitivity** guide near A testing or calibration, with a keyboard shortcut illustration **Right Fn +7**. State that it affects the keyboard's trigger setting, the current stage is not readable, and the manual says shutdown returns the default. The guide itself sends no device command. A simple local text area can let the user compare A response before/after the physical shortcut; do not turn a missed press into a sensor diagnosis.

Possible concise Japanese text: “右Fnを押しながら7を一度押すと、トリガー設定を切り替えます。キーを離してAの反応を確認してください。現在の段階はPCから取得できません。説明書では、電源を切ると既定の設定に戻ります。” Avoid labels that imply a measured high/low state or a confirmed sensitivity direction.

Offline checks: opening/closing guide sends nothing; explanation names Right Fn/physical7, not Left Fn; no DB/DD/E8/new raw opcode emitted; typing check stays usable and does not persist typed text; no stage value inferred from clicks; no threshold included in keymap backups; existing drafts stay intact. If offering action149 in the assignment editor, clearly stage it as a key assignment and use the existing backup/full-write/readback route.

Next actual-device check requires only the user performing the documented shortcut and comparing A response; it does not require USB writes, calibration, reset, flashing, or new firmware analysis. If a future current-stage display is wanted, use a separately authorized full-traffic A/A and A/B/A comparison with established reads; do not scan commands.

First-party source chain: https://www.nizkeyboard.com/pages/firmware-upgrade → official Atom66 Drive folder https://drive.google.com/drive/folders/1uFy1kPoQUHyo8z_4PXmxHVeTmqxKPgX0 → manual folder https://drive.google.com/drive/folders/18UQ4VyLRHLeljHfmk_KgcIS-pS1vxnSK . The supplied XRGB BLE manual is the direct variant-specific evidence. Pinned action sources: https://github.com/cbingb666/niz-web/blob/395a8f8e46ab677ecf30536c3e756ab7e68c2683/src/key-names.ts#L101 and https://github.com/cbingb666/niz-web/blob/395a8f8e46ab677ecf30536c3e756ab7e68c2683/src/devices/legacy.ts#L56 .
