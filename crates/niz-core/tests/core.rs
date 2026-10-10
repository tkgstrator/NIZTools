use niz_core::*;
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
fn fixture(groups: usize) -> Vec<Report> {
    // Independent wire fixture, not the encoder under test.
    (0..groups * 66)
        .map(|i| {
            let mut r = [0; 64];
            r[..7].copy_from_slice(&[0, 0xf0, (i / 66 + 1) as u8, (i % 66 + 1) as u8, 0, 1, 43]);
            if i % 66 == 54 {
                r[6] = 156;
            }
            if i % 66 == 58 {
                r[6] = 166;
            }
            r[63] = 0xa5;
            r
        })
        .collect()
}
fn profile(groups: usize) -> Profile {
    Profile::from_reports(
        "66EC(S)Ble test".into(),
        DeviceIdentity {
            vendor_id: 0x483,
            product_id: 0x502a,
            usage_page: 0x8c,
            usage: 1,
        },
        fixture(groups),
    )
    .unwrap()
}
#[test]
fn command_keeps_logical_zero_prefix() {
    let r = command(0xf9);
    assert_eq!(&r[..3], &[0, 0xf9, 0]);
    assert!(r[2..].iter().all(|b| *b == 0));
}
#[test]
fn golden_single_and_repeat_headers() {
    let mut d = single(68);
    d.keys.push(58);
    assert_eq!(
        &encode_definition(0, &d).unwrap()[0][..8],
        &[0, 0xf0, 1, 1, 0, 2, 68, 58]
    );
    d.kind = 1;
    d.interval = 0x1234;
    d.keys = vec![43, 44];
    assert_eq!(
        &encode_definition(67, &d).unwrap()[0][..10],
        &[0, 0xf0, 2, 2, 1, 0x12, 0x34, 2, 43, 44]
    );
}
#[test]
fn macro_crosses_53_byte_boundary() {
    for n in [53, 54] {
        let d = Definition {
            kind: 2,
            keys: vec![43; n],
            interval: 30,
            cycles: 2,
            custom_delay: false,
            delays: vec![],
        };
        let rs = encode_definition(0, &d).unwrap();
        assert_eq!(rs.len(), if n == 53 { 1 } else { 2 });
        assert_eq!(&rs[0][..11], &[0, 0xf0, 1, 1, 2, 2, 0, 0, 30, 0, n as u8]);
        assert_eq!(decode_definition(&rs).unwrap(), d);
    }
}
#[test]
fn custom_delays_accept_native_and_short_forms() {
    let mut r = [0; 64];
    r[..11].copy_from_slice(&[0, 0xf0, 1, 1, 2, 1, 1, 0, 0, 0, 8]);
    r[11..19].copy_from_slice(&[43, 200, 0x12, 0x34, 44, 200, 0xab, 0xcd]);
    let d = decode_definition(&[r]).unwrap();
    assert_eq!(d.keys, vec![43, 44]);
    assert_eq!(d.delays, vec![0x1234]);
    r[10] = 5;
    assert_eq!(decode_definition(&[r]).unwrap(), d);
    let encoded = encode_definition(0, &d).unwrap();
    assert_eq!(encoded[0][10], 8);
    assert_eq!(&encoded[0][16..19], &[200, 0, 0]);
}
#[test]
fn codec_rejects_incomplete_headers_and_invalid_sequences() {
    assert!(decode_definition(&[]).is_err());
    let mut r = fixture(3)[0];
    r[5] = 59;
    assert!(decode_definition(&[r]).is_err());
    r[4] = 2;
    r[5] = 1;
    r[6] = 1;
    r[9] = 0;
    r[10] = 2;
    assert!(decode_definition(&[r]).is_err());
    let mut d = single(43);
    d.kind = 2;
    d.cycles = 0;
    assert!(encode_definition(0, &d).is_err());
    assert!(encode_definition(594, &single(43)).is_err());
    assert!(encode_definition(0, &single(250)).is_err());
}
#[test]
fn profile_requires_all_unique_valid_records() {
    for groups in [3, 9] {
        let p = profile(groups);
        assert_eq!(p.group_count(), groups);
        assert_eq!(p.reports(), fixture(groups));
    }
    let mut r = fixture(3);
    r.pop();
    assert!(Profile::from_reports("v".into(), DeviceIdentity::default(), r).is_err());
    let mut r = fixture(3);
    r[1] = r[0];
    assert!(Profile::from_reports("v".into(), DeviceIdentity::default(), r).is_err());
    for offset in [0, 1, 2, 3, 4] {
        let mut r = fixture(3);
        r[0][offset] = 255;
        assert!(Profile::from_reports("v".into(), DeviceIdentity::default(), r).is_err());
    }
    assert!(Profile::from_reports("v".into(), DeviceIdentity::default(), fixture(4)).is_err());
    assert!(
        Profile::from_reports("v".into(), DeviceIdentity::default(), vec![[0; 64]; 8193]).is_err()
    );
}
#[test]
fn edit_preserves_unrelated_raw_and_noop_macro() {
    let mut p = profile(9);
    let before = p.reports();
    p.set_definition(0, &single(44)).unwrap();
    let after = p.reports();
    let mut expected = before;
    expected[0][6] = 44;
    assert_eq!(after, expected);
    assert_eq!(p.differences(&profile(9)).unwrap(), vec![0]);
    let mut rs = fixture(3);
    rs[0][4] = 2;
    rs[0][5] = 1;
    rs[0][6] = 1;
    rs[0][7] = 0;
    rs[0][8] = 0;
    rs[0][9] = 0;
    rs[0][10] = 5;
    rs[0][11..16].copy_from_slice(&[43, 200, 0, 30, 44]);
    let mut p = Profile::from_reports("v".into(), DeviceIdentity::default(), rs.clone()).unwrap();
    p.set_definition(0, &p.definition(0).unwrap()).unwrap();
    assert_eq!(p.reports(), rs);
}
#[test]
fn unknown_semantic_code_retained_but_uneditable() {
    let mut rs = fixture(3);
    rs[0][6] = 250;
    let mut p = Profile::from_reports("v".into(), DeviceIdentity::default(), rs.clone()).unwrap();
    assert_eq!(p.reports(), rs);
    assert!(p.set_definition(0, &single(44)).is_err());
    assert!(p.validate_for_writing().is_err());
}
#[test]
fn fn_edits_sync_and_cannot_lose_either_fn() {
    let mut p = profile(3);
    assert!(p.validate_for_writing().is_ok());
    let before = p.reports();
    assert!(p.set_definition(54, &single(43)).is_err());
    assert_eq!(p.reports(), before);
    p.set_definition(0, &single(156)).unwrap();
    assert_eq!(p.definition(66).unwrap(), single(156));
    assert_eq!(p.definition(132).unwrap(), single(156));
    p.set_definition(54, &single(43)).unwrap();
    assert!(p.validate_for_writing().is_ok());
    let mut bad = profile(3);
    bad.records[66 + 54][0][6] = 43;
    assert!(bad.validate_for_writing().is_err());
}
#[test]
fn differences_detect_padding_changes() {
    let a = profile(9);
    let mut b = a.clone();
    b.records[0][0][63] ^= 1;
    assert_eq!(a.differences(&b).unwrap(), vec![0]);
}
#[test]
fn merge_three_onto_nine_preserves_baseline_metadata_and_extra_groups() {
    let mut a = profile(3);
    a.set_definition(0, &single(44)).unwrap();
    let mut b = profile(9);
    b.counters = vec![7; 66];
    b.captured_at = "baseline timestamp".into();
    let c = b.merge_import(&a).unwrap();
    assert_eq!(c.group_count(), 9);
    assert_eq!(c.definition(0).unwrap(), single(44));
    assert_eq!(&c.records[198..], &b.records[198..]);
    assert_eq!(c.counters, b.counters);
    assert_eq!(c.captured_at, b.captured_at);
    a.firmware = "other firmware".into();
    assert!(b.merge_import(&a).is_err());
    a.firmware = b.firmware.clone();
    a.identity.product_id = 0x522a;
    assert!(b.merge_import(&a).is_err());
}
#[test]
fn json_roundtrips_raw_and_rejects_invalid_shape() {
    let p = profile(9);
    let text = p.to_json().unwrap();
    assert!(text.contains("00f0010100012b"));
    assert_eq!(Profile::from_json(&text).unwrap().reports(), p.reports());
    for text in ["{}", "{\"reports\":[]}", "null"] {
        assert!(Profile::from_json(text).is_err());
    }
    let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
    value["reports"][0] = "00".into();
    assert!(Profile::from_json(&value.to_string()).is_err());
    assert!(Profile::from_json(&" ".repeat(4 * 1024 * 1024 + 1)).is_err());
}
#[test]
fn key_names_and_aliases_use_niz_codes() {
    assert_eq!(parse_key("A").unwrap(), 43);
    assert_eq!(parse_key("enter").unwrap(), 54);
    assert_eq!(parse_key("L-Fn").unwrap(), 166);
    assert_eq!(parse_key("#250").unwrap(), 250);
    assert_eq!(parse_key("0xff").unwrap(), 255);
    assert!(parse_key("#256").is_err());
    assert_eq!(key_name(156), "Right Fn");
    assert_eq!(Profile::demo().group_count(), 3);
    assert!(Profile::demo().validate_for_writing().is_ok());
}
fn xml() -> String {
    let mut s = "<66EC(S)Ble><CurrentSettings>".to_string();
    for level in 0..3 {
        for id in 1..=66 {
            let code = if id == 55 {
                156
            } else if id == 59 {
                166
            } else {
                43
            };
            s.push_str(&format!(
                "<KEY ID=\"{id}\" Level=\"{level}\" Mode=\"1\" HWCode=\"{code}\"/>"
            ));
        }
    }
    s.push_str("</CurrentSettings></66EC(S)Ble>");
    s
}
#[test]
fn legacy_import_and_semantic_merge_preserve_baseline_padding() {
    let p = import_windows_profile(&xml()).unwrap();
    assert_eq!(p.firmware, "Windows .pro (offline)");
    assert_eq!(p.identity, DeviceIdentity::default());
    assert!(p.validate_for_writing().is_ok());
    let b = profile(9);
    let merged = b.merge_import(&p).unwrap();
    assert_eq!(merged.records, b.records);
    assert_eq!(merged.firmware, b.firmware);
}
#[test]
fn legacy_rejects_dtd_duplicates_missing_keys_and_unknown_modes() {
    for bad in [
        xml().replace("ID=\"2\" Level=\"0\"", "ID=\"1\" Level=\"0\""),
        xml().replace("Mode=\"1\"", "Mode=\"99\""),
        xml().replace("HWCode=\"43\"", "HWCode=\"-1\""),
        "<66EC(S)Ble><CurrentSettings/></66EC(S)Ble>".into(),
        format!("<!DOCTYPE x [<!ENTITY a 'bad'>]>{}", xml()),
    ] {
        assert!(import_windows_profile(&bad).is_err());
    }
}

