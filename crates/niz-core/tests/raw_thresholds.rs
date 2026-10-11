use niz_core::{RawThresholdDecoder, Report, command, decode_raw_thresholds};

fn rows() -> Vec<Report> {
    (0..11u8)
        .map(|row| {
            let mut report = command(0xdc);
            report[2] = row + 1;
            report[3] = 1;
            for column in 0..6usize {
                report[4 + column] = row * 6 + column as u8;
            }
            report[10..].fill(0xa5 + row);
            report
        })
        .collect()
}

#[test]
fn exact_image_permutation_maps_every_matrix_cell_to_physical_slot() {
    // Independently recovered 0x9dc6 table, image SHA-256
    // b82e11891dfdc138780a81ac635697a7edb0ca1d4e123d6ad38c81a7ba55bf17.
    let matrix_physical_numbers = [
        1, 2, 3, 4, 5, 6, 16, 17, 18, 19, 20, 21, 30, 31, 32, 33, 34, 35, 43, 44, 45, 46, 47, 48,
        12, 11, 10, 9, 8, 7, 27, 26, 25, 24, 23, 22, 41, 40, 39, 38, 37, 36, 54, 53, 52, 51, 50,
        49, 56, 57, 58, 59, 60, 61, 65, 64, 63, 62, 55, 66, 13, 14, 15, 28, 42, 29,
    ];
    let physical = decode_raw_thresholds(&rows()).unwrap();
    for (flat, number) in matrix_physical_numbers.into_iter().enumerate() {
        assert_eq!(physical[number - 1], flat as u8);
    }
    let mut unique = physical;
    unique.sort();
    assert_eq!(unique, std::array::from_fn(|i| i as u8));
    assert_eq!(physical[30], 13, "A is physical key 31");
    assert_eq!(physical[31], 14, "S is physical key 32");
    assert_eq!(physical[41], 64, "Enter is physical key 42");
    assert_eq!(physical[65], 59, "Right is physical key 66");
}

#[test]
fn mode_one_accepts_every_byte_value_and_opaque_nonzero_tails() {
    for value in 0..=255 {
        let mut reports = rows();
        for report in &mut reports {
            report[4..10].fill(value);
        }
        assert_eq!(decode_raw_thresholds(&reports).unwrap(), [value; 66]);
    }
}

#[test]
fn missing_excess_duplicate_reordered_and_out_of_range_rows_fail_closed() {
    let correct = rows();
    for count in 0..11 {
        assert!(decode_raw_thresholds(&correct[..count]).is_err());
    }
    let mut excess = correct.clone();
    excess.push(correct[0]);
    assert!(decode_raw_thresholds(&excess).is_err());
    for replacement in [0, 1, 12, 255] {
        let mut wrong = correct.clone();
        wrong[1][2] = replacement;
        assert!(decode_raw_thresholds(&wrong).is_err());
    }
    let mut reordered = correct;
    reordered.swap(0, 1);
    assert!(decode_raw_thresholds(&reordered).is_err());
}

#[test]
fn incorrect_prefix_opcode_and_modes_cannot_be_decoded_as_mode_one() {
    for (offset, value) in [(0, 1), (1, 0xdf), (1, 0xf6), (3, 0), (3, 2), (3, 255)] {
        let mut wrong = rows();
        wrong[4][offset] = value;
        assert!(decode_raw_thresholds(&wrong).is_err());
    }
}

#[test]
fn incremental_decoder_never_publishes_a_partial_table_and_rejects_first_bad_row() {
    let mut decoder = RawThresholdDecoder::default();
    for report in rows().iter().take(10) {
        decoder.push(report).unwrap();
    }
    assert!(decoder.finish().is_err());
    let mut decoder = RawThresholdDecoder::default();
    assert!(decoder.push(&rows()[1]).is_err());
}

#[test]
fn ignored_incremental_parser_failure_cannot_be_repaired_into_a_snapshot() {
    let mut decoder = RawThresholdDecoder::default();
    assert!(decoder.push(&rows()[1]).is_err());
    for report in rows() {
        assert!(decoder.push(&report).is_err());
    }
    assert!(decoder.finish().is_err());

    let mut decoder = RawThresholdDecoder::default();
    for report in rows() {
        decoder.push(&report).unwrap();
    }
    assert!(decoder.push(&rows()[0]).is_err());
    assert!(decoder.finish().is_err());
}
