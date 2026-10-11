# Atom66 XRGB BLE physical layout evidence and independent plan review

Verified host/workspace: `devonlys-MacBook-Pro.local`, `/Users/devonly/Developer/personal/NIZ`. Read-only analysis; no HID commands, firmware execution, Windows execution, Git mutations or source edits.

## Primary sources

- Supplied `User Manual/66EC(XRGB)Ble User Manual.doc`, saved May 2018, contains an official main-program screenshot (`extracted-manual-2.png`) and lighting-settings screenshot (`extracted-manual-0.png`). Extracted by reconstructing the OLE Compound File Data stream via FAT, then parsing PNG chunks. Raw byte scanning the DOC fails because sector boundaries are interleaved.
- Supplied `Software/66EC(XRGB)BleRES.dll` in original Software ZIP (2022-06-23 timestamp). Exact resource-directory inspection identifies bitmap RT_BITMAP/101, 1050×701, 24-bit. Extracted only that image, `RES-101.png`. It independently agrees with the manual's main keyboard geometry and legends. 52 bitmaps exist; no complete DLL decompile was attempted.
- EXE dialog resources inspected; these are small containers with no66individualbuttoncontrols, so they do not independently establish protocol slot IDs. The bitmap contains no slot numbers.

## Proven physical shape and legends

66 buttons in5rows with counts15,14,13,13,11. The x-coordinate spans total15standardunits per row.

| Row | Physical legends left to right | Approximate width units |
| --- | --- | --- |
| 1 | Esc,1,2,3,4,5,6,7,8,9,0,-,=,grave,Backspace | 15×1 |
| 2 | Tab,Q,W,E,R,T,Y,U,I,O,P,[,],backslash | 1.5,12×1,1.5 |
| 3 | Caps,A,S,D,F,G,H,J,K,L,semicolon,quote,Enter | 1.75,11×1,2.25 |
| 4 | LeftShift,Z,X,C,V,B,N,M,comma,period,slash,RightShift/Up,Delete | 2.25,10×1,1.75,1 |
| 5 | Ctrl,LeftFn,Win,Alt,Space,RightFn,Alt,Ctrl,Left,Down,Right | approx1.5,1,1,1,4.5,1,1,1,1,1,1 |

Widths are inferred from rendered bitmap boundaries to normal keyboard units, not a manufacturer numeric table. In particular bottom Ctrl is visibly ~1.5u and Space ~4.5u; existing niz-web widths1.25×4+4u do not match this bitmap.

If protocol records were entirely in row-major physical order, row spans would be1–15,16–29,30–42,43–55,56–66. That implies A31 and main Enter42, matching existing stable-slot usage. However this is NOT enough evidence to assert every protocol physical identity:

- Physical bottomrow shows RightFn before rightAlt, which row-major mapping would label61Fn and62Alt.
- Current live captured profile instead assigns61rightAlt and62rightFn. Key assignments are mutable and do not prove physical legends or order. It could be custom remapping or an internal record-order mismatch. Static bitmap has no control/record IDs, and bounded inspection found no independent ID table.
- Therefore keep protocol61/62 labels/currentassignments separate from fixed physicallegends, and do not claim a photograph proves slot61/62 mapping. A conservativeUI can show current assigned labels and stable slotnumber in this bottomrow, without asserting fixedFn/Altlegend for ambiguouspositions. Resolving this requires either exact vendor hitrect→slot staticcode proof or actual user physical-key mapping evidence.

## Independent feasibility review

The plan to use immutable protocol slot IDs, separate displayed assignments from physical legends, selected/dirty styling, click-to-selectFrom, and explicit From/To swap choice is feasible within the existing egui renderer/editor. It avoids deriving positions from current key values (duplicates/unbound/opaque values remain selectable).

Material findings:

1. P1: Do not use current output codes or old public KEY_ROWS strings as the authority for physical key identities. Existing rows incorrectly place grave/backslash/Backspace and Delete/Fn. Primary images prove corrected geometry/legends but not all numeric record IDs. For ambiguous61/62, show stable numbers/current assignments and describe uncertainty rather than silently assuming.
2. P2: The old bottom width array is visually inaccurate for this target. Use a fixed total15u shape matching primary bitmap; test each row total and count.
3. P2: Swap needs explicit target selection while the default remains one-way reassignment. Clicking arbitrary key while a popup is open must not silently switch intent or commit a draft. Clicking merely selects; applying existing guarded profile action remains explicit.
4. P2: Disable layout interaction during ownedworkeroperations/calibration and detached/noeditor states. Retain selectedslot through assignmentchanges and device list reorder; require attacheddevicepath eligibility for application.
5. P2: Dirty styling must compare full baseline slot definition/raw record, including macros/continuations. A displayedkeycode-only comparison would miss changes. Existing editor.differences/dirty mechanism should be reused.