#[test]
fn unicode_key_input_never_panics_and_arrows_are_accepted() {
    assert_eq!(parse_key("↑").unwrap(), 87);
    assert!(parse_key("あ").is_err());
}
#[test]
fn captures_include_timestamp_metadata() {
    let p = profile(3);
    assert!(p.captured_at.parse::<u64>().unwrap() > 0);
}
#[test]
fn editing_native_macro_retains_opaque_terminal_secondary() {
    let mut rs = fixture(3);
    rs[0][..11].copy_from_slice(&[0, 0xf0, 1, 1, 2, 1, 1, 0, 0, 0, 8]);
    rs[0][11..19].copy_from_slice(&[43, 200, 0, 30, 44, 200, 0xab, 0xcd]);
    let mut p = Profile::from_reports("v".into(), DeviceIdentity::default(), rs).unwrap();
    let mut d = p.definition(0).unwrap();
    d.keys[0] = 45;
    p.set_definition(0, &d).unwrap();
    assert_eq!(&p.records[0][0][17..19], &[0xab, 0xcd]);
    assert_eq!(p.records[0][0][11], 45);
}
#[test]
fn legacy_custom_macro_accepts_empty_item_lists() {
    let mut text = xml();
    text=text.replacen("<KEY ID=\"1\" Level=\"0\" Mode=\"1\" HWCode=\"43\"/>","<KEY ID=\"1\" Level=\"0\" Mode=\"3\"><ComboKey><List IsAutoInterval=\"0\" PlayMode=\"0\" Cycles=\"2\"><x ID=\"0\" t1=\"A down\"><List/></x><x ID=\"1\" t1=\"&lt;30 ms&gt;\"/><x ID=\"2\" t1=\"B up\"/></List></ComboKey></KEY>",1);
    let p = import_windows_profile(&text).unwrap();
    assert_eq!(
        p.definition(0).unwrap(),
        Definition {
            kind: 2,
            keys: vec![43, 60],
            interval: 0,
            cycles: 2,
            custom_delay: true,
            delays: vec![30]
        }
    );
}
#[test]
fn editing_short_custom_macro_retains_wire_shape_and_padding() {
    let mut rs = fixture(3);
    rs[0][..11].copy_from_slice(&[0, 0xf0, 1, 1, 2, 1, 1, 0, 0, 0, 5]);
    rs[0][11..19].copy_from_slice(&[43, 200, 0, 30, 44, 0xaa, 0xbb, 0xcc]);
    let mut p = Profile::from_reports("v".into(), DeviceIdentity::default(), rs.clone()).unwrap();
    let mut d = p.definition(0).unwrap();
    d.keys[0] = 45;
    p.set_definition(0, &d).unwrap();
    rs[0][11] = 45;
    assert_eq!(p.records[0][0], rs[0]);
    for (i, expected) in rs.iter().enumerate().skip(1) {
        assert_eq!(&p.records[i][0], expected, "untouched record {i}");
    }
}
#[test]
fn json_rejects_partial_counters_and_private_identity_fields() {
    let p = profile(3);
    let mut v: serde_json::Value = serde_json::from_str(&p.to_json().unwrap()).unwrap();
    v["counters"] = serde_json::json!([1]);
    assert!(Profile::from_json(&v.to_string()).is_err());
    let mut v: serde_json::Value = serde_json::from_str(&p.to_json().unwrap()).unwrap();
    v["identity"]["path"] = serde_json::json!("/private/device");
    assert!(Profile::from_json(&v.to_string()).is_err());
}
#[test]
fn continuation_header_mismatch_rejected() {
    let mut d = single(43);
    d.kind = 3;
    d.keys = vec![43; 54];
    let mut rs = encode_definition(0, &d).unwrap();
    rs[1][3] = 2;
    assert!(decode_definition(&rs).is_err());
    let mut p = fixture(3);
    p.splice(0..1, rs);
    assert!(Profile::from_reports("v".into(), DeviceIdentity::default(), p).is_err());
}
#[test]
fn maximum_custom_macro_remains_bounded_and_roundtrips() {
    let d = Definition {
        kind: 4,
        keys: vec![43; 2048],
        interval: 0,
        cycles: 0,
        custom_delay: true,
        delays: vec![65535; 2047],
    };
    let rs = encode_definition(593, &d).unwrap();
    assert_eq!(rs.len(), 155);
    assert_eq!(decode_definition(&rs).unwrap(), d);
    let mut too_many = d;
    too_many.keys.push(44);
    too_many.delays.push(30);
    assert!(encode_definition(0, &too_many).is_err());
}

