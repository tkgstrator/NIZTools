# GUI global sensitivity implementation

Host/workspace: local `/Users/devonly/Developer/personal/NIZ` (native Codex filesystem/commands). Delegated scope: GUI protocol/controller/worker/lib, standalone sensitivity UI, external GUI tests. App/app_tests/input_test were not edited by this worker. Existing calibration message changes preserved.

## Interfaces and behavior

- `Command::AdjustSensitivity { backup, coefficient }`, `Response::Sensitivity(SensitivityStatus)`, `Failure.sensitivity`; device errors retain their sensitivity context in conversion.
- `Controller::adjust_sensitivity(backup, coefficient, Activation)` validates primary-pointer activation, 10..40 range, exact firmware/identity, current concrete target, 3 groups, live baseline and exclusivity. Dispatch consumes live eligibility, clears old calibration/raw/sensitivity observations, and resets only the shared cancellation flag before dispatch.
- Sensitivity status retained on success or error. Success replies must match coefficient and provide Complete phase, sent/verified settings/lighting and both complete 66-value vectors; malformed replies become Failure with observations retained.
- Read, switch, and changed/disconnected enumeration source invalidate sensitivity. Trial/new calibration/new adjustment clear prior analog attribution. Close during adjustment sets the existing flag and waits for the owned response; no speculative cleanup or second command is queued.
- Worker independently validates concrete target/generation/replay/exact supported baseline/range/calibration ownership and invalidates eligibility before entering the backend. NativeBackend takes its baseline, invokes the exact Session API, then drops the session. DemoBackend generates explicit complete examples and cancellation observations; it never enumerates native HID.
- `sensitivity_ui::show` exact requested signature, module registered in lib. Default-collapsed compact panel, inverse slider (`c40` left, `c10` right), primary-pointer-only red Apply, popup/modal/pending/calibration/close guards. Summary allows unchanged thresholds and visibly labels DEMO. Complete comparison details show all 66 physical positions, before→after or missing markers, source, firmware, requested coefficient, verification, errors, backup.

## TDD and verification evidence

- `gui-red.log`: first feature tests fail on missing SensitivityStatus / command / response / controller methods before production implementation.
- `gui-render-red.log`: render test module compiled before show existed, failed on missing show.
- `gui-validation-red.log`: boundary regression test fails because mismatched success coefficient was accepted; production validation added afterward.
- `gui-full.log`: `cargo test -p niz-gui` passed 156 tests across all test targets: lib69/controller27/editor12/held3/keyboard17/raw-controller6/raw-worker4/sensitivity9/worker9, plus empty main/doc targets.
- `gui-focused.log`: all 4 render tests passed with actual accessibility widget ID focus (Enter/Space), actual AccessKit Click rejection, actual secondary/primary pointer behavior, popup guard, inverse slider endpoints, default collapse, unchanged result and complete66 details.
- `gui-final-focused.log`: latest post-format/DEMO-summary `cargo test -p niz-gui --lib --test sensitivity` run; completion checked separately before handoff.

No actual HID operation, native GUI launch, Simulator, Git operation, or extra subagent was used. LocalGPT and Claude generation tools were unavailable; parent authorized native Codex implementation fallback. Native build/device transport qualification and independent integration review remain owned by root.
