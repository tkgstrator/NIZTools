use super::*;
use eframe::App as _;
use egui::Color32;
use std::time::{Duration, Instant};

struct Harness {
    app: App,
    ctx: egui::Context,
    frame: eframe::Frame,
    size: egui::Vec2,
    system_theme: Option<egui::Theme>,
    focused: bool,
    native_pixels_per_point: f32,
    output: egui::FullOutput,
}

#[test]
fn expanded_device_information_uses_one_body_font_for_all_rows() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let mut h = Harness::loaded([960., 820.]);
        h.ctx.set_theme(theme);
        let before = h
            .app
            .controller
            .editor
            .as_ref()
            .unwrap()
            .draft
            .records
            .clone();
        let generation = h.app.controller.generation;
        h.click("デバイス情報");
        for _ in 0..8 {
            h.render(vec![]);
        }
        let firmware = h
            .app
            .controller
            .editor
            .as_ref()
            .unwrap()
            .baseline
            .firmware
            .clone();
        let expected = h.ctx.style_of(theme).text_styles[&egui::TextStyle::Body].clone();
        let mut rows = 0;
        for shape in &h.output.shapes {
            let egui::Shape::Text(text) = &shape.shape else {
                continue;
            };
            let label = text.galley.text();
            if label.starts_with("対象: ")
                || label.starts_with("識別情報: ")
                || label.starts_with("接続先: ")
                || label == "表示中の設定の読み込み元"
                || label == firmware
            {
                rows += 1;
                for section in &text.galley.job.sections {
                    assert_eq!(section.format.font_id, expected, "{theme:?}: {label}");
                }
            }
        }
        assert_eq!(
            rows, 8,
            "both target descriptions, source caption, and firmware must remain visible"
        );
        assert_eq!(
            h.app.controller.editor.as_ref().unwrap().draft.records,
            before
        );
        assert_eq!(h.app.controller.generation, generation);
        assert!(h.app.controller.pending.is_none());
    }
}
impl Harness {
    fn loaded(size: [f32; 2]) -> Self {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let app = App::new(&cc, true);
        let mut h = Self {
            app,
            ctx,
            frame: eframe::Frame::_new_kittest(),
            size: egui::vec2(size[0], size[1]),
            system_theme: Some(egui::Theme::Light),
            focused: true,
            native_pixels_per_point: 1.,
            output: Default::default(),
        };
        h.settle();
        h.app.read_selected();
        h.settle();
        assert!(h.app.controller.editor.is_some());
        assert!(h.app.controller.live);
        h
    }
    fn input(&self, events: Vec<egui::Event>) -> egui::RawInput {
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, self.size)),
            system_theme: self.system_theme,
            focused: self.focused,
            events,
            ..Default::default()
        };
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .native_pixels_per_point = Some(self.native_pixels_per_point);
        input
    }
    fn render(&mut self, events: Vec<egui::Event>) {
        let mut input = self.input(events);
        self.app.raw_input_hook(&self.ctx, &mut input);
        let _ = self
            .ctx
            .run_logic(&input, |ctx| self.app.logic(ctx, &mut self.frame));
        self.output = self
            .ctx
            .run_ui(input, |ui| self.app.ui(ui, &mut self.frame));
        // A headless frame has no texture renderer. Avoid retaining unapplied deltas.
        self.output.textures_delta.clear();
    }
    fn settle(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            self.render(vec![]);
            if self.app.controller.pending.is_none() {
                self.render(vec![]);
                return;
            }
            assert!(
                Instant::now() < deadline,
                "in-process demo worker did not settle"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    fn texts(&self) -> Vec<(String, egui::Rect, egui::Rect)> {
        self.output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) => Some((
                    t.galley.text().to_owned(),
                    egui::Rect::from_min_size(t.pos, t.galley.size()),
                    s.clip_rect,
                )),
                _ => None,
            })
            .collect()
    }
    fn rect(&self, label: &str) -> egui::Rect {
        self.texts()
            .into_iter()
            .find(|(t, _, _)| t == label)
            .unwrap_or_else(|| panic!("missing {label:?}"))
            .1
    }
    fn click(&mut self, label: &str) {
        let pos = self.rect(label).center();
        for pressed in [true, false] {
            self.render(vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ]);
        }
        self.render(vec![]);
    }
    fn visible(&self, label: &str) -> bool {
        self.texts()
            .into_iter()
            .any(|(t, rect, clip)| t == label && clip.contains_rect(rect))
    }
    fn scroll_down(&mut self) {
        self.render(vec![
            egui::Event::PointerMoved(egui::pos2(self.size.x - 30., self.size.y - 30.)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0., -220.),
                phase: egui::TouchPhase::Move,
                modifiers: Default::default(),
            },
        ]);
        for _ in 0..4 {
            self.render(vec![]);
        }
    }
    fn appearance(&mut self, current: &str, next: &str) {
        self.click(current);
        self.click(next);
        self.render(vec![]);
    }
}

#[derive(Clone, Copy, Debug)]
struct PaintedGeometry {
    text: egui::Rect,
    glyphs: egui::Rect,
    control: Option<egui::Rect>,
}
fn painted_geometry(h: &Harness) -> std::collections::BTreeMap<String, PaintedGeometry> {
    let mut geometry = std::collections::BTreeMap::new();
    for (index, shape) in h.output.shapes.iter().enumerate() {
        let egui::Shape::Text(t) = &shape.shape else {
            continue;
        };
        let text = t.galley.rect.translate(t.pos.to_vec2());
        let control = h.output.shapes[..index]
            .iter()
            .rev()
            .find_map(|s| match &s.shape {
                egui::Shape::Rect(r) if r.rect.contains(text.center()) && r.rect.height() < 50. => {
                    Some(r.rect)
                }
                _ => None,
            });
        geometry
            .entry(t.galley.text().to_owned())
            .or_insert(PaintedGeometry {
                text,
                glyphs: t.galley.mesh_bounds.translate(t.pos.to_vec2()),
                control,
            });
    }
    geometry
}
fn rect_delta(a: egui::Rect, b: egui::Rect) -> f32 {
    [
        (a.min.x - b.min.x).abs(),
        (a.min.y - b.min.y).abs(),
        (a.max.x - b.max.x).abs(),
        (a.max.y - b.max.y).abs(),
    ]
    .into_iter()
    .fold(0., f32::max)
}
fn assert_geometry_unchanged(
    h: &Harness,
    before: &std::collections::BTreeMap<String, PaintedGeometry>,
    context: &str,
) {
    let after = painted_geometry(h);
    for (label, old) in before {
        let new = after
            .get(label)
            .unwrap_or_else(|| panic!("{context}: missing {label:?}"));
        assert!(
            rect_delta(old.text, new.text) <= 0.01 && rect_delta(old.glyphs, new.glyphs) <= 0.01,
            "{context}: {label:?} moved from {old:?} to {new:?}"
        );
        if let Some(old_control) = old.control {
            let new_control = new
                .control
                .unwrap_or_else(|| panic!("{context}: missing control for {label:?}"));
            assert!(
                rect_delta(old_control, new_control) <= 0.01,
                "{context}: {label:?} control resized from {old_control:?} to {new_control:?}"
            );
        }
    }
}

#[test]
fn hover_and_press_keep_choice_glyphs_controls_and_neighbors_fixed() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        for size in [[960., 820.], [700., 580.]] {
            for scale in [1., 2.] {
                let mut h = Harness::loaded(size);
                h.native_pixels_per_point = scale;
                h.ctx.set_theme(theme);
                h.render(vec![]);
                assert_eq!(h.ctx.pixels_per_point(), scale);
                for hover in [
                    "2つの位置を交換",
                    "入力しきい値",
                    "入力テスト",
                    "右 Fn",
                    "左 Fn",
                    "出力を変更",
                    "再検索",
                    "システム",
                    "A\n#31",
                ] {
                    h.render(vec![egui::Event::PointerGone]);
                    for _ in 0..60 {
                        h.render(vec![]);
                    }
                    let before = painted_geometry(&h);
                    let pos = h.rect(hover).center();
                    let e = h.app.controller.editor.as_ref().unwrap();
                    let editor_before = (
                        e.draft.records.clone(),
                        e.baseline.records.clone(),
                        e.selection,
                        e.output_ready(),
                        e.can_undo(),
                        e.can_redo(),
                    );
                    let generation = h.app.controller.generation;
                    let target = h.app.controller.attached_target.clone();
                    for frame in 0..120 {
                        h.render(if frame == 0 {
                            vec![egui::Event::PointerMoved(pos)]
                        } else {
                            vec![]
                        });
                        assert_geometry_unchanged(
                            &h,
                            &before,
                            &format!("{theme:?} {size:?} hover={hover:?} frame={frame}"),
                        );
                    }
                    if matches!(
                        hover,
                        "2つの位置を交換"
                            | "入力しきい値"
                            | "入力テスト"
                            | "右 Fn"
                            | "左 Fn"
                            | "出力を変更"
                    ) {
                        h.render(vec![egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            modifiers: Default::default(),
                        }]);
                        assert_geometry_unchanged(&h, &before, "choice pointer down");
                        let outside = egui::pos2(1., 1.);
                        h.render(vec![
                            egui::Event::PointerMoved(outside),
                            egui::Event::PointerButton {
                                pos: outside,
                                button: egui::PointerButton::Primary,
                                pressed: false,
                                modifiers: Default::default(),
                            },
                        ]);
                    }
                    h.render(vec![egui::Event::PointerGone]);
                    for _ in 0..3 {
                        h.render(vec![]);
                        assert_geometry_unchanged(&h, &before, "choice hover exit");
                    }
                    let e = h.app.controller.editor.as_ref().unwrap();
                    assert_eq!(
                        (
                            e.draft.records.clone(),
                            e.baseline.records.clone(),
                            e.selection,
                            e.output_ready(),
                            e.can_undo(),
                            e.can_redo()
                        ),
                        editor_before
                    );
                    assert_eq!(h.app.controller.generation, generation);
                    assert_eq!(h.app.controller.attached_target, target);
                    assert!(h.app.controller.pending.is_none());
                    assert!(h.app.view == View::Assignment);
                }
                h.click("システム");
                h.render(vec![egui::Event::PointerGone]);
                for _ in 0..60 {
                    h.render(vec![]);
                }
                let before = painted_geometry(&h);
                let pos = h.rect("ライト").center();
                for frame in 0..30 {
                    h.render(if frame == 0 {
                        vec![egui::Event::PointerMoved(pos)]
                    } else {
                        vec![]
                    });
                    assert_geometry_unchanged(&h, &before, "appearance menu hover");
                }
                println!(
                    "theme={theme:?} size={size:?} scale={scale} hover_and_press_geometry_delta=0.00"
                );
            }
        }
    }
}