#[test]
fn legacy_source_variant_survives_json_and_rejects_cross_variant_merge() {
    let s = import_windows_profile(&xml()).unwrap();
    let text = s.to_json().unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["legacy_root"], "66EC(S)Ble");
    let s = Profile::from_json(&text).unwrap();
    assert!(profile(9).merge_import(&s).is_ok());
    for version in ["66EC(XRGB)Ble 1.0", "66EC(RGB)Ble 1.0", "66EC(S) 1.0"] {
        let mut target = profile(9);
        target.firmware = version.into();
        let before = target.records.clone();
        assert!(
            target.merge_import(&s).is_err(),
            "accepted mismatched target {version}"
        );
        assert_eq!(target.records, before);
    }
    let rgb = import_windows_profile(&xml().replace("66EC(S)Ble", "66EC(XRGB)Ble")).unwrap();
    assert!(profile(3).merge_import(&rgb).is_err());
    let mut target = profile(3);
    target.firmware = "66EC(XRGB)Ble 1.0".into();
    assert!(target.merge_import(&rgb).is_ok());
    let wired = import_windows_profile(&xml().replace("66EC(S)Ble", "66EC(S)")).unwrap();
    assert!(profile(3).merge_import(&wired).is_err());
    target.firmware = "66EC(S) 1.0".into();
    assert!(target.merge_import(&wired).is_ok());
}
#[test]
fn synthetic_legacy_markers_without_model_cannot_authorize_merge() {
    let p = import_windows_profile(&xml()).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&p.to_json().unwrap()).unwrap();
    json.as_object_mut().unwrap().remove("legacy_root");
    let imported = Profile::from_json(&json.to_string()).unwrap();
    assert!(profile(3).merge_import(&imported).is_err());
    json["legacy_root"] = serde_json::json!("unrecognized model");
    assert!(Profile::from_json(&json.to_string()).is_err());
    json["legacy_root"] = serde_json::json!("66EC(S)Ble");
    json["version"] = serde_json::json!("66EC(S)Ble actual");
    assert!(Profile::from_json(&json.to_string()).is_err());
}
#[test]
fn xml_declaration_must_be_valid_utf8_leading_and_unique() {
    let body = xml();
    for declaration in [
        "<?xml version=\"1.0\"?>",
        "<?xml version='1.0' encoding='UTF-8' standalone='yes'?>",
    ] {
        assert!(import_windows_profile(&format!("{declaration}{body}")).is_ok());
    }
    for bad in [
        format!("{body}<?xml version=\"1.0\"?>"),
        body.replace(
            "<CurrentSettings>",
            "<?xml version=\"1.0\"?><CurrentSettings>",
        ),
        format!("<?xml version=\"banana\"?>{body}"),
        format!("<?xml version=\"1.1\"?>{body}"),
        format!("<?xml version=\"1.0\" encoding=\"UTF-16\"?>{body}"),
        format!("<?xml version=\"1.0\"?><?xml version=\"1.0\"?>{body}"),
        format!("<?xml version=\"1.0\" extra=\"unknown\"?>{body}"),
        format!("<?xml version=\"1.0\" version=\"1.0\"?>{body}"),
        format!("<?xml encoding=\"UTF-8\" version=\"1.0\"?>{body}"),
    ] {
        assert!(
            import_windows_profile(&bad).is_err(),
            "accepted malformed declaration"
        );
    }
}

