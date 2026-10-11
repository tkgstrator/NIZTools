# Device P1 ownership fix

Verified the independent finding in `/private/tmp/niz-global-sensitivity-20261011/source-review.md` against the actual implementation. Before this fix, a short/error D8 write or a post-D8 F9 failure could return before the existing DC-specific settle path ran.

The reviewed device source is now frozen again, in local workspace `/Users/devonly/Developer/personal/NIZ` on `devonlys-MacBook-Pro.local`.

## Change

`src/sensitivity.rs:96` records the original failure before cleanup. Every failure after an attempted D8 invokes bounded owned passive capture before terminalizing the sensitivity status, unless the failing DC reader already completed the same settle. Status stays active while capture occurs, so its trace includes late bytes. All captured bytes remain in `Error.raw_reports`, and before/after observations, snapshot, attempted/sent flags and original failure remain intact. Cancellation arriving during cleanup is reflected in the final aborted status.

`src/session.rs:329` factors the existing passive settling loop into an internal helper shared by failed DC reads and failed attempted-D8 transactions. Its unchanged bounds are 2000ms, 50ms wait steps, 40 attempts, 16 captured reports; probe errors stop capture while bounded waits continue. It sends no output and keeps the same transport/per-path lock owned.

`src/lib.rs:25` adds a private `Error.passive_settle_complete` marker. The helper marks completion, so a failed post-DC path does not repeat its settle. D8-only paths use `D8 delivery or effects may have occurred` and `passive capture after failed D8 transaction`; they do not claim a DC delivery. Existing DC paths retain DC-specific uncertainty and scanner-restoration caution.

Public sensitivity API and fields are unchanged. Changed device files for this fix: `src/lib.rs`, `src/session.rs`, `src/sensitivity.rs`, `src/sensitivity_tests.rs`.

## Red/green evidence

Two strict transport tests were added before production fixes. Each failed at the expected ownership assertion: actual passive waits **0**, required **2000**.

- `uncertain_d8_delivery_holds_owned_passive_capture_before_return_without_extra_output` (`src/sensitivity_tests.rs:567`) covers native D8 error and short write. A literal late packet appears only during subsequent sleep; the test requires that packet in both the error and active-owned trace, complete before/snapshot retained, no after, and exactly the 8 expected output commands.
- `failed_post_f9_write_short_read_and_timeout_hold_owned_d8_settle_without_dc_dispatch` (`:605`) covers native post-F9 write error, short write, short read and timeout. A later packet appears during the passive hold. The test requires it in error/trace, the sent D8 flags, no second DC dispatch, and exactly the 9 expected output commands.

Both also assert no D8 acknowledgement read, no extra output, no native queue drain, no mistaken DC-delivery uncertainty. Existing malformed/partial/late/excess/short post-DC tests continue asserting exactly **2000ms**, demonstrating no double settle.

## Final verification

- `cargo test -p niz-device sensitivity_tests --lib`: **15 passed**, 0 failed.
- `cargo test -p niz-device`: **124 passed**, 0 failed; all raw-threshold/full/held calibration regressions passed; 0 doctests. Final run completed in 12.39 seconds.
- `cargo check -p niz-device`: success without warnings.
- `cargo fmt -p niz-device -- --check`: success.

No real hardware access, app/Simulator operation, Git command, other-crate edit or delegation occurred. Root must refresh the actual review diff and request re-review/full workspace verification before hardware qualification.