#[test]
fn choice_value_preserves_changed_click_disabled_and_keyboard_semantics() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        ctx.set_theme(theme);
        let mut value = false;
        let mut render = |events, enabled, focus| {
            let mut response = None;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    ui.add_enabled_ui(enabled, |ui| {
                        let r = theme::choice_value(ui, &mut value, true, "choice");
                        if focus {
                            r.request_focus();
                        }
                        response = Some(r);
                    });
                },
            );
            output.textures_delta.clear();
            (response.unwrap(), value)
        };
        let (r, _) = render(vec![], true, false);
        let pos = r.rect.center();
        for _ in 0..3 {
            let (r, v) = render(vec![egui::Event::PointerMoved(pos)], true, false);
            assert!(!r.clicked() && !r.changed() && !v);
        }
        let pointer = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let (r, _) = render(vec![pointer(true)], true, false);
        assert!(!r.clicked() && !r.changed());
        let (r, v) = render(vec![pointer(false)], true, false);
        assert!(r.clicked() && r.changed() && v);
        render(vec![pointer(true)], true, false);
        let (r, v) = render(vec![pointer(false)], true, false);
        assert!(r.clicked() && !r.changed() && v);
        render(vec![pointer(true)], false, false);
        let (r, v) = render(vec![pointer(false)], false, false);
        assert!(!r.clicked() && !r.changed() && v);
        render(vec![egui::Event::PointerGone], true, true);
        let (r, v) = render(vec![key_event(egui::Key::Enter, true)], true, false);
        assert!(r.clicked() && !r.changed() && v, "keyboard activation lost");
    }
}

#[test]
fn default_collapsed_views_have_no_wheel_scroll_range() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        for state in 0..11 {
            let mut h = Harness::loaded([960., 820.]);
            h.ctx.set_theme(theme);
            if state == 3 {
                h.app.view = View::Calibration;
            }
            if state == 4 {
                h.app.view = View::InputTest;
            }
            if state == 1 {
                let e = h.app.controller.editor.as_mut().unwrap();
                e.select_from(30);
                e.choose_output(44);
                e.edit().unwrap();
            }
            if state == 2 {
                let e = h.app.controller.editor.as_mut().unwrap();
                e.selection.mode = Mode::Swap;
                e.select_from(30);
                e.selection.to_position = 31;
                e.edit().unwrap();
            }
            if state >= 5 {
                let phase = [
                    CalibrationPhase::AwaitingHeldKeys,
                    CalibrationPhase::AwaitingFinish,
                    CalibrationPhase::Complete,
                    CalibrationPhase::Aborted,
                    CalibrationPhase::ReleaseWaiting,
                    CalibrationPhase::HeldWaiting,
                ][state - 5];
                h.app.view = View::Calibration;
                h.app.controller.live = false;
                h.app.controller.calibration = Some(niz_device::CalibrationStatus {
                    phase,
                    failure_phase: None,
                    backup: None,
                    lock_may_have_been_sent: true,
                    release_attempted: true,
                    release_sent: true,
                    release_completed: true,
                    held_attempted: false,
                    held_sent: false,
                    held_completed: false,
                    cancellation_requested: false,
                    unlock: UnlockOutcome::NotAttempted,
                    settings_verified: false,
                    lighting_verified: false,
                    original_error: None,
                    trace: vec![],
                    held_key: None,
                });
                if state >= 9 {
                    h.app.controller.pending = Some(crate::protocol::Ticket {
                        generation: 0,
                        operation: 999,
                    });
                }
            }
            h.render(vec![]);
            let before = h.rect("NIZ Utility");
            assert!(
                h.visible("操作の記録・詳細"),
                "theme={theme:?} state={state}: closed diagnostics clipped"
            );
            if state >= 5 {
                let keys = h
                    .texts()
                    .into_iter()
                    .filter(|(text, rect, clip)| text.contains("\n#") && clip.contains_rect(*rect))
                    .count();
                assert_eq!(keys, 66);
                match state {
                    5 | 6 => {
                        assert!(
                            h.visible("進行中の校正 · 全キーに作用") && h.visible("中止を要求")
                        );
                    }
                    7 => assert_eq!(
                        h.app.controller.calibration.as_ref().unwrap().phase,
                        CalibrationPhase::Complete
                    ),
                    8 => assert!(
                        h.texts()
                            .iter()
                            .any(|(text, rect, clip)| text.contains("校正に失敗しました")
                                || text.contains("中止しました") && clip.contains_rect(*rect))
                    ),
                    _ => assert!(h.visible("中止を要求")),
                }
            }
            h.scroll_down();
            assert_eq!(
                h.rect("NIZ Utility"),
                before,
                "theme={theme:?} state={state} has unintended default overflow"
            );
            println!("theme={theme:?} default_state={state} wheel_scroll_range=0.00");
        }
    }
}

#[test]
fn mixed_header_and_board_legend_rows_center_actual_galleys() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        for size in [[960., 820.], [700., 580.]] {
            let mut h = Harness::loaded(size);
            h.ctx.set_theme(theme);
            h.render(vec![]);
            for labels in [
                vec!["NIZ Utility", "システム", "外観"],
                vec!["Simulated Atom66", "再検索", "読み込む"],
                vec![
                    "DEMO · 読み込み済み",
                    "実際のキーボードには接続しません",
                    "デバイス情報",
                ],
                vec!["出力を変更", "2つの位置を交換", "通常", "右 Fn", "左 Fn"],
                vec!["現在と同じ割り当てです", "変更を追加"],
                vec![
                    "未適用の変更なし",
                    "元に戻す",
                    "やり直す",
                    "デモで適用を確認…",
                ],
                vec![
                    "編集中の割り当て",
                    "■ From",
                    "● 未適用",
                    "キーをクリックして選択",
                ],
            ] {
                if !h.texts().iter().any(|(text, _, _)| text == labels[0]) {
                    for _ in 0..8 {
                        h.scroll_down();
                        if h.texts().iter().any(|(text, _, _)| text == labels[0]) {
                            break;
                        }
                    }
                }
                let centers: Vec<_> = labels.iter().map(|l| h.rect(l).center().y).collect();
                let range = centers.iter().copied().fold(f32::NEG_INFINITY, f32::max)
                    - centers.iter().copied().fold(f32::INFINITY, f32::min);
                assert!(range <= 1., "{labels:?} centers={centers:?}");
                println!("theme={theme:?} size={size:?} row={labels:?} center_delta={range:.2}");
            }
        }
    }
}

#[test]
fn focused_input_test_ignores_text_and_releases_the_held_output() {
    let mut h = Harness::loaded([960., 820.]);
    h.click("入力テスト");
    h.render(vec![
        key_event(egui::Key::A, true),
        egui::Event::Text("a".into()),
    ]);
    assert_eq!(
        h.app.input_test.codes().0,
        std::collections::BTreeSet::from([43])
    );
    assert!(!h.visible("a"));
    h.render(vec![key_event(egui::Key::A, false)]);
    assert!(h.app.input_test.codes().0.is_empty());
    assert!(!h.app.controller.editor.as_ref().unwrap().dirty());
    assert!(h.app.controller.pending.is_none());
}
fn key_event(key: egui::Key, pressed: bool) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed,
        repeat: false,
        modifiers: Default::default(),
    }
}

#[test]
fn input_test_consumes_activation_shortcuts_without_editor_or_worker_side_effects() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let mut h = Harness::loaded([960., 820.]);
        h.ctx.set_theme(theme);
        h.click("A\n#31");
        let e = h.app.controller.editor.as_mut().unwrap();
        e.choose_output(44);
        e.edit().unwrap();
        let before = (
            e.draft.records.clone(),
            e.baseline.records.clone(),
            e.selection,
            e.output_ready(),
            e.can_undo(),
            e.can_redo(),
        );
        let generation = h.app.controller.generation;
        let target = h.app.controller.attached_target.clone();
        h.click("入力テスト");
        h.render(vec![]);
        let unheld_fill = label_fill(&h, "A\n#31");
        // Focus an interactive mouse-exit tab first: keyboard testing must not activate it.
        let focus = h.ctx.memory(|m| m.focused());
        for key in [
            egui::Key::Tab,
            egui::Key::ArrowLeft,
            egui::Key::ArrowRight,
            egui::Key::ArrowDown,
            egui::Key::Enter,
            egui::Key::Space,
            egui::Key::Escape,
        ] {
            h.render(vec![key_event(key, true)]);
            h.render(vec![key_event(key, false)]);
            assert!(matches!(h.app.view, View::InputTest));
            assert_eq!(h.ctx.memory(|m| m.focused()), focus);
        }
        h.render(vec![
            egui::Event::Copy,
            egui::Event::Cut,
            egui::Event::Paste("private clipboard".into()),
            egui::Event::Ime(egui::ImeEvent::Commit("private ime".into())),
        ]);
        assert!(h.app.input_test.codes().0.is_empty());
        let e = h.app.controller.editor.as_ref().unwrap();
        assert_eq!(
            (
                e.draft.records.clone(),
                e.baseline.records.clone(),
                e.selection,
                e.output_ready(),
                e.can_undo(),
                e.can_redo()
            ),
            before
        );
        assert_eq!(h.app.controller.generation, generation);
        assert_eq!(h.app.controller.attached_target, target);
        assert!(h.app.controller.pending.is_none());
        assert!(h.app.confirm.is_none());
        h.render(vec![key_event(egui::Key::A, true)]);
        assert_eq!(
            h.app
                .input_test
                .matches(&h.app.controller.editor.as_ref().unwrap().baseline),
            std::collections::BTreeSet::from([30])
        );
        assert_text_contrast(&h, "A\n#31", 4.5);
        assert_ne!(
            label_fill(&h, "A\n#31"),
            unheld_fill,
            "held candidate must visibly change"
        );
        h.render(vec![key_event(egui::Key::A, false)]);
        assert_eq!(label_fill(&h, "A\n#31"), unheld_fill);
        h.click("キーの割り当て");
        assert!(h.app.input_test.codes().0.is_empty());
    }
}