fn independent_macro_record(n: usize, custom: bool, short: bool) -> Vec<Report> {
    let mut body = if custom {
        [43, 200, 0, 9].repeat(n)
    } else {
        vec![43; n]
    };
    if short {
        body.truncate(body.len() - 3);
    } else if custom {
        let len = body.len();
        body[len - 2..].copy_from_slice(&[0xab, 0xcd]);
    }
    let size = body.len();
    body.chunks(53)
        .map(|chunk| {
            let mut report = [0xa5; 64];
            report[..11].copy_from_slice(&[
                0,
                0xf0,
                1,
                1,
                2,
                2,
                u8::from(custom),
                0,
                30,
                (size >> 8) as u8,
                size as u8,
            ]);
            report[11..11 + chunk.len()].copy_from_slice(chunk);
            report
        })
        .collect()
}
#[test]
fn macro_transition_matrix_preserves_unrelated_records_padding_and_terminal_fields() {
    let lengths = [1, 13, 14, 26, 27, 53, 54, 2048];
    let mut cases = 0;
    for old_n in lengths {
        for (old_custom, short) in [(false, false), (true, false), (true, true)] {
            let source = independent_macro_record(old_n, old_custom, short);
            for new_n in lengths {
                for custom in [false, true] {
                    let mut p = profile(3);
                    p.records[0] = source.clone();
                    let untouched = p.records[1..].to_vec();
                    let desired = Definition {
                        kind: 2,
                        keys: vec![44; new_n],
                        interval: 30,
                        cycles: 2,
                        custom_delay: custom,
                        delays: if custom { vec![11; new_n - 1] } else { vec![] },
                    };
                    p.set_definition(0, &desired).unwrap();
                    assert_eq!(
                        p.definition(0).unwrap(),
                        desired,
                        "transition {old_n}/{old_custom}/{short} -> {new_n}/{custom}"
                    );
                    assert_eq!(&p.records[1..], &untouched);
                    p.validate_for_writing().unwrap();
                    let body_size = if custom {
                        new_n * 4 - if old_custom && short { 3 } else { 0 }
                    } else {
                        new_n
                    };
                    assert_eq!(
                        u16::from_be_bytes([p.records[0][0][9], p.records[0][0][10]]) as usize,
                        body_size
                    );
                    for (i, report) in p.records[0].iter().enumerate() {
                        let active = 11 + (body_size - i * 53).min(53);
                        if let Some(old) = source.get(i) {
                            assert_eq!(&report[active..], &old[active..]);
                        } else {
                            assert!(report[active..].iter().all(|b| *b == 0));
                        }
                    }
                    if custom {
                        let expected = if old_custom && !short {
                            [0xab, 0xcd]
                        } else {
                            [0, 0]
                        };
                        if !(old_custom && short) {
                            let offset = body_size - 2;
                            assert_eq!(
                                [
                                    p.records[0][offset / 53][11 + offset % 53],
                                    p.records[0][(offset + 1) / 53][11 + (offset + 1) % 53]
                                ],
                                expected,
                                "terminal field on transition {old_n} -> {new_n}"
                            );
                        }
                    }
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 384);
}
fn custom_xml() -> String {
    xml().replacen("<KEY ID=\"1\" Level=\"0\" Mode=\"1\" HWCode=\"43\"/>","<KEY ID=\"1\" Level=\"0\" Mode=\"3\"><ComboKey><List IsAutoInterval=\"0\" PlayMode=\"0\" Cycles=\"2\"><x ID=\"0\" t1=\"A\"/><x ID=\"1\" t1=\"&lt;30 ms&gt;\"/><x ID=\"2\" t1=\"B\"/></List></ComboKey></KEY>",1)
}
#[test]
fn xml_limits_apply_even_to_ignored_metadata() {
    let body = xml();
    let bounded = format!("{}{}", "<metadata>".repeat(8), "</metadata>".repeat(8));
    assert!(
        import_windows_profile(
            &body.replace("<CurrentSettings>", &format!("{bounded}<CurrentSettings>"))
        )
        .is_ok()
    );
    let deep = format!("{}{}", "<metadata>".repeat(33), "</metadata>".repeat(33));
    assert!(
        import_windows_profile(
            &body.replace("<CurrentSettings>", &format!("{deep}<CurrentSettings>"))
        )
        .is_err()
    );
    let many = "<metadata/>".repeat(20001);
    assert!(
        import_windows_profile(
            &body.replace("<CurrentSettings>", &format!("{many}<CurrentSettings>"))
        )
        .is_err()
    );
}
#[test]
fn xml_rejects_duplicate_attributes_entities_sequence_gaps_and_overflow() {
    let body = xml();
    let valid_custom = custom_xml();
    assert!(import_windows_profile(&valid_custom).is_ok());
    let bads = [
        body.replacen("ID=\"1\"", "ID=\"1\" ID=\"2\"", 1),
        body.replacen("HWCode=\"43\"", "HWCode=\"65536\"", 1),
        body.replacen("HWCode=\"43\"", "HWCode=\"256\"", 1),
        body.replacen("HWCode=\"43\"", "HWCode=\" 43\"", 1),
        body.replacen("HWCode=\"43\"", "HWCode=\"+43\"", 1),
        body.replacen("HWCode=\"43\"", "HWCode=\"43\" extra=\"&unknown;\"", 1),
        body.replacen("HWCode=\"43\"", "HWCode=\"43\" extra=\"&unterminated\"", 1),
        valid_custom.replace("<x ID=\"2\" t1=\"B\"/>", "<x ID=\"3\" t1=\"B\"/>"),
        valid_custom.replace("<x ID=\"1\" t1=\"&lt;30 ms&gt;\"/>", ""),
        valid_custom.replace("&lt;30 ms&gt;", "&lt;65536 ms&gt;"),
        valid_custom.replace("&lt;30 ms&gt;", "&lt;-1 ms&gt;"),
    ];
    for (i, bad) in bads.iter().enumerate() {
        assert!(
            import_windows_profile(bad).is_err(),
            "accepted malformed fixture {i}"
        );
    }
}
#[test]
fn json_rejects_duplicate_fields_invalid_hex_metadata_and_record_sets() {
    let text = profile(3).to_json().unwrap();
    let duplicate = text.replacen("\"schema\": 1", "\"schema\": 1, \"schema\": 1", 1);
    assert!(Profile::from_json(&duplicate).is_err());
    for (name, value) in [
        ("schema", serde_json::json!(2)),
        ("version", serde_json::json!("")),
        ("version", serde_json::json!("v".repeat(257))),
        ("captured_at", serde_json::json!("x".repeat(257))),
    ] {
        let mut json: serde_json::Value = serde_json::from_str(&text).unwrap();
        json[name] = value;
        assert!(
            Profile::from_json(&json.to_string()).is_err(),
            "invalid {name}"
        );
    }
    let mut json: serde_json::Value = serde_json::from_str(&text).unwrap();
    json["reports"][0] = serde_json::json!("gg".repeat(64));
    assert!(Profile::from_json(&json.to_string()).is_err());
    let mut json: serde_json::Value = serde_json::from_str(&text).unwrap();
    json["reports"][1] = json["reports"][0].clone();
    assert!(Profile::from_json(&json.to_string()).is_err());
    let mut json: serde_json::Value = serde_json::from_str(&text).unwrap();
    json["reports"].as_array_mut().unwrap().pop();
    assert!(Profile::from_json(&json.to_string()).is_err());
    let mut json: serde_json::Value = serde_json::from_str(&text).unwrap();
    let report = json["reports"][0].clone();
    json["reports"] = serde_json::json!(vec![report; 8193]);
    assert!(Profile::from_json(&json.to_string()).is_err());
}
#[test]
fn linked_fn_edit_rejection_is_atomic_when_other_layer_is_uneditable() {
    let mut p = profile(3);
    p.records[66][0][6] = 250;
    let before = p.records.clone();
    assert!(p.set_definition(0, &single(156)).is_err());
    assert_eq!(p.records, before);
}
