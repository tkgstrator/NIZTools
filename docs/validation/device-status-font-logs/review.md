# Status row font consistency review

Read-only review of current app.rs status row and theme.rs text style sizes in `/Users/devonly/Developer/personal/NIZ`. Native source access verified; no product edits, app/HID/Git operation, or test rerun.

No material findings. app.rs:886 uses ordinary colored_label, inheriting the Body style rather than a hardcoded12pt size. Adjacent DEMO sentence at line888 is ordinary label, and the device-info disclosure title at line891 explicitly selects Body. theme.rs:159-160 sets Body and Button to14pt, so all three adjacent texts now use the same style/size and continue to follow the app's font scaling. Status selection logic, colors, spinner, wrapping, disclosure contents and behavior are unchanged. Existing status alignment/layout/expanded-font tests are sufficient for this bounded presentation change; parent is running them.
