use niz_core::{Definition, Profile};
use niz_gui::{
    editor::{Editor, Mode},
    keyboard::{ClickTarget, KeyboardState, key_view, row_geometry},
    worker::{Backend, DemoBackend},
};

fn captured() -> Profile {
    Profile::from_json(include_str!(
        "../../../docs/validation/fixtures/atom66-xrgb-ble-v1.0.48-settings.json"
    ))
    .unwrap()
}
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
fn loaded_assignment_starts_unselected_and_same_key_click_clears_selection() {
    let mut e = Editor::new(captured());
    let original = e.draft.reports();
    let mut board = KeyboardState::default();
    assert_eq!(e.selection.from, None);
    assert_eq!(e.selection.to_position, None);
    assert!(e.proposed().is_err());
    assert!(board.click(&mut e, 30, false, false, false));
    assert_eq!(e.selection.from, Some(30));
    assert!(board.click(&mut e, 30, false, false, false));
    assert_eq!(e.selection.from, None);
    assert!(e.proposed().is_err());
    assert_eq!(e.draft.reports(), original);
}

#[test]
fn swap_clicking_highlighted_from_then_to_clears_each_operand() {
    let mut e = Editor::new(captured());
    let mut board = KeyboardState::default();
    board.set_mode(&mut e, Mode::Swap);
    assert!(board.click(&mut e, 30, false, false, false));
    assert_eq!(board.target, ClickTarget::To);
    assert!(board.set_target(ClickTarget::From, false, false, false));
    assert!(board.click(&mut e, 30, false, false, false));
    assert_eq!(e.selection.from, None);
    assert_eq!(board.target, ClickTarget::From);
    assert!(board.click(&mut e, 30, false, false, false));
    assert_eq!(board.target, ClickTarget::To);
    assert!(board.click(&mut e, 31, false, false, false));
    assert!(board.click(&mut e, 31, false, false, false));
    assert_eq!(e.selection.to_position, None);
    assert!(e.proposed().is_err());
}

