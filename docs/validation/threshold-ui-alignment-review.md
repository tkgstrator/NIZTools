# Independent threshold disclosure alignment review

Host verified: `devonlys-MacBook-Pro.local`. Workspace: `/Users/devonly/Developer/personal/NIZ`. Date: 2026-10-11. Independent Codex fallback review authorized by coordinator; preferred LocalGPT/Claude MCP capabilities were unavailable. No LocalGPT or Claude review is claimed.

## Verdict

**Clean: no unresolved P1, P2, or P3 findings in the actual bounded diff.** The custom painter aligns the disclosure arrow with the visible Japanese title glyphs while preserving the existing disclosure interaction and layout. No source fix is requested.

The current source matches `alignment.diff` and the saved after snapshots. Before backups match the prior approved copy manifest. All 17 GUI source/test hashes match the new freeze before and after independent tests. Only `app.rs` and `app_tests.rs` changed; the other 15 remain identical to the prior independently reviewed copy state. Evidence: `independent-source-sha256.json`.

## Source checks

- At `crates/niz-gui/src/app.rs:388-400`, the production delta adds an icon callback to this one `しきい値について` header. It uses the unchanged title string and Button text style, lays out the glyphs, matches the pixel-snapped text origin, computes the visible mesh center, translates only the response copy passed to `paint_default_icon`, and keeps the default openness value.
- Pinned egui 0.36.2 `src/containers/collapsing_header.rs:512-539` obtains the header identity from the same title salt, lays out the title with `TextWrapMode::Extend` and `TextStyle::Button`, allocates the original header space, and centers the logical line box. The application callback uses the same text/style/wrap mode. Infinity width is compatible here because Extend does not wrap this short title.
- The same pinned file at `:584-598` constructs a centered icon rectangle from the original header response, calls the custom icon painter, then paints the original title. `Response::with_new_rect` at `src/response.rs:1099-1100` changes only the returned copy's `rect`; it does not re-register interaction or allocate space. The callback does not call `interact`, allocate a widget, or modify the title response. Header hit area, ID, focus/action routing, disclosure state, and reader auto-ID therefore remain unchanged.
- Pinned epaint 0.36.2 `src/tessellator.rs:2022-2028` snaps the galley origin to physical pixels; text rounding is enabled by default at `:734`. Matching that origin plus `galley.mesh_bounds.center()` accounts for the Japanese fallback font's visible-glyph offset. This avoids a fixed point/pixel tweak tied to one scale.
- Default egui triangle painting at `src/containers/collapsing_header.rs:336-355` retains the existing color, hover state, size, and openness rotation. Only its center is moved vertically. Other disclosures, theme/fonts, cards, keyboard geometry, default layout allocation, read warning/provenance, and device-operation code are unchanged.
- `app_tests.rs:1239-1321` is a meaningful regression against the actual rendered title Text shape and the actual nearby filled three-point triangle Path. It tessellates the title and compares bounds in physical pixels, checks clipping, and opens/closes the real header. It also verifies detail visibility, no pending command/snapshot, and editor cleanliness. It is independent of the production callback's alignment arithmetic.

## Fresh independent verification

Ran `cargo test -p niz-gui --lib raw_threshold_ -- --nocapture` against frozen source: **exit 0, 11 passed, 0 failed**, 41 filtered, 1.57 s. Log: `independent-raw-tests.log`.

The new alignment regression reports **0.00 physical pixels in all 12 cases**: Light/Dark × scales 1/2 × closed/open/closed again. Existing affected tests also pass: optional detail toggling, explicit manual snapshot, focused Enter/Space, actual egui AccessKit action routing, disabled/source/modal/popup/close refusals, pending/error cache behavior, no automatic refresh, default/minimum all-key reachability, and stationary read-button hover.

The owner's `alignment-red.log` shows all 12 cases failing before production changes at 1.00 physical pixel for scale1 and 1.50 pixels for scale2. That retained fail-first evidence was inspected. `gui-all-targets-final.log` was also inspected: 127 GUI tests pass (52 library, 27 controller, 12 editor, 17 keyboard, 6 raw-controller, 4 raw-worker, 9 worker), including all 22 default collapsed states, device-information Body font consistency, and long-path layout. This is owner-run evidence, not an independent full-suite rerun. The expensive full suite was not repeated for this painter-only change.

The source hash record confirms 17/17 source matches after the independent run, the exact saved diff, prior-backup identity, and the 12 measured deltas.

## Limits

No source edits, native GUI, screenshots, HID, firmware writes, Git, executable/distribution rebuild, or Simulator operations were performed. Headless mesh/path evidence verifies geometric centers rather than native macOS raster appearance. The coordinator owns the normal rebuild and final native visual verification. Settled scale1/2 states are directly tested; intermediate animation and other scales retain the default painter but are not separately quantified by this regression. No claim is made that per-key threshold writing exists or that firmware behavior was newly validated.
