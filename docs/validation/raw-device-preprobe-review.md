# Exact-target raw threshold API and CLI: pre-probe source review

2026-10-11. Independent read-only review on `devonlys-MacBook-Pro.local`, workspace `/Users/devonly/Developer/personal/NIZ`. No HID, live-device enumeration/open, application UI, Git, vendor/firmware execution, product dependency installation or repository file changes. Only this temporary review report was written. Source was read directly; the eight Core/Device files all matched the implementation worker manifest `/private/tmp/niz-raw-20261011/device-source-sha256.json`. CLI source SHA-256 at review: `c03bdd1078b3e028f08df702d9148de4b994d8179d924810bcd37f5cc99ca088`.

## Disposition

**Request changes before the native qualification probe.** The command allowlist, exact identity/version gate, strict decoder and full physical map are correct. Three P2 findings concern excess-input rejection, retained ownership during a failed in-flight getter, and the proven runtime compensation caveat. There is no demonstrated threshold setter or persistent-writing command in this read path.

## Findings

### P2: fresh post-F9 input is silently drained before DC

References: `crates/niz-device/src/session.rs:249`, `crates/niz-device/src/session.rs:174`, `crates/niz-device/src/transport.rs:64`, `crates/niz-device/src/raw_threshold_tests.rs:27`, `crates/niz-device/src/raw_threshold_tests.rs:147`.

`raw_thresholds_inner` receives and qualifies a fresh F9, then calls `prepare_transaction` a second time. Outside calibration, this invokes Native::prepare, which reads/discards every queued report within its bounded drain. A duplicated F9, unsolicited stream or unexpected DC/DF report arriving after the fresh F9 is therefore discarded, and the DC mode-1 request still goes out. The prior qualification plan treats unexpected traffic/contending clients as a stop condition. Existing post-stream queued-input rejection cannot detect these discarded pre-DC bytes.

Keep the initial bounded stale drain before F9. Replace the second prepare with a strict nonblocking queued-input check that retains unexpected bytes and returns a poisoned transaction error before DC. Add a fixture with an extra report introduced after F9, proving F9 is the only output and preserving the extra byte sequence. Initial stale-drain logging remains a diagnostic limitation of the existing transport, which does not expose discarded bytes.

### P2: a failed DC transaction releases ownership without passive settling

References: `crates/niz-device/src/session.rs:255`, `crates/niz-device/src/session.rs:258`, `crates/niz-device/src/session.rs:278`, `crates/niz-cli/src/main.rs:303`, `crates/niz-device/src/transport.rs:57`, `crates/niz-device/src/raw_threshold_tests.rs:75`, `crates/niz-device/src/raw_threshold_tests.rs:233`, `crates/niz-device/src/raw_threshold_tests.rs:292`.

After a DC write, a malformed first row, short read, I/O error, short output or uncertain output delivery returns directly through the error path. The Session is poisoned, correctly preventing another request on that same object, but there is no bounded passive read/settling phase. The CLI propagates the error with `?`, immediately drops the native Session/handle and releases its per-path lock. The firmware can still be inside the eleven-send DC routine and has not necessarily reached scan-enable/compensation restoration. A newly opened session can then acquire the lock and issue a new command while the previous getter remains in flight.

The exact firmware audit establishes that host parser failure does not cancel the routine. Each USB send may make 30000 delay calls; eleven sends may make 330000. Nominal programmed waiting is roughly 0.33 seconds plus overhead, and SysTick has no secondary timeout. Thus malformed early responses are materially different from a late timeout; immediate handle drop cannot be represented as cancellation or proven restoration.

Once DC dispatch has been attempted, keep the owned handle/path lock through a bounded, output-free passive capture/settling phase whenever the connection remains usable. Preserve the original error, append useful passive bytes, then release ownership and explicitly report any restoration uncertainty after the cleanup budget. Do not send D9, other recovery commands or retries. A disconnected transport must return without invented writes, with restoration uncertainty retained. Add a fixture/gate proving that ownership remains held during the passive phase and that no output is added on malformed/short/uncertain-delivery paths. The current Fake::sleep_ms panic explicitly forbids settling waits and must be adjusted if the implementation uses a bounded sleep/read strategy. No fixed host cleanup budget can prove restoration after a broken SysTick; use accurate wording.

### P2: CLI/API documentation omits the compensation restoration caveat

References: `crates/niz-cli/src/main.rs:308`, `crates/niz-cli/src/main.rs:320`, `crates/niz-device/src/session.rs:218`, `crates/niz-device/src/raw_thresholds.rs:14`.