Recommended meaningful tests: exactly66uniqueIDs in bounds; rowcounts/totals; knownslotA31; selection identity unchanged after remap/duplicates; all3layers select same physicalslot; assign/swap selection effect and same-key behavior; layoutinteractiondenied whileworkerowns session or calibrationactive; disablednoeditor/detachedapply; changedkey styling andUndo restore; unknown/macro outputlabeldoesnotpanic; resize at minimumwindowwidth prevents horizontalclipping; popup interaction cannot leak click onto layout.

No implementation changes were made. Parent can use the extracted PNG files for visual inspection; proprietary images should stay in tasktemp and need not ship in public repo.

## Hashes
- `/private/tmp/niz-analysis-20261010/User Manual/66EC(XRGB)Ble User Manual.doc` SHA256 `e07495d4a738e1b26d6b5fb6e4194b39594d3cc17d2b946875e1d6ae4fbffbea`
- `/private/tmp/niz-implementation-20261011-layout/RES.dll` SHA256 `b50d437ca7f1a491702e91947ab8892cfbfb66f12a70a2fdcdd18376e03a9393`
- `/private/tmp/niz-analysis-20261010/Software/66EC(XRGB)Ble.exe` SHA256 `3189ce0a95911b7cbeffd6a15092f38af490aa8bf2d90115453d11f401974555`

## Bounded static trace addendum: UI positions to vendor view IDs

A targeted EXE disassembly (private temp `exe-disassembly.txt`, not for repository publication) finds the66-key lighting-view initializer. Its keyviewrecords are84bytes (`0x54`) apart. Rect starts object+`0x22c` for viewID1, with the1-based viewID at rect+`0x18`. This is independent of current assignment values.

| View ID | ID storage instruction VA | Rect storage base | Rect pixels | Physical bitmap legend |
| --- | --- | --- | --- | --- |
| 1 | `0x402c4f` stores1 atobject+`0x244` | `0x22c` |49,120 →96,169 | Esc |
| 31 | `0x4061bf` stores31 atobject+`0xc1c` | `0xc04` |145,226 →192,274 (`0x406231`–`0x406257`) | A |
| 57 | `0x407e96` stores57 atobject+`0x14a4` | `0x148c` | bottomleftFn rectangle (same84bytestride) | LeftFn |
| 61 | `0x4082ea` stores61 atobject+`0x15f4` | `0x15dc` |509,330 →557,380 (`0x40832f`–`0x408355`) | RightFn |
| 62 | `0x4083ff` stores62 atobject+`0x1648` | `0x1630` |561,330 →609,380 (`0x408444`–`0x40846a`) | RightAlt |

Lightingview click handler `0x4092f0` loops keyviewrecords at stride84, calls Windows rectangle test via import `0x587764`, retrieves the record's viewID at`0x409367`, and writes a selected flag to a global array indexed by that ID at`0x409380`. This grounds the vendorviewID↔physicalrect relation. Thus viewID61isphysRightFn andviewID62isphysRightAlt in this supplied official UI.

**Remaining evidence boundary:** The exact mapping from these lightingviewIDs to F2 configurationrecord addresses was not traced to completion within the bounded request. These IDs might match protocolslots, or lighting and configuration matrices may differ. Captured outputs alone cannot resolve that. Therefore this addendum strengthens fixedgeometry/numberedviewIDs but does not claim protocol61/62correspondence proven. Use immutableprotocolID/currentassignment labels if implementing now; avoid asserting a staticphysicallegend certifies eachslotwithout documenting this boundary. Further statictrace would need mainUI click/index handler through `setSingleKey` or configuration serializer, rather than merely more bitmap/image evidence.

## Main-view and F2 bridge follow-up

This supersedes the earlier statement that only lightingview IDs had been located. The actual66-key mainview has been found and decoded. `main-key-records.json` contains all66pairs of immutableviewID andpixelrect; everyID1..66appears exactlyonce in row-major order, and all66rectanglecoordinatewrites were recovered from immediate loads in the mainviewinitializer. It is not inferred from mutableassignments.