#[test]
fn selecting_a_loads_its_setting_without_editing_or_creating_history() {
    let mut e = Editor::new(captured());
    let before = e.draft.reports();
    let mut board = KeyboardState::default();
    assert!(board.click(&mut e, 30, false, false, false));
    assert_eq!((e.selection.from, e.selection.to_code), (Some(30), 43));
    assert_eq!(e.proposed().unwrap().reports(), before);
    assert_eq!(e.draft.reports(), before);
    assert!(!e.dirty());
    assert!(!e.can_undo());
    assert!(!e.can_redo());
    e.choose_output(44);
    assert!(board.click(&mut e, 30, false, false, false));
    assert_eq!(e.selection.from, None);
    assert!(!e.output_ready());
    assert!(e.proposed().is_err());
    assert_eq!(e.draft.reports(), before);
}
#[test]
fn from_dropdown_uses_the_same_navigation_and_only_draft_values() {
    let mut e = Editor::new(captured());
    e.select_from(30);
    e.choose_output(44);
    e.edit().unwrap();
    e.select_from(31);
    e.select_from(30);
    assert_eq!(e.selection.to_code, 44);
    assert_eq!(e.preview().unwrap(), vec![30]);
    assert!(e.undo());
    assert!(!e.dirty());
    assert!(!e.undo(), "navigation must not add an undo entry");
}
#[test]
fn layer_navigation_preserves_slots_and_initializes_current_layer_output() {
    let mut e = Editor::new(captured());
    e.select_from(7);
    e.selection.to_position = Some(61);
    assert_eq!(e.selection.to_code, 21);
    e.select_layer(1);
    assert_eq!(
        (e.selection.from, e.selection.to_position),
        (Some(7), Some(61))
    );
    assert!(
        !e.output_ready(),
        "trigger action is preserved until explicitly replaced"
    );
    assert!(e.proposed().is_err());
    e.select_layer(2);
    assert_eq!(e.selection.to_code, 8);
    assert!(e.output_ready());
    assert!(!e.dirty());
    assert!(!e.can_undo());
    assert!(!e.select_layer(3));
    assert_eq!(e.selection.layer, 2);
}
#[test]
fn swap_target_transitions_are_explicit_and_never_swap_on_a_click() {
    let mut e = Editor::new(captured());
    let before = e.draft.reports();
    let mut board = KeyboardState::default();
    board.set_mode(&mut e, Mode::Swap);
    assert_eq!(board.target, ClickTarget::From);
    board.click(&mut e, 30, false, false, false);
    assert_eq!(board.target, ClickTarget::To);
    board.click(&mut e, 31, false, false, false);
    assert_eq!(
        (e.selection.from, e.selection.to_position),
        (Some(30), Some(31))
    );
    board.click(&mut e, 32, false, false, false);
    assert_eq!(
        (e.selection.from, e.selection.to_position),
        (Some(30), Some(32))
    );
    assert_eq!(board.target, ClickTarget::To);
    e.select_layer(2);
    assert_eq!(
        (e.selection.from, e.selection.to_position),
        (Some(30), Some(32))
    );
    assert_eq!(e.draft.reports(), before);
    assert!(!e.can_undo());
    board.set_target(ClickTarget::From, false, false, false);
    board.click(&mut e, 29, false, false, false);
    assert_eq!(
        (e.selection.from, e.selection.to_position),
        (Some(29), Some(32))
    );
    board.set_mode(&mut e, Mode::Reassign);
    assert_eq!(board.target, ClickTarget::From);
    board.set_mode(&mut e, Mode::Swap);
    assert_eq!(board.target, ClickTarget::From);
}
#[test]
fn busy_confirmation_and_invalid_slot_clicks_preserve_selection_target_and_history() {
    let mut e = Editor::new(captured());
    let mut board = KeyboardState::default();
    board.set_mode(&mut e, Mode::Swap);
    let selection = e.selection;
    for (blocked, confirmation, popup, slot) in [
        (true, false, false, 30),
        (false, true, false, 30),
        (false, false, true, 30),
        (false, false, false, 66),
    ] {
        assert!(!board.click(&mut e, slot, blocked, confirmation, popup));
        assert_eq!(e.selection, selection);
        assert_eq!(board.target, ClickTarget::From);
    }
    assert!(!board.set_target(ClickTarget::To, true, false, false));
    assert!(!board.set_target(ClickTarget::To, false, true, false));
    assert!(!board.set_target(ClickTarget::To, false, false, true));
    assert_eq!(board.target, ClickTarget::From);
    assert!(!e.dirty());
    assert!(!e.can_undo());
}
#[test]
fn unsupported_unknown_macro_and_unassigned_keys_keep_raw_records_on_navigation() {
    let mut p = captured();
    p.records[30][0][6] = 178;
    p.set_definition(
        31,
        &Definition {
            kind: 1,
            keys: vec![43, 44],
            interval: 100,
            cycles: 1,
            custom_delay: false,
            delays: vec![],
        },
    )
    .unwrap();
    p.set_definition(
        32,
        &Definition {
            keys: vec![],
            ..single(43)
        },
    )
    .unwrap();
    let mut e = Editor::new(p);
    let before = e.draft.reports();
    for slot in [30, 31, 32] {
        e.select_from(slot);
        assert!(!e.output_ready());
        assert!(e.proposed().is_err());
        assert!(e.edit().is_err());
        assert_eq!(e.draft.reports(), before);
        assert!(!e.can_undo());
    }
    e.select_from(31);
    e.choose_output(43);
    e.edit().unwrap();
    assert_eq!(e.preview().unwrap(), vec![31]);
}
#[test]
fn raw_dirty_markers_distinguish_padding_and_follow_layer_and_undo() {
    let mut e = Editor::new(captured());
    let initial = key_view(&e, 30);
    assert_eq!(initial.label, "A");
    assert!(!initial.changed);
    let mut imported = e.draft.clone();
    imported.records[30][0][63] = 73;
    e.import(&imported).unwrap();
    assert_eq!(key_view(&e, 30).label, "A");
    assert!(key_view(&e, 30).changed);
    assert!(key_view(&e, 30).tooltip.contains("変更あり"));
    e.select_layer(1);
    assert!(!key_view(&e, 30).changed);
    e.select_layer(0);
    assert!(e.undo());
    assert!(!key_view(&e, 30).changed);
    assert!(!e.dirty());
}
#[test]
fn every_fixed_slot_remains_reachable_at_minimum_width_and_duplicates_do_not_reorder() {
    for width in [620., 660., 920.] {
        let rows = row_geometry(width);
        assert_eq!(
            rows.iter().map(Vec::len).collect::<Vec<_>>(),
            vec![15, 14, 13, 13, 11]
        );
        let mut slots = vec![];
        for row in &rows {
            assert!(row[0].x.abs() < 0.01);
            assert!((row.last().unwrap().x + row.last().unwrap().width - width).abs() < 0.01);
            for (i, key) in row.iter().enumerate() {
                assert!(key.width >= 36.);
                if i > 0 {
                    assert!(key.x > row[i - 1].x + row[i - 1].width);
                }
                slots.push(key.slot);
            }
        }
        assert_eq!(slots, (0..66).collect::<Vec<_>>());
        assert_eq!(
            rows[2][1].slot, 30,
            "stable slot #31 remains at the A-shaped position"
        );
        assert_eq!(rows[4][1].slot, 56);
        assert_eq!(rows[4][6].slot, 61);
    }
    let mut e = Editor::new(captured());
    e.draft.set_definition(30, &single(44)).unwrap();
    assert_eq!(key_view(&e, 30).label, key_view(&e, 31).label);
    let mut board = KeyboardState::default();
    board.click(&mut e, 30, false, false, false);
    assert_eq!(e.selection.from, Some(30));
    board.click(&mut e, 31, false, false, false);
    assert_eq!(e.selection.from, Some(31));
}
#[test]
fn clicking_swap_from_again_clears_it_without_editing() {
    let mut e = Editor::new(captured());
    let mut board = KeyboardState::default();
    board.set_mode(&mut e, Mode::Swap);
    board.click(&mut e, 30, false, false, false);
    board.click(&mut e, 30, false, false, false);
    let view = key_view(&e, 30);
    assert!(!view.is_from && !view.is_to);
    assert!(!e.dirty());
    assert!(e.proposed().is_err());
}
#[test]
fn demo_uses_the_captured_profile_without_changing_core_demo_semantics() {
    let mut demo = DemoBackend::default();
    let (p, _) = demo.read("demo://keyboard").unwrap();
    assert_eq!(p.reports(), captured().reports());
    assert_eq!(p.definition(30).unwrap().keys, vec![43]);
    assert_eq!(p.definition(56).unwrap().keys, vec![166]);
    assert_eq!(p.definition(61).unwrap().keys, vec![156]);
    assert_eq!(p.definition(66 + 7).unwrap().keys, vec![149]);
    assert_eq!(Profile::demo().definition(54).unwrap().keys, vec![156]);
}

