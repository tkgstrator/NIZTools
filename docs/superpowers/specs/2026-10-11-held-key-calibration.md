# Official held-key calibration

The user accepts the stock firmware's official held-key calibration, including the possibility that other keys change, provided all 66 normal scan thresholds are compared before and after. The selected physical key is a hold instruction, not a firmware write mask. Stock firmware remains unchanged.

The UI keeps the physical keyboard below the controls. Select a key, prepare with all keys released, then hold only that physical key and click a red Execute button with the mouse. Cancel sits beside Execute. Selection stays frozen while the operation owns the device. Execution automatically completes verification; it never repeats DD.

Use retained release calibration. Do not send DB, D5, D8, configuration/RGB writes or firmware commands. Read qualified DC mode 1 thresholds before D9/DD and again after DE while the same D9 output lock remains held. Then unlock once and verify unchanged keymap/RGB. DC mode 1 preserves the previous scanner enable state; keeping it locked prevents ordinary key/mode input during comparison.

Report all 66 physical positions and literal before/after values. Distinguish no observed change, target-only change and changes in other keys. Missing/partial post-read is comparison unavailable; it is never proof of unchanged values. Complete observations survive a later settings failure. Cancellation/failure retains dispatch/DE state, traces and cleanup outcome. Backup stores keymap/RGB, not restorable analog calibration.

Normal threshold readback does not establish stored analog table equality, persistence, all scan modes or improved physical response. Numeric threshold sliders and all-key reset remain outside this implementation. DEMO values are explicitly simulation. Preserve light/dark themes, centered rows/disclosure glyph, stable hover, and default window fit.