CLI JSON/text accurately call this a raw normal-scan R snapshot and state that no persistent settings were written. They do not expose the statically proven runtime caveat. API docs say the getter temporarily affects scanner output state, which is incomplete when the prior active compensation differs from the configured/default byte.

Exact getter instructions `0x6728` and `0x67b0..0x67b4` clear active compensation at RAM `0x200009b0`, then copy configured/default `0x200009b1` into it. The previous active byte is never saved. B/T/R/global-mode/persistent fields remain unchanged, but effective signal compensation may differ after the read. Include this caveat in CLI text and structured JSON, and state it precisely in Session documentation. Also retain the existing normal-scanner scope: alternate scanner state 4 uses fixed press/release thresholds 6/4, so R is not guaranteed to be active in every mode. This is a proved runtime behavior, not a hypothetical calibration warning.

## Verified controls and evidence

- `supports_raw_thresholds` (`crates/niz-device/src/raw_thresholds.rs:6`) restricts identity to VID0483/PID502a, usage008c/1, and exact `66EC(XRGB)BLe;V1.0.48;V1.0;`.
- `Session::read_raw_thresholds` rejects busy calibration/poisoned sessions and wrong identity before output. `version_capture_inner` performs one fresh F9 using the existing 65-byte output/64-byte input conventions. A fresh wrong version or retained baseline metadata mismatch stops before DC (`session.rs:237..247`).
- The reachable raw getter emits exactly zero-padded F9 then zero-padded `00 DC 01` (`session.rs:253..255`). No D9, DB, DD, D5, D8, F0/configuration write, flash/reset command or other DC mode is constructed. CLI arguments cannot select another mode or a setter.
- Native open retains the existing per-path SHA-256 file lock (`transport.rs:102..104`, `:264..291`), compares opened versus enumerated identity (`:117..124`), and validates unnumbered 512-bit input/output descriptor semantics (`:125..129`, `:140..262`). The raw read stays on that same handle. A cooperative lock does not establish exclusion from unrelated clients that ignore it.
- Core parser requires exactly eleven ascending rows 1..11, prefix00DC and mode1; an error poisons that decoder instance. Missing, duplicate, reordered, out-of-range, excess, wrong prefix/mode and ignored parser-error repair are rejected. No DF/E6 terminator is invented. Bytes10..63 remain opaque and are preserved rather than used as a padding predicate.
- Every one of the 66 `PHYSICAL_TO_MATRIX` entries was independently compared by a local Python read-only check to the inverse of the bytes at exact image address0x9dc6. All match. A/physical31 -> flat13; Enter/physical42 -> flat64; Right/physical66 -> flat59. The check asserted the exact firmware SHA before using its bytes.
- Full output length65 and input length64 are checked. The 30-second monotonic transaction deadline includes the F9 stage; individual reads use at most1000ms. Errors preserve the F9 plus previously received/failed/extra reports, poison Session, clear retained baseline and increment generation. No automatic retry occurs. `began_write=false` describes the absence of a settings setter, not absence of HID read-request writes.
- Success exposes immutable decoded physical-slot values and eleven original rows. Offline `from_reports` pins metadata and packet shape while documenting that it does not authenticate live device origin. Values are raw u8 firmware units without mm/travel/force conversion or ADC-stream claims.
- Existing source tests cover full outgoing bytes, every exact identity/version rejection, stale baseline mismatch, malformed/truncated/oversize/missing/I/O-failed replies, preparation/short/unknown output errors, deadline, poison/no retry, final queued excess, all66-map cells, opaque tails and all256 possible R bytes.

## Validation limits

I inspected saved `core-device-test.log`, which records 31+16+6+94 =147 passing tests with zero failures. The corresponding saved Clippy log ends successfully and the fmt check log is empty. All eight source hashes still matched that worker's frozen checkpoint when checked. I did not rerun package tests, inspect an actual hardware result, execute the CLI or claim the parent's later CLI/GUI checks passed. The review findings are not covered by the existing passing tests; new targeted cases are needed after correction. Revised files require a focused rereview before the native probe.

Firmware source: `/private/tmp/niz-ui-reference-20261011/target-v1.0.48-private-image.bin`, SHA-256 `b82e11891dfdc138780a81ac635697a7edb0ca1d4e123d6ad38c81a7ba55bf17`. Full static getter/runtime analysis: `/private/tmp/niz-perkey-20261011/firmware/raw-read-feasibility-review.md`.