#[test]
fn compact_navigation_labels_keep_the_vendor_action_meaning() {
    let mut e = Editor::new(captured());
    for (code, label, full) in [
        (84, "Del", "Delete"),
        (87, "↑", "Up"),
        (88, "←", "Left"),
        (89, "↓", "Down"),
        (90, "→", "Right"),
        (85, "特殊", "End"),
        (91, "特殊", "Num Lock"),
    ] {
        e.draft.set_definition(30, &single(code)).unwrap();
        let view = key_view(&e, 30);
        assert_eq!(
            view.label, label,
            "code {code} must not acquire another action's visual label"
        );
        assert!(view.tooltip.contains(full));
    }
}

#[test]
fn official_position_legends_remain_distinct_from_current_captured_assignments() {
    let e = Editor::new(captured());
    for (slot, legend, current) in [
        (30, "A", "A"),
        (56, "Left Fn", "LFn"),
        (60, "Right Fn", "RAlt"),
        (61, "Right Alt", "RFn"),
        (13, "`", "`"),
        (14, "Backspace", "BS"),
        (28, "\\", "\\"),
        (54, "Delete", "Del"),
    ] {
        let view = key_view(&e, slot);
        assert_eq!(view.legend, legend);
        assert_eq!(
            view.label, current,
            "current settings must not be reset to match a printed legend"
        );
        assert!(view.tooltip.contains(legend));
    }
}