Mainviewkeyrecords follow84bytestride; first keyrectisobject+`0x5cc`, and ID is rect+`0x18`. The mainwindow clickhandler loads thatfield asobject+`0xf8+84*record_index` at`0x4467f8`, and stores its unchangedID as selectedkey atglobal`0x5ec224` at`0x446807`. There is no61/62swap in that handler.

| Mainview ID | ID instruction VA | Rect pixels | Printed legend |
| --- | --- | --- | --- |
| 1 |0x43e11b|197,182 →248,235|Esc|
| 14 |0x43f03a|863,182 →914,235|grave|
| 15 |0x43f14f|914,182 →965,235|Backspace|
| 29 |0x44007c|890,235 →966,288|backslash|
| 31 |0x4402a6|294,288 →345,341|A|
| 55 |0x441c9e|913,341 →964,394|Delete|
| 57 |0x441ec8|268,394 →320,447|LeftFn|
| 61 |0x44231c|657,394 →709,447|RightFn|
| 62 |0x442431|709,394 →761,447|RightAlt|
| 66 |0x442885|914,394 →965,447|RightArrow|

Independent F2 retrieval chain in the same supplied app:

1. EXE imports `getKeyConfiguration` atIAT`0x587018`; mainreadfunction starts`0x44f610`, callsnativeF2readviaIAT`0x58701c` at`0x44f65a`, then loops groupEDI0..2 andkeyESI1..66 (`0x44f660`, `0x44f670`, endtests`0x45013b` and`0x45014b`).
2. At`0x44f694` it passes groupEDI andkeyESI directly to`getKeyConfiguration`, with no mappingtableor61/62conditional. For basicassignment, it stores returnedcode at objectoffset `4*(3*(key+0x34e)+group)` (`0x44fd9a`–`0x44fdab`), equivalentto `0x27a8+12*key+4*group`.
3. DLL getter`0x10002160` computesflatentry`66*zero_based_group+one_based_key-1` at`0x10002163`–`0x10002179`; no key permutation.
4. DLL F2decoder`0x10001584` reads rawlogicalpayloadbyte2 asone_based_group andbyte3 asone_based_key. Instructions`0x10001592`–`0x100015a3` compute`66*raw_group+raw_key-67`, identicalto getterindex aftergroupconversion. It storesparseddefinitioninto thatexactentryat`0x1000162e`.
5. Mainpopup/UIusesselectedID`0x5ec224` andlayer`0x5ec228` ininterleavedmatrixindex`3*ID+layer`; e.g.`0x4290c9`–`0x4290d9` and`0x44505b`–`0x44506b`. The configurationcopyhelperwritesmatrixentries linearly (`0x44afe6`–`0x44affe`, increments4andthreegroupsin`0x44b00a`–`0x44b021`) withoutswaplookup. WriteAPIcalls`setKeyConfiguration` throughIAT`0x58702c` at`0x44f29f`, with groupEBX0..2 andkeyEDI1..66 (`0x44f29d`–`0x44f29f`, loopbounds`0x44f2cd` and`0x44f2ec`).

These establish the supplied hostsoftware's intended row-major mainview↔keyindex correspondence and show no61/62permutation in the read/get/click/write indexpaths. The mainviewID31thereforecorrespondstoF2key31A; itsID57/61/62rectanglesareLeftFn/RightFn/RightAlt.

**Practical boundary:** This is staticproof of the supplied hostsoftware’s mapping, not a physicalkeyscan test of the attached firmwareV1.0.48. The currentcaptureassignsRightAlt toindex61andRightFn toindex62, opposite theprintedlegends. A customremap is consistentwiththeevidence; firmwareinternalphysicaldifference is not ruledout withoutactualphysicalpress/protocolconfirmation. The GUI can ground geometry/immutableIDs in officialstaticsoftware and display actualcurrentassignments separately. It must not force factorydefaultsorrenamecapturedassignmentvalues to match printedlegends. No hardware qualificationclaim follows from this analysis.

The copy/displayserializationpaths are complex and were inspected only for linear index-preserving loops; no executable run or test of Windowssoftware was performed. Keep suppliedbitmapandfulldisassemblyprivatein tasktemp.