#[test]
fn input_test_clears_on_focus_loss_pending_confirmation_and_fresh_reference() {
    let mut h = Harness::loaded([960., 820.]);
    h.click("入力テスト");
    h.render(vec![
        key_event(egui::Key::A, true),
        egui::Event::Text("入力🙂".repeat(120)),
    ]);
    assert!(!h.visible("入力🙂"));
    h.render(vec![egui::Event::WindowFocused(false)]);
    assert!(h.app.input_test.codes().0.is_empty());
    h.focused = false;
    h.render(vec![key_event(egui::Key::S, true)]);
    assert!(h.app.input_test.codes().0.is_empty());
    h.focused = true;
    h.render(vec![]);
    assert!(h.app.input_test.codes().0.is_empty());
    h.render(vec![
        egui::Event::WindowFocused(true),
        key_event(egui::Key::A, true),
    ]);
    assert!(!h.app.input_test.codes().0.is_empty());
    h.app.controller.pending = Some(crate::protocol::Ticket {
        generation: 0,
        operation: 999,
    });
    h.render(vec![
        key_event(egui::Key::S, true),
        egui::Event::Text("paused input".into()),
    ]);
    assert!(h.app.input_test.codes().0.is_empty());
    assert!(!h.visible("paused input"));
    assert!(
        h.texts()
            .iter()
            .any(|(text, _, _)| text.contains("入力テストを停止")),
        "busy input test must visibly explain capture is paused"
    );
    h.app.controller.pending = None;
    h.render(vec![key_event(egui::Key::A, true)]);
    h.app.confirm = Some(Confirm::Load(
        h.app.controller.editor.as_ref().unwrap().baseline.clone(),
    ));
    h.render(vec![key_event(egui::Key::S, true)]);
    assert!(h.app.input_test.codes().0.is_empty());
    h.app.confirm = None;
    h.render(vec![key_event(egui::Key::A, true)]);
    h.app.controller.editor.as_mut().unwrap().baseline.records[30][0][6] = 44;
    h.render(vec![]);
    assert!(h.app.input_test.codes().0.is_empty());
}

// These catch layout growth that puts a complete row or the apply action below the fold.
#[test]
fn default_assignment_frame_shows_every_key_and_primary_actions() {
    assert_default_frame(Mode::Reassign);
}
#[test]
fn default_swap_frame_shows_every_key_and_primary_actions() {
    assert_default_frame(Mode::Swap);
}
fn assert_default_frame(mode: Mode) {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        assert_default_frame_in_theme(mode, theme);
    }
}
fn assert_default_frame_in_theme(mode: Mode, theme: egui::Theme) {
    let mut h = Harness::loaded([960., 820.]);
    h.ctx.set_theme(theme);
    h.app
        .keyboard
        .set_mode(h.app.controller.editor.as_mut().unwrap(), mode);
    h.render(vec![]);
    let texts = h.texts();
    let keys: Vec<_> = texts
        .iter()
        .filter(|(t, _, _)| t.contains('\n') && t.contains('#'))
        .collect();
    assert_eq!(keys.len(), 66);
    for (label, rect, clip) in keys {
        assert!(
            clip.contains_rect(*rect),
            "key {label:?} clipped: {rect:?} / {clip:?}"
        );
    }
    assert!(h.visible("変更を追加"));
    assert!(h.visible("デモで適用を確認…"));
    assert!(h.visible("元に戻す"));
}

// A confirmation must freeze navigation behind the dialog, not just board clicks.
#[test]
fn confirmation_does_not_allow_background_tab_switch() {
    let mut h = Harness::loaded([960., 820.]);
    h.app.confirm = Some(Confirm::Load(
        h.app.controller.editor.as_ref().unwrap().draft.clone(),
    ));
    h.render(vec![]);
    h.click("入力しきい値");
    assert!(matches!(h.app.view, View::Assignment));
    assert!(h.app.confirm.is_some());
}

// A running procedure must retain visible stage and cancellation when a tab is clicked.
#[test]
fn active_calibration_keeps_its_stage_visible_when_assignment_tab_is_clicked() {
    let mut h = Harness::loaded([960., 820.]);
    h.app.view = View::Calibration;
    h.app.controller.calibration = Some(niz_device::CalibrationStatus {
        phase: CalibrationPhase::AwaitingHeldKeys,
        failure_phase: None,
        backup: None,
        lock_may_have_been_sent: true,
        release_attempted: true,
        release_sent: true,
        release_completed: true,
        held_attempted: false,
        held_sent: false,
        held_completed: false,
        cancellation_requested: false,
        unlock: UnlockOutcome::NotAttempted,
        settings_verified: false,
        lighting_verified: false,
        original_error: None,
        trace: vec![],
        held_key: None,
    });
    h.render(vec![]);
    h.click("キーの割り当て");
    assert!(matches!(h.app.view, View::Calibration));
    assert!(h.visible("中止を要求"));
}

#[test]
fn minimum_frame_scroll_reaches_every_key_apply_and_files_without_horizontal_clipping() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        assert_minimum_frame(theme);
    }
}
fn assert_minimum_frame(theme: egui::Theme) {
    let mut h = Harness::loaded([700., 580.]);
    h.ctx.set_theme(theme);
    h.render(vec![]);
    let mut slots = std::collections::BTreeSet::new();
    let mut apply = false;
    let mut files = false;
    for _ in 0..8 {
        for (text, rect, clip) in h.texts() {
            if !clip.contains_rect(rect) {
                continue;
            }
            if let Some((_, number)) = text.split_once("\n#") {
                assert!(
                    rect.left() >= 0. && rect.right() <= 700.,
                    "horizontal overflow: {text:?} / {rect:?}"
                );
                slots.insert(number.parse::<usize>().unwrap());
            }
            apply |= text == "デモで適用を確認…";
            files |= text == "ファイル・バックアップ";
        }
        h.scroll_down();
    }
    assert_eq!(slots, (1..=66).collect());
    assert!(apply && files, "footer and files must remain reachable");
}

#[test]
fn minimum_frame_keeps_long_device_details_inside_the_viewport() {
    let mut h = Harness::loaded([700., 580.]);
    let product = "日本語の長いキーボード名称".repeat(30);
    h.app.controller.devices[0].product = product.clone();
    let target = h.app.controller.attached_target.as_mut().unwrap();
    target.product = product;
    target.path = format!("/device/{}", "long-path-".repeat(100));
    h.render(vec![]);
    h.click("デバイス情報");
    h.render(vec![]);
    assert!(h.visible("読み込む"));
    for (text, rect, clip) in h.texts() {
        if rect.top() >= clip.top() && rect.bottom() <= clip.bottom() {
            assert!(
                rect.left() >= 0. && rect.right() <= 700.,
                "visible label overflow: {text:?} / {rect:?}"
            );
        }
    }
}

#[test]
fn board_click_opens_current_output_without_creating_a_draft() {
    let mut h = Harness::loaded([960., 820.]);
    h.click("A\n#31");
    let editor = h.app.controller.editor.as_ref().unwrap();
    assert_eq!(editor.selection.from, 30);
    assert_eq!(editor.selection.to_code, 43);
    assert!(!editor.dirty());
    assert!(!editor.can_undo());
}

#[test]
fn swap_clicks_add_two_changes_only_after_add_and_undo_restores_them() {
    let mut h = Harness::loaded([960., 820.]);
    h.click("2つの位置を交換");
    h.click("A\n#31");
    h.click("S\n#32");
    let editor = h.app.controller.editor.as_ref().unwrap();
    assert_eq!(
        (editor.selection.from, editor.selection.to_position),
        (30, 31)
    );
    assert!(!editor.dirty());
    h.click("変更を追加");
    assert_eq!(
        h.app.controller.editor.as_ref().unwrap().preview().unwrap(),
        [30, 31]
    );
    h.click("元に戻す");
    assert!(!h.app.controller.editor.as_ref().unwrap().dirty());
    assert!(h.app.controller.editor.as_ref().unwrap().can_redo());
}

#[test]
fn open_output_popup_and_its_closing_click_do_not_select_an_underlying_key() {
    let mut h = Harness::loaded([960., 820.]);
    h.click("A\n#31");
    h.click("A");
    assert!(egui::Popup::is_any_open(&h.ctx));
    h.click("Space\n#60");
    assert_eq!(h.app.controller.editor.as_ref().unwrap().selection.from, 30);
    h.click("Space\n#60");
    assert_eq!(h.app.controller.editor.as_ref().unwrap().selection.from, 59);
    assert!(!h.app.controller.editor.as_ref().unwrap().dirty());
}

#[test]
fn busy_operation_disables_board_navigation() {
    let mut h = Harness::loaded([960., 820.]);
    h.app.controller.pending = Some(crate::protocol::Ticket {
        generation: 0,
        operation: 999,
    });
    h.render(vec![]);
    let before = h.app.controller.editor.as_ref().unwrap().selection;
    h.click("A\n#31");
    assert_eq!(h.app.controller.editor.as_ref().unwrap().selection, before);
}

// These fail if System is sampled only once or a user override is discarded.
#[test]
fn appearance_defaults_to_system_and_tracks_native_theme_changes() {
    let mut h = Harness::loaded([960., 820.]);
    assert_eq!(
        h.ctx.options(|o| o.theme_preference),
        egui::ThemePreference::System
    );
    assert_eq!(h.ctx.theme(), egui::Theme::Light);
    h.system_theme = Some(egui::Theme::Dark);
    h.render(vec![]);
    assert_eq!(h.ctx.theme(), egui::Theme::Dark);
    h.system_theme = Some(egui::Theme::Light);
    h.render(vec![]);
    assert_eq!(h.ctx.theme(), egui::Theme::Light);
}

