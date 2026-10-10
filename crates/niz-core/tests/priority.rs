use niz_core::{
    Definition, DeviceIdentity, Profile, Report, encode_definition, import_windows_profile,
};

const LIVE: &str =
    include_str!("../../../docs/validation/fixtures/atom66-xrgb-ble-v1.0.48-settings.json");
const OPAQUE: [usize; 6] = [86, 87, 94, 152, 153, 160];
fn live() -> Profile {
    Profile::from_json(LIVE).unwrap()
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
fn assert_records_equal(actual: &Profile, expected: &[Vec<Report>]) {
    assert_eq!(actual.records.len(), expected.len());
    for (i, (actual, expected)) in actual.records.iter().zip(expected).enumerate() {
        assert_eq!(actual, expected, "record {i}");
    }
}
fn nine_groups(mut p: Profile) -> Profile {
    let base = p.records.clone();
    for group in 3..9 {
        for (key, original) in base.iter().take(66).enumerate() {
            let mut record = original.clone();
            for report in &mut record {
                report[2] = (group + 1) as u8;
                report[63] = (group * 7 + key) as u8;
            }
            if key == 20 {
                record[0][6] = 250;
            }
            p.records.push(record);
        }
    }
    p
}
#[test]
fn actual_baseline_allows_unrelated_remap_preserving_extra_fn_and_six_opaque_records() {
    let baseline = live();
    assert_eq!(baseline.definition(56).unwrap().keys, vec![166]);
    assert_eq!(baseline.definition(61).unwrap().keys, vec![156]);
    assert_eq!(baseline.definition(60).unwrap().keys, vec![71]);
    assert_eq!(baseline.definition(126).unwrap().keys, vec![156]);
    assert_eq!(baseline.definition(192).unwrap().keys, vec![156]);
    for (i, code) in OPAQUE.into_iter().zip([182, 178, 179, 182, 178, 179]) {
        assert_eq!(baseline.definition(i).unwrap().keys, vec![code]);
    }
    assert!(baseline.validate_for_writing().is_err());
    baseline.validate_changes_against(&baseline).unwrap();
    let mut draft = baseline.clone();
    draft.set_definition(0, &single(43)).unwrap();
    draft.validate_changes_against(&baseline).unwrap();
    for i in OPAQUE {
        assert_eq!(draft.records[i], baseline.records[i]);
    }
    assert_eq!(draft.differences(&baseline).unwrap(), vec![0]);
}
#[test]
fn extra_alternate_fn_is_local_when_reassigned_and_exact_noop_does_not_promote_it() {
    let baseline = live();
    for index in [126, 192] {
        let mut p = baseline.clone();
        p.set_definition(index, &p.definition(index).unwrap())
            .unwrap();
        assert_records_equal(&p, &baseline.records);
        p.set_definition(index, &single(43)).unwrap();
        assert_eq!(p.differences(&baseline).unwrap(), vec![index]);
        p.validate_changes_against(&baseline).unwrap();
        assert_eq!(p.records[60], baseline.records[60]);
    }
}
#[test]
fn normal_anchor_removal_rejects_even_with_extra_alternate_fn_and_duplicate_anchor_allows_move() {
    let mut p = live();
    let original = p.records.clone();
    assert!(p.set_definition(61, &single(43)).is_err());
    assert_records_equal(&p, &original);
    p.set_definition(0, &single(156)).unwrap();
    for layer in 0..3 {
        assert_eq!(p.definition(layer * 66).unwrap(), single(156));
    }
    p.set_definition(61, &single(43)).unwrap();
    p.validate_changes_against(&live()).unwrap();
    assert_eq!(p.definition(126).unwrap(), single(156));
}
#[test]
fn ordinary_swaps_each_layer_move_raw_records_only_and_are_involutions() {
    for layer in 0..3 {
        let mut p = nine_groups(live());
        let from = layer * 66 + 1;
        let to = layer * 66 + 2;
        p.records[from][0][63] = 0xa5;
        p.records[to][0][63] = 0x5a;
        let baseline = p.clone();
        let mut expected = p.records.clone();
        expected.swap(from, to);
        expected[from][0][3] = (from % 66 + 1) as u8;
        expected[to][0][3] = (to % 66 + 1) as u8;
        p.swap_keys(from, to).unwrap();
        assert_records_equal(&p, &expected);
        p.validate_changes_against(&baseline).unwrap();
        p.swap_keys(from, to).unwrap();
        assert_records_equal(&p, &baseline.records);
        assert_eq!(p.counters, baseline.counters);
        assert_eq!(p.captured_at, baseline.captured_at);
    }
}
#[test]
fn fn_anchor_swaps_exchange_six_records_from_any_selected_layer() {
    for layer in 0..3 {
        for (a, b) in [(56, 0), (56, 61), (61, 60), (56, 1)] {
            let mut p = nine_groups(live());
            if b == 1 {
                p.set_definition(1, &single(166)).unwrap();
                for row in 0..3 {
                    p.records[row * 66 + 56][0][63] = 0x50 + row as u8;
                    p.records[row * 66 + 1][0][63] = 0x90 + row as u8;
                }
            }
            let baseline = p.clone();
            let mut expected = p.records.clone();
            for row in 0..3 {
                let from = row * 66 + a;
                let to = row * 66 + b;
                expected.swap(from, to);
                for r in &mut expected[from] {
                    r[3] = (a + 1) as u8;
                }
                for r in &mut expected[to] {
                    r[3] = (b + 1) as u8;
                }
            }
            p.swap_keys(layer * 66 + a, layer * 66 + b).unwrap();
            assert_records_equal(&p, &expected);
            p.validate_changes_against(&baseline).unwrap();
            p.swap_keys(layer * 66 + a, layer * 66 + b).unwrap();
            assert_records_equal(&p, &baseline.records);
        }
    }
}
#[test]
fn alternate_only_fn_swap_is_layer_local() {
    for layer in [1, 2] {
        let mut p = live();
        let baseline = p.clone();
        let from = layer * 66 + 60;
        let to = layer * 66;
        p.swap_keys(from, to).unwrap();
        assert_eq!(p.differences(&baseline).unwrap(), vec![to, from]);
        assert_eq!(p.definition(60).unwrap(), single(71));
        p.validate_changes_against(&baseline).unwrap();
        p.swap_keys(from, to).unwrap();
        assert_records_equal(&p, &baseline.records);
    }
}
#[test]
fn linked_swap_checks_all_six_sources_before_committing() {
    for unknown in [56, 0, 122, 66, 188, 132] {
        let mut p = live();
        p.records[unknown] = encode_definition(unknown, &single(43)).unwrap();
        p.records[unknown][0][6] = 250;
        // Unknown at an Fn anchor also destroys that anchor; retain another synchronized Left Fn.
        if unknown % 66 == 56 {
            for layer in 0..3 {
                p.records[layer * 66 + 1] =
                    encode_definition(layer * 66 + 1, &single(166)).unwrap();
            }
        }
        let original = p.records.clone();
        assert!(
            p.swap_keys(56, 0).is_err(),
            "moved unknown source {unknown}"
        );
        assert_records_equal(&p, &original);
    }
}
#[test]
fn swap_rejects_invalid_indices_cross_layers_opaque_sources_and_invalid_noops_atomically() {
    let baseline = live();
    for (from, to) in [
        (0, 66),
        (0, 198),
        (198, 198),
        (usize::MAX, usize::MAX),
        (86, 87),
        (87, 0),
        (94, 1),
        (152, 153),
        (160, 132),
        (86, 86),
    ] {
        let mut p = nine_groups(baseline.clone());
        let original = p.records.clone();
        assert!(p.swap_keys(from, to).is_err(), "accepted {from} -> {to}");
        assert_records_equal(&p, &original);
    }
    for index in [0, 56, 61, 126, 192] {
        let mut p = baseline.clone();
        p.swap_keys(index, index).unwrap();
        assert_records_equal(&p, &baseline.records);
    }
    let mut malformed = baseline.clone();
    malformed.records[0][0][3] = 2;
    let original = malformed.records.clone();
    assert!(malformed.swap_keys(0, 0).is_err());
    assert_records_equal(&malformed, &original);
}
#[test]
fn equal_semantics_with_distinct_raw_padding_are_still_swapped() {
    let mut p = live();
    p.records[0] = encode_definition(0, &single(43)).unwrap();
    p.records[1] = encode_definition(1, &single(43)).unwrap();
    p.records[0][0][63] = 0x34;
    p.records[1][0][63] = 0x56;
    let baseline = p.clone();
    p.swap_keys(0, 1).unwrap();
    assert_eq!(p.records[0][0][63], 0x56);
    assert_eq!(p.records[1][0][63], 0x34);
    assert_eq!(p.differences(&baseline).unwrap(), vec![0, 1]);
    p.swap_keys(0, 1).unwrap();
    assert_records_equal(&p, &baseline.records);
}
#[test]
fn baseline_relative_validator_rejects_any_changed_opaque_source_or_target_or_tail() {
    let baseline = live();
    for index in OPAQUE {
        for change in 0..3 {
            let mut p = baseline.clone();
            match change {
                0 => p.records[index][0][6] = 43,
                1 => p.records[index][0][6] = 251,
                _ => p.records[index][0][63] ^= 1,
            }
            assert!(
                p.validate_changes_against(&baseline).is_err(),
                "accepted opaque delta at {index}"
            );
        }
    }
    let mut p = baseline.clone();
    p.records[0][0][6] = 178;
    assert!(p.validate_changes_against(&baseline).is_err());
    let mut p = baseline.clone();
    p.records.swap(86, 0);
    p.records[0][0][3] = 1;
    p.records[0][0][2] = 1;
    p.records[86][0][3] = 21;
    p.records[86][0][2] = 2;
    assert!(p.validate_changes_against(&baseline).is_err());
}
#[test]
fn baseline_relative_validator_checks_identity_firmware_groups_and_both_fn_structures() {
    let baseline = nine_groups(live());
    for change in 0..7 {
        let mut p = baseline.clone();
        match change {
            0 => p.identity.product_id ^= 1,
            1 => p.firmware.push('x'),
            2 => p.records.truncate(198),
            3 => p.records[198][0][63] ^= 1,
            4 => p.records[122][0][6] = 43,
            5 => p.records[0][0][3] = 2,
            _ => {
                for layer in 0..3 {
                    p.records[layer * 66 + 61][0][6] = 43
                }
            }
        }
        assert!(
            p.validate_changes_against(&baseline).is_err(),
            "accepted mismatch {change}"
        );
    }
    let mut bad = baseline.clone();
    bad.records[122][0][6] = 43;
    assert!(baseline.validate_changes_against(&bad).is_err());
    let mut bad = baseline.clone();
    bad.records[0][0][3] = 2;
    assert!(baseline.validate_changes_against(&bad).is_err());
    let mut metadata = baseline.clone();
    metadata.counters = vec![33; 66];
    metadata.captured_at = "new timestamp".into();
    metadata.validate_changes_against(&baseline).unwrap();
}
#[test]
fn native_import_retains_unchanged_live_opaque_records_and_target_extra_groups() {
    let baseline = nine_groups(live());
    let mut imported = live();
    imported.set_definition(0, &single(43)).unwrap();
    let merged = baseline.merge_import(&imported).unwrap();
    assert_eq!(merged.group_count(), 9);
    assert_eq!(merged.differences(&baseline).unwrap(), vec![0]);
    for i in OPAQUE {
        assert_eq!(merged.records[i], baseline.records[i]);
    }
    for i in 198..594 {
        assert_eq!(merged.records[i], baseline.records[i]);
    }
    assert_eq!(merged.counters, baseline.counters);
    assert_eq!(merged.captured_at, baseline.captured_at);
    imported.records[86][0][63] ^= 1;
    assert!(baseline.merge_import(&imported).is_err());
    let mut imported = baseline.clone();
    imported.records[198][0][63] ^= 1;
    assert!(baseline.merge_import(&imported).is_err());
}
#[test]
fn native_import_rejects_metadata_mismatch_and_legacy_import_remains_strict() {
    let baseline = live();
    for change in 0..3 {
        let mut imported = baseline.clone();
        match change {
            0 => imported.firmware.push('x'),
            1 => imported.identity = DeviceIdentity::default(),
            _ => imported.records[0][0][6] = 250,
        }
        assert!(baseline.merge_import(&imported).is_err());
    }
    let mut xml = "<66EC(XRGB)BLe><CurrentSettings>".to_string();
    for layer in 0..3 {
        for key in 1..=66 {
            let code = if key == 57 {
                166
            } else if key == 62 {
                156
            } else if key == 21 {
                178
            } else {
                43
            };
            xml.push_str(&format!(
                "<KEY ID=\"{key}\" Level=\"{layer}\" Mode=\"1\" HWCode=\"{code}\"/>"
            ));
        }
    }
    xml.push_str("</CurrentSettings></66EC(XRGB)BLe>");
    assert!(import_windows_profile(&xml).is_err());
}

// Literal wire builder independent of the production definition encoder.
fn raw_macro(index: usize, n: usize, form: u8, terminal: [u8; 2], padding: u8) -> Vec<Report> {
    let mut body = if form == 0 {
        vec![43; n]
    } else {
        [43, 200, 0, 17].repeat(n)
    };
    if form == 2 {
        body.truncate(body.len() - 3);
    } else if form == 1 {
        let len = body.len();
        body[len - 2..].copy_from_slice(&terminal);
    }
    let len = body.len();
    body.chunks(53)
        .enumerate()
        .map(|(i, chunk)| {
            let mut r = [padding.wrapping_add(i as u8); 64];
            r[..11].copy_from_slice(&[
                0,
                0xf0,
                (index / 66 + 1) as u8,
                (index % 66 + 1) as u8,
                2,
                2,
                u8::from(form != 0),
                0,
                30,
                (len >> 8) as u8,
                len as u8,
            ]);
            r[11..11 + chunk.len()].copy_from_slice(chunk);
            r
        })
        .collect()
}
fn moved(mut record: Vec<Report>, index: usize) -> Vec<Report> {
    for r in &mut record {
        r[2] = (index / 66 + 1) as u8;
        r[3] = (index % 66 + 1) as u8;
    }
    record
}
#[test]
fn raw_macro_swaps_keep_every_continuation_native_short_terminal_and_padding_byte() {
    for layer in 0..3 {
        let from = layer * 66;
        let to = layer * 66 + 1;
        for (left_n, left_form, right_n, right_form) in [
            (53, 0, 54, 0),
            (14, 1, 27, 2),
            (2048, 1, 13, 2),
            (1, 2, 54, 0),
        ] {
            let mut p = nine_groups(live());
            p.records[from] = raw_macro(from, left_n, left_form, [0xab, 0xcd], 0x90);
            p.records[to] = raw_macro(to, right_n, right_form, [0x12, 0x34], 0x60);
            let baseline = p.clone();
            let mut expected = p.records.clone();
            expected[from] = moved(baseline.records[to].clone(), from);
            expected[to] = moved(baseline.records[from].clone(), to);
            p.swap_keys(from, to).unwrap();
            assert_records_equal(&p, &expected);
            p.validate_changes_against(&baseline).unwrap();
            p.swap_keys(from, to).unwrap();
            assert_records_equal(&p, &baseline.records);
        }
    }
}
#[test]
fn linked_swap_readdresses_all_macro_reports_in_each_moved_column() {
    let mut p = live();
    for layer in 0..3 {
        p.records[layer * 66] = raw_macro(
            layer * 66,
            54 + layer,
            layer as u8,
            [0xab, 0xcd],
            0x40 + layer as u8,
        );
    }
    let baseline = p.clone();
    let mut expected = p.records.clone();
    for layer in 0..3 {
        let from = layer * 66 + 56;
        let to = layer * 66;
        expected[from] = moved(baseline.records[to].clone(), from);
        expected[to] = moved(baseline.records[from].clone(), to);
    }
    p.swap_keys(122, 66).unwrap();
    assert_records_equal(&p, &expected);
    p.validate_changes_against(&baseline).unwrap();
    p.swap_keys(122, 66).unwrap();
    assert_records_equal(&p, &baseline.records);
}
#[test]
fn equal_native_macro_semantics_do_not_erase_distinct_opaque_terminal_fields_on_swap() {
    let mut p = live();
    p.records[0] = raw_macro(0, 14, 1, [0xab, 0xcd], 0x90);
    p.records[1] = raw_macro(1, 14, 1, [0x12, 0x34], 0x90);
    assert_eq!(p.definition(0).unwrap(), p.definition(1).unwrap());
    let baseline = p.clone();
    p.swap_keys(0, 1).unwrap();
    assert_eq!(p.records[0], moved(baseline.records[1].clone(), 0));
    p.swap_keys(0, 1).unwrap();
    assert_records_equal(&p, &baseline.records);
}
