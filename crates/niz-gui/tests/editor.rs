use niz_core::{Definition, Profile};
use niz_gui::editor::{Editor, Mode, regenerate};
fn single(code: u8) -> Definition {
    Definition {
        kind: 0,
        keys: vec![code],
        interval: 0,
        cycles: 1,
        custom_delay: false,
        delays: vec![],
    }
}
#[test]
fn reassign_to_is_output_not_physical_position() {
    let mut e = Editor::new(Profile::demo());
    e.select_from(1);
    e.choose_output(43);
    e.selection.to_position = Some(2);
    e.edit().unwrap();
    assert_eq!(e.draft.definition(1).unwrap().keys, vec![43]);
    assert_eq!(e.draft.records[2], e.baseline.records[2]);
    assert!(e.dirty());
}
#[test]
fn swap_to_is_physical_position_and_moves_whole_raw_record() {
    let mut p = Profile::demo();
    p.records[1][0][63] = 91;
    let mut e = Editor::new(p);
    e.selection.mode = Mode::Swap;
    e.selection.from = Some(1);
    e.selection.to_position = Some(2);
    e.selection.to_code = 43;
    e.edit().unwrap();
    assert_eq!(
        e.draft.definition(1).unwrap().keys,
        e.baseline.definition(2).unwrap().keys
    );
    assert_eq!(e.draft.records[2][0][63], 91);
}
#[test]
fn layer_selection_changes_same_physical_key_only_on_that_layer() {
    for layer in 0..3 {
        let mut e = Editor::new(Profile::demo());
        e.selection.layer = layer;
        e.select_from(2);
        e.choose_output(43);
        e.edit().unwrap();
        assert_eq!(e.preview().unwrap(), vec![layer * 66 + 2]);
    }
}
#[test]
fn undo_redo_and_noop_history_are_byte_exact() {
    let mut e = Editor::new(Profile::demo());
    e.select_from(1);
    e.choose_output(15);
    e.edit().unwrap();
    assert!(!e.undo());
    e.choose_output(43);
    e.edit().unwrap();
    let edited = e.draft.reports();
    assert!(e.undo());
    assert!(!e.dirty());
    assert!(e.redo());
    assert_eq!(e.draft.reports(), edited);
    assert!(e.undo());
    e.choose_output(44);
    e.edit().unwrap();
    assert!(!e.redo());
}
#[test]
fn fn_swap_preview_is_all_layers_atomic_and_undoable() {
    let mut e = Editor::new(Profile::demo());
    e.selection.mode = Mode::Swap;
    e.selection.from = Some(54);
    e.selection.to_position = Some(2);
    e.edit().unwrap();
    assert_eq!(e.preview().unwrap(), vec![2, 54, 68, 120, 134, 186]);
    assert!(e.undo());
    assert_eq!(e.draft.reports(), e.baseline.reports());
}
#[test]
fn unknown_source_cannot_be_edited_and_history_remains() {
    let mut p = Profile::demo();
    p.records[1][0][6] = 178;
    let mut e = Editor::new(p);
    let before = e.draft.reports();
    e.selection.to_code = 43;
    assert!(e.edit().is_err());
    assert_eq!(e.draft.reports(), before);
    assert!(!e.undo());
}
#[test]
fn invalid_selection_and_wrong_import_never_change_draft() {
    let mut e = Editor::new(Profile::demo());
    e.selection.from = Some(66);
    assert!(e.edit().is_err());
    let mut other = Profile::demo();
    other.identity.product_id = 2;
    assert!(e.import(&other).is_err());
    assert!(!e.dirty());
}
#[test]
fn native_import_retains_opaque_and_is_one_undo_step() {
    let mut p = Profile::demo();
    p.records[10][0][6] = 178;
    let mut imported = p.clone();
    imported.set_definition(1, &single(43)).unwrap();
    let mut e = Editor::new(p);
    e.import(&imported).unwrap();
    assert_eq!(e.draft.records[10], e.baseline.records[10]);
    assert!(e.dirty());
    assert!(e.undo());
    assert!(!e.dirty());
}
#[test]
fn regeneration_uses_selection_without_mutating_source() {
    let p = Profile::demo();
    let s = niz_gui::editor::Selection {
        from: Some(1),
        to_code: 43,
        ..Default::default()
    };
    let draft = regenerate(&p, s).unwrap();
    assert_ne!(draft.records[1], p.records[1]);
    assert_eq!(draft.records[2], p.records[2]);
}

#[test]
fn fn_swap_with_unknown_alternate_record_is_atomic() {
    let mut p = Profile::demo();
    p.records[2 + 66][0][6] = 178;
    let original = p.records.clone();
    let mut e = Editor::new(p);
    e.selection.mode = Mode::Swap;
    e.selection.from = Some(54);
    e.selection.to_position = Some(2);
    assert!(e.edit().is_err());
    assert_eq!(e.draft.records, original);
    assert!(!e.undo());
}
#[test]
fn unknown_import_delta_leaves_existing_dirty_history_untouched() {
    let p = Profile::demo();
    let mut e = Editor::new(p.clone());
    e.select_from(1);
    e.choose_output(43);
    e.edit().unwrap();
    let draft = e.draft.records.clone();
    let mut imported = p;
    imported.records[10][0][6] = 178;
    assert!(e.import(&imported).is_err());
    assert_eq!(e.draft.records, draft);
    assert!(e.undo());
    assert!(!e.dirty());
}

#[test]
fn trigger_guide_requires_live_exact_target_and_existing_right_fn_7_action() {
    use niz_gui::editor::{TriggerGuide, trigger_guide};
    let mut p = Profile::demo();
    p.firmware = "66EC(XRGB)BLe;V1.0.48;V1.0;".into();
    p.identity = niz_core::DeviceIdentity {
        vendor_id: 0x0483,
        product_id: 0x502a,
        usage_page: 0x8c,
        usage: 1,
    };
    p.set_definition(66 + 7, &single(149)).unwrap();
    assert_eq!(
        trigger_guide(Some(&p), true),
        TriggerGuide::CapturedShortcut
    );
    assert_eq!(
        trigger_guide(Some(&p), false),
        TriggerGuide::UnverifiedTarget
    );
    p.set_definition(66 + 7, &single(43)).unwrap();
    assert_eq!(trigger_guide(Some(&p), true), TriggerGuide::CustomMapping);
    p.firmware = "Other firmware".into();
    assert_eq!(
        trigger_guide(Some(&p), true),
        TriggerGuide::UnverifiedTarget
    );
}