#[test]
fn appearance_menu_switches_explicit_themes_and_resumes_system_without_editor_side_effects() {
    let mut h = Harness::loaded([960., 820.]);
    h.click("A\n#31");
    let editor = h.app.controller.editor.as_mut().unwrap();
    editor.choose_output(44);
    editor.edit().unwrap();
    let before = (
        editor.draft.records.clone(),
        editor.selection,
        editor.output_ready(),
        editor.can_undo(),
        editor.can_redo(),
    );
    let generation = h.app.controller.generation;
    let target = h.app.controller.attached_target.clone();
    h.render(vec![]);
    h.appearance("システム", "ダーク");
    assert_eq!(h.ctx.theme(), egui::Theme::Dark);
    h.system_theme = Some(egui::Theme::Light);
    h.render(vec![]);
    assert_eq!(h.ctx.theme(), egui::Theme::Dark);
    h.appearance("ダーク", "ライト");
    h.system_theme = Some(egui::Theme::Dark);
    h.render(vec![]);
    assert_eq!(h.ctx.theme(), egui::Theme::Light);
    h.appearance("ライト", "システム");
    assert_eq!(h.ctx.theme(), egui::Theme::Dark);
    let editor = h.app.controller.editor.as_ref().unwrap();
    assert_eq!(
        (
            editor.draft.records.clone(),
            editor.selection,
            editor.output_ready(),
            editor.can_undo(),
            editor.can_redo()
        ),
        before
    );
    assert_eq!(h.app.controller.generation, generation);
    assert_eq!(h.app.controller.attached_target, target);
    assert!(h.app.controller.pending.is_none());
}

#[test]
fn strong_card_labels_have_readable_rendered_foreground() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let mut h = Harness::loaded([960., 820.]);
        h.ctx.set_theme(theme);
        h.render(vec![]);
        assert_text_contrast(&h, "From · 変更する位置", 4.5);
        assert_text_contrast(&h, "編集中の割り当て", 4.5);
        assert_text_contrast(&h, "キーの割り当て", 4.5);
        assert_text_contrast(&h, "通常", 4.5);
        assert_text_contrast(&h, "● 未適用", 4.5);
        assert_text_contrast(&h, "A\n#31", 4.5);
        assert_text_contrast(&h, "元に戻す", 3.);
        assert_text_contrast(&h, "変更を追加", 3.);
        h.click("2つの位置を交換");
        h.click("A\n#31");
        h.click("S\n#32");
        assert_text_contrast(&h, "■ From", 4.5);
        assert_text_contrast(&h, "■ To", 4.5);
        assert_text_contrast(&h, "A\n#31", 4.5);
        assert_text_contrast(&h, "S\n#32", 4.5);
        assert_text_contrast(&h, "変更を追加", 4.5);
    }
}

#[test]
fn appearance_popup_closing_click_preserves_board_selection() {
    let mut h = Harness::loaded([960., 820.]);
    let before = h.app.controller.editor.as_ref().unwrap().selection;
    h.click("システム");
    assert!(egui::Popup::is_any_open(&h.ctx));
    h.click("A\n#31");
    assert_eq!(h.app.controller.editor.as_ref().unwrap().selection, before);
    assert!(!egui::Popup::is_any_open(&h.ctx));
    h.click("A\n#31");
    assert_eq!(h.app.controller.editor.as_ref().unwrap().selection.from, 30);
    assert!(!h.app.controller.editor.as_ref().unwrap().dirty());
}

#[test]
fn confirmation_blocks_the_appearance_menu_without_losing_the_confirmation() {
    let mut h = Harness::loaded([960., 820.]);
    h.app.confirm = Some(Confirm::Load(
        h.app.controller.editor.as_ref().unwrap().draft.clone(),
    ));
    h.render(vec![]);
    h.click("システム");
    assert!(!egui::Popup::is_any_open(&h.ctx));
    assert_eq!(
        h.ctx.options(|o| o.theme_preference),
        egui::ThemePreference::System
    );
    assert!(h.app.confirm.is_some());
    assert!(h.visible("キャンセル"));
    // Assess the settled modal, after the framework's entrance fade completes.
    for _ in 0..12 {
        h.render(vec![]);
    }
    assert_text_contrast(&h, "内容を確認", 4.5);
}

#[test]
fn appearance_switch_preserves_active_calibration_phase_and_cancel_visibility() {
    let mut h = Harness::loaded([960., 820.]);
    h.app.view = View::Calibration;
    h.app.controller.calibration = Some(niz_device::CalibrationStatus {
        phase: CalibrationPhase::ReleaseWaiting,
        failure_phase: None,
        backup: None,
        lock_may_have_been_sent: true,
        release_attempted: true,
        release_sent: true,
        release_completed: false,
        held_attempted: false,
        held_sent: false,
        held_completed: false,
        cancellation_requested: false,
        unlock: UnlockOutcome::NotAttempted,
        settings_verified: false,
        lighting_verified: false,
        original_error: None,
        trace: vec![],
        held_key: None,
    });
    let generation = h.app.controller.generation;
    h.render(vec![]);
    h.appearance("システム", "ダーク");
    assert_eq!(h.ctx.theme(), egui::Theme::Dark);
    assert_eq!(
        h.app.controller.calibration.as_ref().unwrap().phase,
        CalibrationPhase::ReleaseWaiting
    );
    assert!(matches!(h.app.view, View::Calibration));
    assert!(h.visible("中止を要求"));
    assert_eq!(h.app.controller.generation, generation);
    assert!(h.app.controller.pending.is_none());
    assert!(
        !h.app
            .controller
            .cancel
            .load(std::sync::atomic::Ordering::Acquire)
    );
}

#[test]
fn semantic_palette_roles_meet_text_and_state_contrast_in_both_themes() {
    for dark in [false, true] {
        let p = Palette::for_dark(dark);
        for (role, foreground, surfaces) in [
            (
                "ordinary/strong",
                p.text,
                vec![p.root, p.card, p.control, p.hover, p.pressed, p.input],
            ),
            (
                "secondary",
                p.muted,
                vec![p.root, p.card, p.control, p.input],
            ),
            ("warning", p.warning, vec![p.root, p.card]),
            ("loaded/link", p.accent_text, vec![p.root, p.card]),
            ("selected/action", p.on_accent, vec![p.accent]),
            ("From key", p.from_text, vec![p.from_fill]),
            ("To key", p.to_text, vec![p.to_fill]),
            ("input match", p.input_text, vec![p.input_fill]),
            ("destructive action", p.on_danger, vec![p.danger]),
            ("From legend", p.from_border, vec![p.card]),
            ("To legend", p.to_border, vec![p.card]),
            ("dirty legend", p.dirty, vec![p.card]),
        ] {
            for surface in surfaces {
                let ratio = contrast(foreground, surface);
                assert!(ratio >= 4.5, "dark={dark} role={role} contrast={ratio:.2}");
            }
        }
        for (role, color, surfaces) in [
            ("From state", p.from_border, vec![p.from_fill, p.card]),
            ("To state", p.to_border, vec![p.to_fill, p.card]),
            ("destructive surface", p.danger, vec![p.card, p.root]),
            (
                "input match state",
                p.input_border,
                vec![p.input_fill, p.card],
            ),
            (
                "dirty marker",
                p.dirty,
                vec![p.control, p.from_fill, p.to_fill],
            ),
            (
                "control outline",
                p.control_border,
                vec![p.control, p.input, p.hover, p.pressed],
            ),
        ] {
            for surface in surfaces {
                let ratio = contrast(color, surface);
                assert!(ratio >= 3., "dark={dark} role={role} contrast={ratio:.2}");
            }
        }
    }
}
fn label_fill(h: &Harness, label: &str) -> Color32 {
    let center = h.rect(label).center();
    h.output
        .shapes
        .iter()
        .rev()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if rect.rect.contains(center) && rect.fill.a() == 255 => {
                Some(rect.fill)
            }
            _ => None,
        })
        .expect("opaque input key surface")
}

#[test]
fn apply_confirmation_execution_and_cancel_share_a_readable_danger_row() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        for demo_label in [true, false] {
            // Both label variants use the explicitly in-process demo worker.
            let mut h = Harness::loaded([960., 820.]);
            h.ctx.set_theme(theme);
            h.app.demo = demo_label;
            let e = h.app.controller.editor.as_mut().unwrap();
            e.select_from(30);
            e.choose_output(44);
            e.edit().unwrap();
            let before = e.draft.records.clone();
            let generation = h.app.controller.generation;
            let approval = h
                .app
                .controller
                .prepare_trial(h.app.backup.clone())
                .unwrap();
            h.app.confirm = Some(Confirm::Apply(Box::new(approval)));
            for _ in 0..12 {
                h.render(vec![]);
            }
            let execute = if demo_label {
                "デモで実行"
            } else {
                "確認した対象に適用"
            };
            assert!((h.rect(execute).center().y - h.rect("キャンセル").center().y).abs() <= 1.);
            let color = label_fill(&h, execute).to_srgba_unmultiplied();
            assert!(
                color[0] > color[1] && color[0] > color[2],
                "execution must have a red danger surface"
            );
            assert_text_contrast(&h, execute, 4.5);
            h.click("キャンセル");
            assert!(h.app.confirm.is_none());
            assert!(h.app.controller.pending.is_none());
            assert_eq!(h.app.controller.generation, generation);
            assert_eq!(
                h.app.controller.editor.as_ref().unwrap().draft.records,
                before
            );
            assert!(h.app.controller.editor.as_ref().unwrap().can_undo());
        }
    }
}

#[test]
fn compact_threshold_values_share_vertical_center_and_manual_snapshot() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let mut h = Harness::loaded([960., 820.]);
        h.ctx.set_theme(theme);
        h.click("入力しきい値");
        assert!(h.visible("FROM: —"));
        assert!(h.visible("TO: —"));
        assert!(h.visible("生値: —"));
        assert!(!h.visible("しきい値について"));
        h.click("しきい値を読み込む");
        h.settle();
        let read_at = h.app.controller.raw_thresholds().unwrap().read_at;
        let generation = h.app.controller.generation;
        assert!(h.visible("FROM: 13"));
        assert!(h.visible("TO: —"));
        assert!(h.visible("生値: 13 (0x0D)"));
        assert!((h.rect("FROM: 13").center().y - h.rect("TO: —").center().y).abs() <= 1.);
        h.click("S\n#32");
        assert!(h.visible("FROM: 14"));
        assert!(h.visible("生値: 14 (0x0E)"));
        assert_eq!(h.app.controller.raw_thresholds().unwrap().read_at, read_at);
        assert_eq!(h.app.controller.generation, generation);
        assert!(h.app.controller.pending.is_none());
        assert!(!h.app.controller.editor.as_ref().unwrap().dirty());
    }
}

