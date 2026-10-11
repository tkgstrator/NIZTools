# Arbitrary-key response adjustment: independent capability and plan review

Native host devonlys-MacBook-Pro.local and /Users/devonly/Developer/personal/NIZ verified. Base f78d1617405e91387f03515df469d32c9e6ca5d4; ongoing GUI work inspected read-only. Reviewed current core/device/calibration/controller source, retained calibration/trigger research, supplied EXE/HWI DLL bytes, manual extraction and captured target profile. No source edits, native GUI, HID requests/writes, calibration run, Git mutation or OS changes.

## Capability conclusion

**No numeric or per-key trigger-threshold setter/getter/mask has been established for this target.** The current proven client paths distinguish two different operations:

| Operation | Evidence-supported scope | GUI capability |
| --- | --- | --- |
| Action149 / RightFn+7 trigger switch | Documented keyboard trigger high/low switch; volatile after shutdown; current stage unreadable | Explain the physical global shortcut and current mapping; do not expose a measured value or software threshold setter |
| DB release / DD physically held calibration | Release calibration for every key, followed by calibration while the user physically holds affected keys; outgoing command has no GUI slot/index/mask/value | Generic selected-position guidance is feasible; selection is a physical instruction/marker, not a protocol target |
| Arbitrary per-key numerical threshold | No established host command, unit, range, sensor readback, persistence or qualification | Remains unimplemented/unproven; generic calibration must not be reported as completing this capability |

Absence from the inspected client paths is not proof the firmware can never support it. It is insufficient evidence to implement or claim a working per-key numerical threshold feature.

## Independently checked primary bytes and retained evidence

I directly parsed the actual supplied PE export table and checked the EXE action bytes in this review, without executing either binary. Output: arbitrary-key-vendor-byte-check.json.

- 66EC(XRGB)BleHWI.dll SHA-256: 1ca6c2141ff2aac1d38c03533ac56a64c27beb9e4e82eae9a1f6c9abf5c108d3. Its22 exports cover configuration/version/light/counters/firmware and Windows hook plumbing. No identified threshold-setting/readback export exists. `setKeyConfiguration` and `getKeyConfiguration` are generic key-definition accessors, not analog parameter APIs. Repository bounded flow evidence is docs/reverse-engineering/evidence/dll/report.md:24-37,67-96.
- 66EC(XRGB)Ble.exe SHA-256: 3189ce0a95911b7cbeffd6a15092f38af490aa8bf2d90115453d11f401974555. Raw0x2457f / VA0x0042517f is `c7 83 b8 3a 00 00 95 00 00 00`, storing149. Raw0x1b1a20 / VA0x005b3420 labels it 按键触发行程切换; raw0x1b1a34 / VA0x005b3434 labels it Adjust Trigger Point. These are action-table labels, not a numeric setting or standalone opcode0x95.
- Supplied XRGB BLE manual extraction, /private/tmp/niz-analysis-20261010/manual-agent/66EC-XRGB-Ble.txt:59-65, documents physical7/& under the RightFn function table, two trigger stages and loss of the setting after shutdown; it separately describes keyboard locking. It supplies no per-key choice, numerical units, polarity/sensitivity direction, readback or USB setter.
- Current Rust command builder is crates/niz-core/src/protocol.rs:20-23: zeroed64-byte report with only report[1]=opcode. Device calibration sends command(DB) at calibration.rs:337, command(DD) at361; it supplies no selected key, mask or threshold. D9 lock/unlock uses payload[2]=0/1 (323 and152-154). DA/DE completion is awaited, not decoded as analog values (222-265,341,365).
- Calibration's official Windows DLL is a separate KBDLL.dll, not the supplied HWI configuration DLL. Existing independently verified research records CalibrationInit VA0x10001320/raw0x720, store `00 DB` at0x10001348; CalibrationPress VA0x10001370/raw0x770, store `00 DD` at0x10001398; Keylock VA0x100013c0/raw0x7c0, argument at raw byte3. Receiver compares DA at0x1000118a and DE at0x100011a2. Relevant retained report: docs/reverse-engineering/calibration-research.md:18-36,38-50. Its vendor instructions require all keys released for DB, then one or several affected keys physically held until DD completion.
- Retained official native Mac static evidence also uses short DB/DD commands and DA/DE prefix completion, with no index/mask/value. That client targets PID502b rather than this user's502a and therefore does not qualify target operation: calibration-research.md:115-131.

**Evidence limit:** KBDLL/CalibrationLite bytes were originally inspected in memory and are not present in the supplied fixture directories. I did not redownload or freshly disassemble those absent binaries in this review. The original bounded report is explicitly the source for those addresses; direct fresh byte verification here concerns the retained EXE/HWI DLL. No firmware command handler was decoded and no calibration hardware trace was acquired.

## Material plan findings

### P1: Do not equate generic key calibration with per-key threshold editing

References: device/calibration.rs:268-370; calibration-research.md:48-56; GUI protocol.rs:39-44.