#[test]
fn importing_an_advanced_selected_action_requires_an_explicit_replacement() {
    let mut e = Editor::new(captured());
    e.select_from(1);
    let slot = e.selection.from.unwrap();
    let mut imported = e.draft.clone();
    imported
        .set_definition(
            slot,
            &Definition {
                kind: 1,
                keys: vec![43, 44],
                interval: 100,
                cycles: 1,
                custom_delay: false,
                delays: vec![],
            },
        )
        .unwrap();
    e.import(&imported).unwrap();
    let advanced = e.draft.reports();
    assert!(
        !e.output_ready(),
        "an imported combination cannot inherit the old output choice"
    );
    assert!(e.proposed().is_err());
    let mut board = KeyboardState::default();
    assert!(board.click(&mut e, slot, false, false, false));
    assert_eq!(e.selection.from, None);
    assert!(e.edit().is_err());
    assert_eq!(e.draft.reports(), advanced);
    assert!(e.undo());
    e.select_from(slot);
    assert_eq!(e.selection.to_code, 15);
    assert!(e.output_ready());
    assert!(e.redo());
    assert!(!e.output_ready());
    assert!(e.proposed().is_err());
    e.choose_output(43);
    e.edit().unwrap();
    assert!(e.output_ready());
    assert!(
        e.undo(),
        "undoing an explicit replacement restores the advanced action"
    );
    assert!(!e.output_ready());
    assert_eq!(e.draft.reports(), advanced);
    assert!(e.redo());
    assert!(e.output_ready());
    assert_eq!(e.selection.to_code, 43);
}
#[test]
fn history_restores_unassigned_and_special_actions_without_a_stale_replacement() {
    for original in [
        Definition {
            keys: vec![],
            ..single(43)
        },
        single(149),
    ] {
        let mut p = captured();
        p.set_definition(1, &original).unwrap();
        let mut e = Editor::new(p);
        e.select_from(1);
        assert!(!e.output_ready());
        e.choose_output(43);
        e.edit().unwrap();
        assert!(e.undo());
        assert_eq!(e.draft.definition(1).unwrap(), original);
        assert!(!e.output_ready());
        assert!(e.proposed().is_err());
        assert!(e.redo());
        assert!(e.output_ready());
        assert_eq!(e.selection.to_code, 43);
    }
}
#[test]
fn applying_an_updated_selected_record_refreshes_both_known_and_unknown_action_state() {
    let mut e = Editor::new(captured());
    e.select_from(1);
    e.choose_output(44);
    let mut p = e.draft.clone();
    p.records[e.selection.from.unwrap()][0][6] = 178;
    e.accept_applied(p);
    assert!(!e.output_ready());
    assert!(e.proposed().is_err());
    let mut p = e.draft.clone();
    p.records[e.selection.from.unwrap()][0][6] = 43;
    e.accept_applied(p);
    assert!(e.output_ready());
    assert_eq!(e.selection.to_code, 43);
    assert_eq!(e.proposed().unwrap().reports(), e.draft.reports());
}
#[test]
fn unrelated_import_history_and_acceptance_preserve_the_pending_output() {
    let mut e = Editor::new(captured());
    e.select_from(30);
    e.choose_output(45);
    let mut p = e.draft.clone();
    p.set_definition(31, &single(46)).unwrap();
    e.import(&p).unwrap();
    assert_eq!(e.selection.to_code, 45);
    assert!(e.output_ready());
    assert!(e.undo());
    assert_eq!(e.selection.to_code, 45);
    assert!(e.redo());
    assert_eq!(e.selection.to_code, 45);
    e.accept_applied(e.draft.clone());
    assert_eq!(e.selection.to_code, 45);
    assert!(e.output_ready());
}

#[test]
fn selected_opaque_padding_changes_refresh_pending_output_from_the_current_action() {
    let mut e = Editor::new(captured());
    e.select_from(30);
    e.choose_output(44);
    let mut p = e.draft.clone();
    p.records[30][0][63] = 73;
    e.import(&p).unwrap();
    assert_eq!(
        e.selection.to_code, 43,
        "even raw-only changes refresh the selected settings"
    );
    assert!(e.output_ready());
    assert!(e.undo());
    assert_eq!(e.selection.to_code, 43);
}