#[test]
fn raw_threshold_read_is_manual_and_demo_values_are_explicitly_simulated() {
    let mut h = Harness::loaded([960., 820.]);
    h.click("入力しきい値");
    assert!(h.visible("FROM: —"));
    assert!(h.visible("しきい値を読み込む"));
    assert!(h.visible("すべてのキーを離して準備"));
    h.click("S\n#32");
    assert!(h.visible("FROM: —"));
    h.click("しきい値を読み込む");
    h.settle();
    assert!(h.visible("FROM: 14"));
    assert!(h.visible("DEMO値"));
    h.click("A\n#31");
    assert!(h.visible("FROM: 13"));
    h.click("=\n#13");
    assert!(h.visible("FROM: 60"));
    assert!(h.visible("生値: 60 (0x3C)"));
    assert!(h.visible("すべてのキーを離して準備"));
    h.click("生しきい値の読み取り記録");
    for _ in 0..8 {
        h.render(vec![]);
    }
    assert!(
        h.texts()
            .iter()
            .any(|(text, _, _)| text.starts_with("読み取り時刻:"))
    );
    assert!(
        h.texts()
            .iter()
            .any(|(text, _, _)| text == "対象: Simulated Atom66 · demo://keyboard")
    );
    assert!(!h.app.controller.editor.as_ref().unwrap().dirty());
}

#[test]
fn raw_threshold_pending_and_error_show_no_old_or_partial_values() {
    let mut h = Harness::loaded([960., 820.]);
    h.click("入力しきい値");
    h.click("しきい値を読み込む");
    h.settle();
    assert!(h.visible("FROM: 13"));
    h.app.controller.read_raw_thresholds().unwrap();
    // Render the owned pending state before consuming the asynchronous demo reply.
    h.output = h
        .ctx
        .run_ui(h.input(vec![]), |ui| h.app.ui(ui, &mut h.frame));
    h.output.textures_delta.clear();
    assert!(h.visible("FROM: …"));
    assert!(h.visible("生値: …"));
    assert!(!h.visible("生値: 13 (0x0D)"));
    assert!(!h.visible("FROM: 13"));
    h.app.controller.process(crate::protocol::Event {
        ticket: h.app.controller.pending.unwrap(),
        response: crate::protocol::Response::Error("raw snapshot test error".to_string().into()),
    });
    h.render(vec![]);
    assert!(h.visible("FROM: —"));
    assert!(h.visible("生値: —"));
    assert!(h.app.controller.raw_thresholds().is_none());
    assert!(!h.app.controller.live);
    assert!(
        h.texts()
            .iter()
            .any(|(text, _, _)| text.contains("raw snapshot test error"))
    );
}

#[test]
fn raw_threshold_read_button_hover_keeps_geometry_fixed_in_both_themes() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        for size in [[960., 820.], [700., 580.]] {
            let mut h = Harness::loaded(size);
            h.ctx.set_theme(theme);
            h.click("入力しきい値");
            h.render(vec![egui::Event::PointerGone]);
            let before = painted_geometry(&h);
            let pos = h.rect("しきい値を読み込む").center();
            h.render(vec![egui::Event::PointerMoved(pos)]);
            for _ in 0..6 {
                h.render(vec![]);
                assert_geometry_unchanged(&h, &before, "raw read button hover");
            }
            assert!(h.app.controller.raw_thresholds().is_none());
            assert!(h.app.controller.pending.is_none());
        }
    }
}

#[test]
fn raw_threshold_read_button_respects_source_modal_popup_and_close_guards() {
    for case in 0..7 {
        let mut h = Harness::loaded([960., 820.]);
        h.click("入力しきい値");
        match case {
            0 => h.app.controller.live = false,
            1 => h.app.controller.editor.as_mut().unwrap().baseline.firmware = "wrong".into(),
            2 => {
                h.app.controller.pending = Some(crate::protocol::Ticket {
                    generation: 0,
                    operation: 999,
                })
            }
            3 => h.app.controller.close_requested = true,
            4 => {
                h.app.confirm = Some(Confirm::Load(
                    h.app.controller.editor.as_ref().unwrap().baseline.clone(),
                ))
            }
            5 => {
                h.app
                    .controller
                    .editor
                    .as_mut()
                    .unwrap()
                    .baseline
                    .identity
                    .product_id = 0x512a
            }
            _ => h.click("システム"),
        }
        h.render(vec![]);
        h.click("しきい値を読み込む");
        assert!(h.app.controller.raw_thresholds().is_none(), "case {case}");
        assert!(!h.app.controller.raw_thresholds_pending(), "case {case}");
    }
}

#[test]
fn raw_threshold_reader_accepts_focused_enter_and_space_as_explicit_actions() {
    for key in [egui::Key::Enter, egui::Key::Space] {
        let mut h = Harness::loaded([960., 820.]);
        h.click("入力しきい値");
        let button = read_and_get_button_id(&mut h);
        let previous = h.app.controller.raw_thresholds().unwrap().read_at;
        h.ctx.memory_mut(|m| m.request_focus(button));
        h.render(vec![key_event(key, true)]);
        assert!(
            h.app.controller.raw_thresholds_pending(),
            "focused {key:?} must request the read"
        );
        assert!(h.app.controller.raw_thresholds().is_none());
        h.render(vec![key_event(key, false)]);
        h.settle();
        assert!(h.app.controller.raw_thresholds().unwrap().read_at > previous);
        assert!(h.visible("FROM: 13"));
        assert!(h.app.controller.live);
        assert!(!h.app.controller.editor.as_ref().unwrap().dirty());
    }
}

#[test]
fn raw_threshold_disabled_reader_rejects_focused_keyboard_activation() {
    for case in 0..5 {
        for key in [egui::Key::Enter, egui::Key::Space] {
            let mut h = Harness::loaded([960., 820.]);
            h.click("入力しきい値");
            let button = read_and_get_button_id(&mut h);
            match case {
                0 => h.app.controller.live = false,
                1 => {
                    h.app.controller.pending = Some(crate::protocol::Ticket {
                        generation: 0,
                        operation: 999,
                    })
                }
                2 => h.app.controller.close_requested = true,
                3 => {
                    h.app.confirm = Some(Confirm::Load(
                        h.app.controller.editor.as_ref().unwrap().baseline.clone(),
                    ))
                }
                _ => h.click("システム"),
            }
            let pending = h.app.controller.pending;
            h.ctx.memory_mut(|m| m.request_focus(button));
            h.render(vec![key_event(key, true)]);
            h.render(vec![key_event(key, false)]);
            assert!(
                !h.app.controller.raw_thresholds_pending(),
                "case {case}, key {key:?}"
            );
            assert_eq!(h.app.controller.pending, pending);
        }
    }
}

#[test]
fn raw_threshold_accesskit_click_targets_the_real_reader_and_respects_guards() {
    for case in 0..6 {
        let mut h = Harness::loaded([960., 820.]);
        h.ctx.enable_accesskit();
        h.click("入力しきい値");
        h.render(vec![]);
        let node = h
            .output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find(|(_, node)| {
                node.role() == egui::accesskit::Role::Button
                    && node.label() == Some("しきい値を読み込む")
            })
            .expect("actual reader button must exist in the accessibility tree")
            .0;
        match case {
            0 => {}
            1 => h.app.controller.live = false,
            2 => {
                h.app.controller.pending = Some(crate::protocol::Ticket {
                    generation: 0,
                    operation: 999,
                })
            }
            3 => h.app.controller.close_requested = true,
            4 => {
                h.app.confirm = Some(Confirm::Load(
                    h.app.controller.editor.as_ref().unwrap().baseline.clone(),
                ))
            }
            _ => h.click("システム"),
        }
        let pending = h.app.controller.pending;
        h.render(vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Click,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: node,
                data: None,
            },
        )]);
        if case == 0 {
            assert!(
                h.app.controller.raw_thresholds_pending(),
                "AccessKit Click must request the read"
            );
            h.settle();
            assert!(h.visible("FROM: 13"));
            assert!(!h.app.controller.editor.as_ref().unwrap().dirty());
        } else {
            assert!(!h.app.controller.raw_thresholds_pending(), "case {case}");
            assert_eq!(h.app.controller.pending, pending);
            assert!(h.app.controller.raw_thresholds().is_none());
        }
    }
}

fn read_and_get_button_id(h: &mut Harness) -> egui::Id {
    let pos = h.rect("しきい値を読み込む").center();
    for pressed in [true, false] {
        h.render(vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            },
        ]);
    }
    let button = h
        .ctx
        .interaction_snapshot(|s| s.clicked)
        .expect("reader button was clicked");
    // The pending frame disables the reader and may surrender its focus.
    h.settle();
    button
}

#[test]
fn raw_threshold_unfocused_enter_theme_and_key_navigation_never_refresh_the_snapshot() {
    let mut h = Harness::loaded([960., 820.]);
    h.click("入力しきい値");
    h.click("しきい値を読み込む");
    h.settle();
    let read_at = h.app.controller.raw_thresholds().unwrap().read_at;
    h.app.controller.list().unwrap();
    let before_ticket = h.app.controller.pending.unwrap();
    h.settle();
    if let Some(focused) = h.ctx.memory(|m| m.focused()) {
        h.ctx.memory_mut(|m| m.surrender_focus(focused));
    }
    assert!(h.ctx.memory(|m| m.focused()).is_none());
    h.render(vec![key_event(egui::Key::Enter, true)]);
    h.render(vec![key_event(egui::Key::Enter, false)]);
    assert_eq!(h.app.controller.raw_thresholds().unwrap().read_at, read_at);
    h.click("S\n#32");
    h.click("S\n#32");
    h.appearance("システム", "ダーク");
    h.click("入力テスト");
    h.render(vec![
        key_event(egui::Key::A, true),
        egui::Event::Text("a".into()),
    ]);
    h.render(vec![key_event(egui::Key::A, false)]);
    assert_eq!(h.app.controller.raw_thresholds().unwrap().read_at, read_at);
    h.app.controller.list().unwrap();
    assert_eq!(
        h.app.controller.pending.unwrap().operation,
        before_ticket.operation + 1
    );
    h.settle();
}