The user asks for arbitrary-key threshold changes. Merely replacing A with a selectable name does not implement a per-key threshold. DB operates on released state for all keys; DD has no selection parameter and relies on physical holds. The app cannot ensure only the selected key changes, verify that the intended key is held, or display a new numeric threshold. Generic UI is useful but must remain labeled calibration, with the numerical threshold capability explicitly outstanding. Do not add a fake High/Low/mm control or infer a selected key's threshold from clicking its diagram.

Do not reinterpret the pressed-calibration step as "hold the key at the desired actuation depth" to simulate a threshold setter. The inspected clients/instructions provide no depth-to-threshold algorithm or proof that partially holding a key requests that actuation point. Ordinary vendor released/pressed guidance is the supported client procedure; arbitrary depth setting remains unqualified.

### P2: The trigger guide must identify the current RightFn function, not assume its printed position

I directly decoded the captured profile fixture in this review. Main/group1 position61 outputs71 (RightAlt); position62 outputs156 (RightFn). Group2 position8 outputs149. Thus the actual current RightFn function starts at position62, whose official legend is RightAlt; the official RightFn position61 is currently mapped differently in the normal layer. `trigger_guide` (editor.rs:270-290) presently checks the group2 action149 but does not identify the normal-layer RightFn position.

When reworking the global guide, derive/show the unique normal-layer single RightFn156 position (with physical position/official legend) or mark it unavailable/ambiguous. Avoid telling this remapped user to press the printed RightFn key. The guide remains global and manual and must not stage an action149 mapping or issue a standalone0x95 command.

### P2: Freeze an instruction target separately from calibration protocol ownership

A selected physical slot should remain0..65 and use its official/physical legend, independently of the current draft output. A remapped physicalA can outputS, but the user must hold the physicalA position. Calibration is per physical sensor, not a remapping layer. Capture the chosen slot/legend when Begin is clicked and keep it stable through pending, AwaitingHeldKeys, AwaitingFinish, cancel/close cleanup and result display; changing an idle selection afterward must not relabel an earlier run's result.

The selection is UI instruction state only. It must not be inserted into Begin/Held request packets, Controller's device Target, generation, Profile records or mask bytes. Existing exact-target/fresh-baseline gates remain. Do not narrow physical selection to basic output assignments: Fn, unassigned, macro and unknown-output positions are still physical keys, and their definitions must be preserved. They may be selected for guidance without qualifying new firmware actions.

## Proposed revised UI plan assessment

The parent's bounded plan is feasible: rename the tab to response adjustment, remove the A-only typing card because the dedicated InputTest tab now handles response observation, keep a compact explicitly global RightFn+7 guide with unknown state, and show generic calibration steps above a66-position selector. DefaultA can reflect the original symptom; every physical slot is selectable. Keep selection local to this view so choosing guidance does not alter Editor/history, and freeze its per-run snapshot.

Make the steps precise: release every key and click Start with the mouse; physically hold only the chosen position and click Continue with the mouse; wait for completion, then release all keys and click Finish. Explain that the screen selection is a guide and the actual held keys determine the press-calibration instruction, while the app does not measure holding. DB's all-key released-state scope should be visible so the UI does not promise a selected-key-only mutation.

Use the existing experimental consent, pointer-only gates and owned-operation cleanup. No additional approval workflow, calibration commands, auto-start, auto-repeat, numerical parameters, simulated key presses or automatic configuration writes are warranted. No actual hardware mutation is needed to implement/verify this presentation change. Exact V1.0.48 analog behavior and persistence remain unverified as before.

## Required state and tests

- Choose A, S, a remapped physical key, Fn, unassigned, macro and unknown-output positions; selection/legend changes guidance only and preserves all raw records/history/request counts/generation/target.
- Verify physical legend is used rather than output name, and calibration wording is independent of assignment/reference layer. Invalid positions cannot be selected.
- Capture selected position on Begin; pending/active/modal states block target changes. Held/Finish/cancel/result copy remains bound to that captured instruction target. Selecting another key after completion must not rewrite previous result provenance.
- Preserve Enter/Space rejection for Start/Held/Finish, mouse activation, all-key release instructions, exact firmware/identity/fresh-baseline gate, safe waiting/cancel/close ownership and original failure/unlock details.
- Verify Begin/DB and Held/DD packet construction remains byte-for-byte unchanged with no index/mask/value. Selecting a diagram key sends no operation; no F0/E0/F1/E1/firmware/reset/unknown commands are introduced.
- The global shortcut locator must show captured normal-layerRightFn at position62, recognize moved/missing/duplicate/advanced Fn assignments, and require the existing exact live target/action149 evidence. Do not infer the global stage from UI clicks.
- Retain accepted Light/Dark/System styles, <=1-point row centers, zero default collapsed scroll range at960×820,66-key/minimum reachability and palette contrast. The new selector should sit below the step card without increasing the default window height.
- After successful calibration, guide the user to the existing InputTest tab and explicit fresh Read, rather than claim a threshold value or sensor result. InputTest remains an OS-event/output-candidate display.

No implementation was performed by this reviewer. The general physical calibration-guidance plan may proceed after these constraints are adopted; actual arbitrary-key numerical threshold editing remains a separate unresolved capability, not a completed feature.
