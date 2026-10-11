# Device status row typography

2026-10-11 presentation-only fix: the status label had an explicit12-point font while the adjacent device-information disclosure used14-point text. The status now inherits Body; the disclosure explicitly uses Body; the adjacent DEMO explanation also uses Body. Current Body is14 logical points and follows theme/font scaling. Status predicates, colors, spinner, wrapping, device ownership and all transport operations are unchanged.

Existing rendered tests for mixed-row vertical centers, default collapsed-view overflow, and expanded device-information fonts each passed. GUI all-target Clippy with warnings denied and workspace formatting passed. No new test was needed for this small presentation fix. [Verification logs](device-status-font-logs/alignment.log) and [independent read-only review](device-status-font-logs/review.md) are retained.

Both release architectures were rebuilt; signatures, plist and arm64/x86_64 slices verified. The coordinator reopened the packaged actual app through CUA and read settings. A screenshot confirmed the loaded-status label and device disclosure share the same size and vertical center. The reported offline status uses that same unconditional text style; no hardware mutation was used to recreate it. A native accessibility tab click did not change the selected view, so no return to the threshold tab is claimed. The owned app was left open and no Simulator resources were used.

[Release hashes](release-sha256.json) and [changed source hash](device-status-font-source-sha256.json) record the checked build and source. This font fix does not implement persistent numerical threshold writing.