#[test]
fn raw_threshold_snapshot_fits_default_and_remains_reachable_at_minimum() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        for size in [[960., 820.], [700., 580.]] {
            let mut h = Harness::loaded(size);
            h.ctx.set_theme(theme);
            h.click("入力しきい値");
            h.click("しきい値を読み込む");
            h.settle();
            let mut positions = std::collections::BTreeSet::new();
            let before = h.rect("NIZ Utility");
            assert!(h.visible("しきい値を読み込む"));
            assert!(h.visible("FROM: 13"));
            for _ in 0..8 {
                for (text, rect, clip) in h.texts() {
                    if clip.contains_rect(rect)
                        && let Some((_, id)) = text.split_once("\n#")
                    {
                        assert!(rect.left() >= 0. && rect.right() <= size[0]);
                        positions.insert(id.parse::<usize>().unwrap());
                    }
                }
                if size[0] == 960. {
                    break;
                }
                h.scroll_down();
            }
            assert_eq!(positions, (1..=66).collect());
            if size[0] == 960. {
                assert!(h.visible("操作の記録・詳細"));
                h.scroll_down();
                assert_eq!(h.rect("NIZ Utility"), before);
            }
        }
    }
}

#[test]
fn held_key_calibration_is_visible_without_legacy_full_write_controls() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        for live in [true, false] {
            let mut h = Harness::loaded([960., 820.]);
            h.ctx.set_theme(theme);
            h.app.view = View::Calibration;
            h.app.controller.live = live;
            h.render(vec![]);
            assert!(h.visible("選択中のキー: #31 · A"));
            assert!(h.visible("すべてのキーを離して準備"));
            assert!(!h.visible("しきい値について"));
            assert!(h.visible("入力しきい値"));
            for _ in 0..8 {
                h.scroll_down();
            }
            h.click("対象デバイス");
            for _ in 0..8 {
                h.render(vec![]);
            }
            for (text, _, _) in h.texts() {
                assert!(!text.contains("全体の反応しきい値"));
                assert!(!text.contains("7/&"));
                assert!(!text.contains("復元できない実験的操作"));
                assert!(!text.contains("全キーを離して開始"));
                assert!(!text.contains("押したまま、マウスで続ける"));
                assert!(!text.contains("すべてのキーを離して終了"));
            }
            assert!(h.app.controller.pending.is_none());
            assert!(h.app.controller.calibration.is_none());
        }
    }
}

#[test]
fn per_key_threshold_selection_never_edits_or_dispatches_for_any_physical_key() {
    let physical_legends = [
        "Esc",
        "1",
        "2",
        "3",
        "4",
        "5",
        "6",
        "7",
        "8",
        "9",
        "0",
        "-",
        "=",
        "`",
        "Backspace",
        "Tab",
        "Q",
        "W",
        "E",
        "R",
        "T",
        "Y",
        "U",
        "I",
        "O",
        "P",
        "[",
        "]",
        "\\",
        "Caps Lock",
        "A",
        "S",
        "D",
        "F",
        "G",
        "H",
        "J",
        "K",
        "L",
        ";",
        "'",
        "Enter",
        "Left Shift",
        "Z",
        "X",
        "C",
        "V",
        "B",
        "N",
        "M",
        ",",
        ".",
        "/",
        "Right Shift / Up",
        "Delete",
        "Left Control",
        "Left Fn",
        "Win",
        "Left Alt",
        "Space",
        "Right Fn",
        "Right Alt",
        "Right Control",
        "Left",
        "Down",
        "Right",
    ];
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let mut h = Harness::loaded([960., 820.]);
        h.ctx.set_theme(theme);
        let e = h.app.controller.editor.as_mut().unwrap();
        e.select_from(30);
        e.choose_output(44);
        e.edit().unwrap();
        e.select_from(31);
        e.choose_output(45);
        e.edit().unwrap();
        assert!(e.undo());
        let before = (
            e.draft.records.clone(),
            e.baseline.records.clone(),
            e.selection,
            e.output_ready(),
            e.can_undo(),
            e.can_redo(),
        );
        assert!(e.can_undo() && e.can_redo());
        let generation = h.app.controller.generation;
        let target = h.app.controller.attached_target.clone();
        let selected_path = h.app.controller.selected_path.clone();
        h.app.controller.list().unwrap();
        let before_ticket = h.app.controller.pending.unwrap();
        h.settle();
        h.app.view = View::Calibration;
        h.render(vec![]);
        for (slot, legend) in physical_legends.iter().enumerate() {
            let label = h
                .texts()
                .into_iter()
                .find(|(text, _, _)| text.ends_with(&format!("\n#{:02}", slot + 1)))
                .unwrap()
                .0;
            for _ in 0..2 {
                h.click(&label);
                assert_eq!(h.app.calibration_position, slot);
                assert!(h.visible(&format!("選択中のキー: #{:02} · {legend}", slot + 1)));
                h.render(vec![key_event(egui::Key::Enter, true)]);
                h.render(vec![key_event(egui::Key::Enter, false)]);
                assert!(h.app.controller.pending.is_none());
                assert!(h.app.controller.calibration.is_none());
            }
        }
        let e = h.app.controller.editor.as_ref().unwrap();
        assert_eq!(
            (
                e.draft.records.clone(),
                e.baseline.records.clone(),
                e.selection,
                e.output_ready(),
                e.can_undo(),
                e.can_redo()
            ),
            before,
        );
        assert_eq!(h.app.controller.generation, generation);
        assert_eq!(h.app.controller.attached_target, target);
        assert_eq!(h.app.controller.selected_path, selected_path);
        assert!(h.app.controller.live);
        h.app.controller.list().unwrap();
        let after_ticket = h.app.controller.pending.unwrap();
        assert_eq!(
            after_ticket.operation,
            before_ticket.operation + 1,
            "physical selection or Enter dispatched an unrequested device command"
        );
        h.settle();
    }
}

#[test]
fn per_key_threshold_board_tooltip_states_selection_has_no_device_effect() {
    let mut h = Harness::loaded([960., 820.]);
    h.app.view = View::Calibration;
    h.render(vec![]);
    let pos = h.rect("S\n#32").center();
    h.render(vec![egui::Event::PointerMoved(pos)]);
    for _ in 0..90 {
        h.render(vec![]);
    }
    assert!(h.texts().iter().any(|(text, _, _)| {
        text.contains("キーの選択だけでは設定を変更しません") && text.contains("位置 #32")
    }));
    assert_eq!(h.app.calibration_position, 30);
    assert!(h.app.controller.pending.is_none());
}

#[test]
fn physical_calibration_choices_respect_popup_modal_and_pending_guards() {
    let mut h = Harness::loaded([960., 820.]);
    h.click("入力しきい値");
    h.click("S\n#32");
    h.click("システム");
    h.click("LFn\n#57");
    assert_eq!(h.app.calibration_position, 31);
    h.click("LFn\n#57");
    assert_eq!(h.app.calibration_position, 56);
    h.app.confirm = Some(Confirm::Load(
        h.app.controller.editor.as_ref().unwrap().baseline.clone(),
    ));
    h.render(vec![]);
    h.click("RFn\n#61");
    assert_eq!(h.app.calibration_position, 56);
    h.app.confirm = None;
    h.app.controller.pending = Some(crate::protocol::Ticket {
        generation: 0,
        operation: 999,
    });
    h.render(vec![]);
    h.click("RAlt\n#62");
    assert_eq!(h.app.calibration_position, 56);
    h.app.controller.pending = None;
    h.app.controller.close_requested = true;
    h.render(vec![]);
    h.click("RAlt\n#62");
    assert_eq!(h.app.calibration_position, 56);
    assert!(!h.app.controller.editor.as_ref().unwrap().dirty());
}

#[test]
fn per_key_threshold_tab_preserves_owned_demo_cancel_and_generic_terminal_result() {
    let mut h = Harness::loaded([960., 820.]);
    h.app.view = View::Calibration;
    h.render(vec![]);
    h.click("S\n#32");
    let target = h.app.controller.attached_target.clone();
    h.app
        .controller
        .begin(h.app.backup.clone(), crate::controller::Activation::Pointer)
        .unwrap();
    h.render(vec![]);
    h.click("A\n#31");
    assert_eq!(h.app.calibration_position, 31);
    h.settle();
    assert!(h.visible("進行中の校正 · 全キーに作用"));
    assert!(h.visible("中止を要求"));
    for (text, _, _) in h.texts() {
        assert!(!text.contains("押したまま、マウスで続ける"));
        assert!(!text.contains("すべてのキーを離して終了"));
        assert!(!text.contains("全キーを離して開始"));
    }
    h.click("A\n#31");
    assert_eq!(h.app.calibration_position, 31);
    h.click("中止を要求");
    h.settle();
    let status = h.app.controller.calibration.as_ref().unwrap();
    assert_eq!(status.phase, CalibrationPhase::Aborted);
    assert!(status.cancellation_requested);
    assert_eq!(status.unlock, UnlockOutcome::Sent);
    assert!(!h.app.controller.calibration_active());
    assert!(h.app.controller.pending.is_none());
    let result = "中止しました";
    assert!(h.visible(result));
    h.click("A\n#31");
    assert_eq!(h.app.calibration_position, 30);
    assert!(h.visible(result));
    h.click("校正の結果・対象・バックアップ");
    for _ in 0..8 {
        h.render(vec![]);
    }
    assert!(h.visible("ロック解除: 送信済み。応答・タイピング回復は未確認です。"));
    assert_eq!(h.app.controller.attached_target, target);
    assert!(!h.app.controller.editor.as_ref().unwrap().dirty());
}

#[test]
fn physical_calibration_board_is_complete_default_and_reachable_at_minimum() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        for size in [[960., 820.], [700., 580.]] {
            let mut h = Harness::loaded(size);
            h.ctx.set_theme(theme);
            h.click("入力しきい値");
            let mut positions = std::collections::BTreeSet::new();
            for _ in 0..8 {
                for (text, rect, clip) in h.texts() {
                    if clip.contains_rect(rect)
                        && let Some((_, id)) = text.split_once("\n#")
                    {
                        assert!(rect.left() >= 0. && rect.right() <= size[0]);
                        positions.insert(id.parse::<usize>().unwrap());
                    }
                }
                if size[0] == 960. {
                    break;
                }
                h.scroll_down();
            }
            assert_eq!(positions, (1..=66).collect());
        }
    }
}

#[test]
fn disabled_keycaps_and_popup_labels_remain_readable_in_both_themes() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let mut h = Harness::loaded([960., 820.]);
        h.ctx.set_theme(theme);
        h.app.controller.pending = Some(crate::protocol::Ticket {
            generation: 0,
            operation: 999,
        });
        h.render(vec![]);
        assert_text_contrast(&h, "A\n#31", 3.);
        assert_text_contrast(&h, "1\n#02", 3.);
        h.app.controller.pending = None;
        h.render(vec![]);
        h.click("システム");
        for _ in 0..12 {
            h.render(vec![]);
        }
        assert_text_contrast(&h, "ダーク", 4.5);
    }
}

#[test]
fn appearance_switch_preserves_unsupported_raw_assignment_and_output_readiness() {
    let mut h = Harness::loaded([960., 820.]);
    let mut profile = h.app.controller.editor.as_ref().unwrap().draft.clone();
    profile.records[30][0][6] = 178;
    let before = profile.records.clone();
    let mut editor = crate::editor::Editor::new(profile);
    editor.select_from(30);
    assert!(!editor.output_ready());
    h.app.controller.editor = Some(editor);
    h.render(vec![]);
    h.appearance("システム", "ダーク");
    let editor = h.app.controller.editor.as_ref().unwrap();
    assert!(!editor.output_ready());
    assert!(!editor.can_undo());
    assert!(!editor.dirty());
    assert_eq!(editor.draft.records, before);
    assert_eq!(editor.selection.from, 30);
}

fn assert_text_contrast(h: &Harness, label: &str, minimum: f64) {
    let (index, text) = h
        .output
        .shapes
        .iter()
        .enumerate()
        .find_map(|(i, s)| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == label => Some((i, t)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing rendered {label:?}"));
    let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
    let background = h.output.shapes[..index]
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Rect(r) if r.rect.contains(rect.center()) && r.fill.a() != 0 => {
                Some(r.fill)
            }
            _ => None,
        })
        .fold(None, |background, fill| {
            Some(match background {
                Some(background) => composite(fill, background),
                None => {
                    assert_eq!(fill.a(), 255, "first background must be opaque");
                    fill
                }
            })
        })
        .expect("painted background behind label");
    let color = text.override_text_color.unwrap_or_else(|| {
        // Painter disabled opacity is baked into glyph mesh colors, not job formats.
        let row = &text.galley.rows[0].visuals;
        let c = row.mesh.vertices[row.glyph_vertex_range.start].color;
        if c == Color32::PLACEHOLDER {
            text.fallback_color
        } else {
            c
        }
    });
    let color = color.gamma_multiply(text.opacity_factor);
    let ratio = contrast(color, background);
    assert!(
        ratio >= minimum,
        "{label:?} contrast {ratio:.2} < {minimum}: {color:?} on {background:?}"
    );
}
fn composite(foreground: Color32, background: Color32) -> Color32 {
    let fg = foreground.to_srgba_unmultiplied();
    let bg = background.to_srgba_unmultiplied();
    let alpha = f64::from(fg[3]) / 255.;
    let channels: [u8; 3] = std::array::from_fn(|i| {
        (f64::from(fg[i]) * alpha + f64::from(bg[i]) * (1. - alpha)).round() as u8
    });
    Color32::from_rgb(channels[0], channels[1], channels[2])
}
fn contrast(foreground: Color32, background: Color32) -> f64 {
    let bg = background.to_srgba_unmultiplied();
    let fg = foreground.to_srgba_unmultiplied();
    let alpha = f64::from(fg[3]) / 255.;
    let luminance = |rgb: [f64; 3]| {
        let linear = rgb.map(|v| {
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        });
        linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722
    };
    let bg_rgb = [bg[0], bg[1], bg[2]].map(|v| f64::from(v) / 255.);
    let fg_rgb =
        std::array::from_fn(|i| f64::from(fg[i]) / 255. * alpha + bg_rgb[i] * (1. - alpha));
    let (a, b) = (luminance(fg_rgb), luminance(bg_rgb));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

// Catches omitted held-only entry, accidental keyboard dispatch, and target thawing.
#[test]
fn held_calibration_prepares_frozen_target_and_finishes_without_an_os_keypress() {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let mut h = Harness::loaded([960., 820.]);
        h.ctx.set_theme(theme);
        h.click("入力しきい値");
        h.click("S\n#32");
        assert!(h.visible("すべてのキーを離して準備"));
        h.click("すべてのキーを離して準備");
        h.click("A\n#31");
        assert_eq!(h.app.calibration_position, 31);
        h.settle();
        assert!(h.visible("Sを押したまま実行"));
        let execute = h.rect("実行");
        let cancel = h.rect("キャンセル");
        assert!((execute.center().y - cancel.center().y).abs() < 1.);
        // Weak/remapped keys may produce no OS event. Execution still works by pointer.
        h.render(vec![key_event(egui::Key::Enter, true)]);
        h.render(vec![key_event(egui::Key::Enter, false)]);
        assert!(h.app.controller.pending.is_none());
        h.click("実行");
        h.settle();
        assert_eq!(
            h.app.controller.calibration.as_ref().unwrap().phase,
            CalibrationPhase::Complete
        );
        assert!(!h.app.controller.calibration_active());
        assert!(h.visible("TO: 32"));
        assert!(!h.visible("他のキーの変化: 0 / 65"));
        assert!(h.visible("DEMO値"));
        assert!(!h.app.controller.live);
        assert!(!h.app.controller.editor.as_ref().unwrap().dirty());
        assert!(h.visible("→\n#66"));
    }
}

#[test]
fn held_calibration_cancel_keeps_the_selected_position_and_reports_missing_comparison() {
    let mut h = Harness::loaded([960., 820.]);
    h.click("入力しきい値");
    h.click("すべてのキーを離して準備");
    h.settle();
    h.click("S\n#32");
    assert_eq!(h.app.calibration_position, 30);
    h.click("キャンセル");
    h.settle();
    assert_eq!(
        h.app.controller.calibration.as_ref().unwrap().phase,
        CalibrationPhase::Aborted
    );
    assert!(h.visible("中止しました"));
    assert!(!h.visible("他のキーの変化: 0 / 65"));
}

// Catches stale/unavailable selected values despite a complete post-observation, and neutral neighbor warnings.
#[test]
fn held_result_uses_complete_after_values_and_warns_about_neighbor_changes() {
    let mut h = Harness::loaded([960., 820.]);
    h.click("入力しきい値");
    h.click("すべてのキーを離して準備");
    h.settle();
    h.click("実行");
    h.settle();
    let held = h
        .app
        .controller
        .calibration
        .as_mut()
        .unwrap()
        .held_key
        .as_mut()
        .unwrap();
    held.after.as_mut().unwrap()[31] = 90;
    h.render(vec![]);
    assert!(h.visible("TO: 31"));
    assert!(h.visible("他のキーも変化: 1個"));
    let color = h
        .output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == "他のキーも変化: 1個" => {
                Some(t.galley.job.sections[0].format.color)
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(
        color,
        crate::theme::Palette::for_dark(h.ctx.theme() == egui::Theme::Dark).warning
    );
    h.click("S\n#32");
    assert!(h.visible("TO: 90"));
    assert!(h.visible("FROM: 31"));
}

#[test]
fn held_result_never_claims_no_change_from_partial_read_and_keeps_complete_observation_on_failure()
{
    let mut h = Harness::loaded([960., 820.]);
    h.click("入力しきい値");
    h.click("すべてのキーを離して準備");
    h.settle();
    h.click("実行");
    h.settle();
    let status = h.app.controller.calibration.as_mut().unwrap();
    status.phase = CalibrationPhase::Failed;
    status.settings_verified = false;
    status.original_error = Some("settings mismatch".into());
    let held = status.held_key.as_mut().unwrap();
    held.after = held.before.clone();
    h.render(vec![]);
    assert!(h.visible("TO: 30"));
    assert!(!h.visible("他のキーの変化: 0 / 65"));
    assert!(h.visible("FROM: 30"));
    h.app
        .controller
        .calibration
        .as_mut()
        .unwrap()
        .held_key
        .as_mut()
        .unwrap()
        .after = Some(vec![30; 65]);
    h.render(vec![]);
    assert!(h.visible("校正に失敗しました。読み込み直してください。"));
    assert!(!h.visible("他のキーの変化: 0 / 65"));
    assert!(!h.visible("値の変化は観測されませんでした。反応が改善したことは未確認です。"));
    assert!(h.visible("FROM: 30"));
    assert!(h.visible("TO: —"));
    assert!(h.visible("生値: 30 (0x1E)"));
}

#[test]
fn held_execute_rejects_real_focused_keyboard_and_accessibility_actions() {
    for label in ["すべてのキーを離して準備", "実行"] {
        let mut h = Harness::loaded([960., 820.]);
        h.ctx.enable_accesskit();
        h.click("入力しきい値");
        if label == "実行" {
            h.click("すべてのキーを離して準備");
            h.settle();
        }
        h.render(vec![]);
        let node = h
            .output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find(|(_, n)| n.role() == egui::accesskit::Role::Button && n.label() == Some(label))
            .expect("actual changing action in accessibility tree")
            .0;
        // AccessKit nodes store the actual high-entropy egui widget ID.
        let button_id = unsafe { egui::Id::from_high_entropy_bits(node.0) };
        for key in [egui::Key::Enter, egui::Key::Space] {
            h.ctx.memory_mut(|m| m.request_focus(button_id));
            h.render(vec![key_event(key, true)]);
            h.render(vec![key_event(key, false)]);
            assert_eq!(h.ctx.memory(|m| m.focused()), Some(button_id));
            assert!(h.app.controller.pending.is_none());
            assert!(
                h.app
                    .controller
                    .calibration
                    .as_ref()
                    .is_none_or(|s| !s.held_attempted)
            );
        }
        h.render(vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Click,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: node,
                data: None,
            },
        )]);
        assert!(h.app.controller.pending.is_none());
        assert!(
            h.app
                .controller
                .calibration
                .as_ref()
                .is_none_or(|s| !s.held_attempted)
        );
        h.click(label);
        h.settle();
        assert!(h.app.controller.calibration.is_some());
    }
}

#[test]
fn held_results_expose_all_66_literal_pairs_in_collapsed_details() {
    let mut h = Harness::loaded([960., 820.]);
    h.click("入力しきい値");
    h.click("すべてのキーを離して準備");
    h.settle();
    h.click("実行");
    h.settle();
    assert!(!h.visible("全66キーの比較 · 公式配列の刻印"));
    h.scroll_down();
    h.click("校正の結果・対象・バックアップ");
    for _ in 0..10 {
        h.render(vec![]);
    }
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..20 {
        let texts = h.texts();
        for (label, rect, clip) in &texts {
            let Some((number, _)) = label
                .strip_prefix('#')
                .and_then(|label| label.split_once(" · "))
            else {
                continue;
            };
            if label.contains(":") || !clip.contains_rect(*rect) {
                continue;
            }
            let position = number.parse::<usize>().unwrap() - 1;
            let expected = if position == 30 {
                "30 → 31".to_owned()
            } else {
                format!("{position} → {position}")
            };
            assert!(
                texts
                    .iter()
                    .any(|(value, value_rect, value_clip)| value == &expected
                        && value_clip.contains_rect(*value_rect)
                        && (value_rect.center().y - rect.center().y).abs() < 1.),
                "missing row values for {label}"
            );
            seen.insert(position);
        }
        h.scroll_down();
    }
    assert_eq!(seen, (0..66).collect());
    assert!(h.app.controller.pending.is_none());
}

#[test]
fn held_changing_controls_respect_popup_confirmation_close_and_pending_guards() {
    for stage in [false, true] {
        for guard in 0..4 {
            let mut h = Harness::loaded([960., 820.]);
            h.click("入力しきい値");
            if stage {
                h.click("すべてのキーを離して準備");
                h.settle();
            }
            match guard {
                0 => h.click("システム"),
                1 => {
                    h.app.confirm = Some(Confirm::Load(
                        h.app.controller.editor.as_ref().unwrap().baseline.clone(),
                    ))
                }
                2 => h.app.controller.close_requested = true,
                _ => {
                    h.app.controller.pending = Some(crate::protocol::Ticket {
                        generation: 1,
                        operation: 999,
                    })
                }
            }
            let pending = h.app.controller.pending;
            h.render(vec![]);
            h.click(if stage {
                "実行"
            } else {
                "すべてのキーを離して準備"
            });
            assert_eq!(h.app.controller.pending, pending);
            assert!(
                h.app
                    .controller
                    .calibration
                    .as_ref()
                    .is_none_or(|s| !s.held_attempted)
            );
            assert_eq!(h.app.calibration_position, 30);
        }
    }
}

// Catches calibration values from A appearing under B, or under a replaced/removed descriptor.
fn assert_held_result_source_invalidation(change: usize) {
    let mut h = Harness::loaded([960., 820.]);
    h.click("入力しきい値");
    h.click("すべてのキーを離して準備");
    h.settle();
    h.click("実行");
    h.settle();
    assert!(h.visible("TO: 31"));
    assert!(h.visible("TO: 31"));
    if change == 0 {
        let mut second = h.app.controller.devices[0].clone();
        second.path = "demo://second-keyboard".into();
        second.product = "Simulated second keyboard".into();
        h.app.controller.devices.push(second);
        h.app
            .controller
            .select_device("demo://second-keyboard".into())
            .unwrap();
    } else {
        let mut devices = h.app.controller.devices.clone();
        match change {
            1 => devices[0].product = "Replacement descriptor".into(),
            2 => devices[0].identity.product_id = 0x512a,
            _ => devices.clear(),
        }
        h.app.controller.list().unwrap();
        h.app.controller.process(crate::protocol::Event {
            ticket: h.app.controller.pending.unwrap(),
            response: crate::protocol::Response::Devices(devices),
        });
    }
    h.render(vec![]);
    assert!(
        h.visible("FROM: —"),
        "change {change}: stale post-observation under new source"
    );
    assert!(!h.visible("TO: 31"));
    assert!(h.visible("生値: —"));
    assert!(!h.visible("生値: 31 (0x1F)"));
    assert!(!h.visible("他のキーの変化: 0 / 65"));
    assert!(h.app.controller.calibration.is_none());
    assert!(!h.app.controller.live);
}

#[test]
fn held_result_is_invalidated_on_switch_to_second_device() {
    assert_held_result_source_invalidation(0);
}
#[test]
fn held_result_is_invalidated_on_same_path_product_change() {
    assert_held_result_source_invalidation(1);
}
#[test]
fn held_result_is_invalidated_on_same_path_identity_change() {
    assert_held_result_source_invalidation(2);
}
#[test]
fn held_result_is_invalidated_on_device_removal() {
    assert_held_result_source_invalidation(3);
}

#[test]
fn held_result_is_cleared_before_advancing_read_generation() {
    let mut h = Harness::loaded([960., 820.]);
    h.click("入力しきい値");
    h.click("すべてのキーを離して準備");
    h.settle();
    h.click("実行");
    h.settle();
    assert!(h.visible("TO: 31"));
    let generation = h.app.controller.generation;
    h.app
        .controller
        .read(
            "demo://keyboard".into(),
            crate::controller::ReadChoice::Discard,
        )
        .unwrap();
    assert_eq!(h.app.controller.generation, generation + 1);
    assert!(h.app.controller.calibration.is_none());
    h.render(vec![]);
    assert!(h.visible("FROM: —"));
    assert!(!h.visible("TO: 31"));
    assert!(!h.visible("生値: 31 (0x1F)"));
}

// Catches a transcript/panel returning or key-up leaving a candidate lit.
#[test]
fn compact_input_test_only_renders_the_keyboard_and_tracks_key_release() {
    let mut h = Harness::loaded([960., 820.]);
    h.click("入力テスト");
    let idle = label_fill(&h, "A\n#31");
    h.render(vec![
        key_event(egui::Key::A, true),
        egui::Event::Text("private test text".into()),
    ]);
    assert_ne!(label_fill(&h, "A\n#31"), idle);
    for removed in [
        "private test text",
        "このアプリが受け取った入力",
        "押下中: A",
        "表示を消す",
        "通常",
        "右 Fn",
        "左 Fn",
    ] {
        assert!(!h.visible(removed), "extra input UI: {removed}");
    }
    assert_eq!(
        h.texts()
            .iter()
            .filter(|(s, r, c)| s.contains("\n#") && c.contains_rect(*r))
            .count(),
        66
    );
    h.render(vec![key_event(egui::Key::A, false)]);
    assert_eq!(label_fill(&h, "A\n#31"), idle);
    assert!(h.app.controller.pending.is_none());
}

// Catches verbose duplicate summaries and missing complete before/after values.
#[test]
fn compact_threshold_result_only_shows_selected_from_to_and_actions() {
    let mut h = Harness::loaded([960., 820.]);
    h.click("入力しきい値");
    h.click("すべてのキーを離して準備");
    h.settle();
    h.click("実行");
    h.settle();
    assert!(h.visible("FROM: 30"));
    assert!(h.visible("TO: 31"));
    assert!(h.visible("生値: 31 (0x1F)"));
    for removed in [
        "しきい値について",
        "他のキーの変化: 0 / 65",
        "しきい値: 31",
        "通常スキャンの値を比較しました。実際の反応は入力テストで確認してください。",
    ] {
        assert!(!h.visible(removed), "extra threshold UI: {removed}");
    }
    assert!(h.visible("選択中のキー: #31 · A"));
    assert!(h.app.controller.pending.is_none());
}

// Catches missing result wiring and wrong physical position after a global apply.
#[test]
fn compact_threshold_global_result_uses_current_selected_from_to() {
    let mut h = Harness::loaded([960., 820.]);
    h.click("入力しきい値");
    h.click("全体の反応（一時）");
    for _ in 0..8 {
        h.render(vec![]);
    }
    h.app.sensitivity_coefficient = 20;
    assert_text_contrast(&h, "適用", 4.5);
    h.click("適用");
    h.settle();
    assert!(h.visible("FROM: 20"));
    assert!(h.visible("TO: 16"));
    assert!(h.visible("生値: 16 (0x10)"));
    assert!(h.app.controller.calibration.is_none());
    h.click("S\n#32");
    assert!(h.visible("FROM: 21"));
    assert!(h.visible("TO: 17"));
    assert!(h.visible("生値: 17 (0x11)"));
    assert!(h.app.controller.pending.is_none());
    assert!(!h.app.controller.editor.as_ref().unwrap().dirty());
}

// Catches showing the released observation while the held operation may already change R.
#[test]
fn raw_value_hides_pre_calibration_observation_during_owned_write() {
    let mut h = Harness::loaded([960., 820.]);
    h.click("入力しきい値");
    h.click("すべてのキーを離して準備");
    h.settle();
    assert!(h.visible("生値: 30 (0x1E)"));
    h.app.controller.held(Activation::Pointer).unwrap();
    // Render before consuming the worker's completed reply.
    h.output = h
        .ctx
        .run_ui(h.input(vec![]), |ui| h.app.ui(ui, &mut h.frame));
    h.output.textures_delta.clear();
    assert!(h.visible("生値: …"));
    assert!(!h.visible("生値: 30 (0x1E)"));
    h.settle();
    assert!(h.visible("生値: 31 (0x1F)"));
}
