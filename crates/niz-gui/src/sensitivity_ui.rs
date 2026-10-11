use crate::{
    controller::{Activation, Controller, sensitivity_message},
    keyboard,
    worker::ChannelPort,
};
use eframe::egui;
use std::path::Path;

pub(crate) fn show(
    ui: &mut egui::Ui,
    controller: &mut Controller<ChannelPort>,
    coefficient: &mut u8,
    backup: &Path,
    demo: bool,
    popup_at_frame_start: bool,
) {
    egui::CollapsingHeader::new("全体の反応（一時）").show(ui, |ui| {
        let enabled = ui.is_enabled()
            && !controller.blocked()
            && !popup_at_frame_start
            && !egui::Popup::is_any_open(ui.ctx())
            && ui.ctx().memory(|m| m.top_modal_layer().is_none());
        ui.small("次に適用する設定です。現在の設定値は読み取りません。");
        ui.horizontal(|ui| {
            ui.label("反応しにくい");
            let mut ease = 40u8.saturating_sub((*coefficient).clamp(10, 40));
            if ui
                .add_enabled(
                    enabled,
                    egui::Slider::new(&mut ease, 0..=30).show_value(false),
                )
                .changed()
            {
                *coefficient = 40 - ease;
            }
            ui.label("反応しやすい");
            if ui
                .add_enabled(
                    enabled && controller.can_read_raw_thresholds(),
                    egui::Button::new(egui::RichText::new("適用").color(egui::Color32::WHITE))
                        .fill(egui::Color32::from_rgb(180, 40, 40)),
                )
                .on_hover_text("キーを離して適用してください。入力が一時停止します。")
                .clicked_by(egui::PointerButton::Primary)
                && let Err(error) = controller.adjust_sensitivity(
                    backup.to_path_buf(),
                    *coefficient,
                    Activation::Pointer,
                )
            {
                controller.message = error;
            }
            if controller.sensitivity_pending() {
                ui.spinner();
            }
        });
        if let Some(status) = &controller.sensitivity {
            let failed = controller
                .failure
                .as_ref()
                .is_some_and(|f| f.sensitivity.is_some());
            if failed {
                ui.colored_label(
                    egui::Color32::from_rgb(200, 70, 50),
                    "適用を完了できませんでした · 詳細を確認してください",
                );
            } else {
                let summary = sensitivity_message(status);
                ui.label(if demo {
                    format!("DEMO · {summary}")
                } else {
                    summary
                });
            }
            egui::CollapsingHeader::new("前後のしきい値・詳細").show(ui, |ui| {
                if demo {
                    ui.strong("DEMO: 生成した比較用の値です。実機の読み取りではありません。");
                } else {
                    ui.label("DC1 の前後読み取り · 66キーの物理配置順 · 内部しきい値");
                }
                if let Some(target) = &controller.attached_target {
                    ui.label(format!("対象: {} · {}", target.product, target.path));
                }
                if let Some(editor) = &controller.editor {
                    ui.label(&editor.baseline.firmware);
                }
                ui.label(format!(
                    "今回指定した係数: {} · 送信試行: {} · 送信完了: {}",
                    status.coefficient, status.attempted, status.sent
                ));
                ui.label(format!(
                    "段階: {:?} · 設定確認: {} · 照明確認: {} · 中止要求: {}",
                    status.phase,
                    status.settings_verified,
                    status.lighting_verified,
                    status.cancellation_requested
                ));
                if let Some(error) = &status.original_error {
                    ui.label(error);
                }
                if let Some(failure) = &controller.failure {
                    ui.label(&failure.message);
                }
                if let Some(paths) = &status.backup {
                    ui.label(format!("バックアップ: {paths:?}"));
                }
                let before = status.before.as_deref().filter(|v| v.len() == 66);
                let after = status.after.as_deref().filter(|v| v.len() == 66);
                if before.is_none() || after.is_none() {
                    ui.label("前後いずれかの完全な読み取りがありません。変化は判定できません。");
                }
                egui::Grid::new("sensitivity_physical_comparison")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("物理キー");
                        ui.strong("前 → 後");
                        ui.end_row();
                        for position in 0..66 {
                            ui.label(format!(
                                "#{:02} · {}",
                                position + 1,
                                keyboard::physical_legend(position)
                            ));
                            let b = before
                                .map(|v| v[position].to_string())
                                .unwrap_or_else(|| "—".into());
                            let a = after
                                .map(|v| v[position].to_string())
                                .unwrap_or_else(|| "—".into());
                            ui.label(format!("{b} → {a}"));
                            ui.end_row();
                        }
                    });
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        controller::{Controller, ReadChoice},
        worker::{ChannelPort, spawn},
    };
    use eframe::egui;
    use std::path::Path;
    struct Harness {
        ctx: egui::Context,
        c: Controller<ChannelPort>,
        coefficient: u8,
        output: egui::FullOutput,
        popup: bool,
        height: f32,
    }
    impl Harness {
        fn new() -> Self {
            let ctx = egui::Context::default();
            let mut c = Controller::new(spawn(true, ctx.clone()));
            c.read("demo://keyboard".into(), ReadChoice::Discard)
                .unwrap();
            c.process(
                c.port
                    .receiver
                    .recv_timeout(std::time::Duration::from_secs(2))
                    .unwrap(),
            );
            let mut h = Self {
                ctx,
                c,
                coefficient: 24,
                output: Default::default(),
                popup: false,
                height: 800.,
            };
            h.render(vec![]);
            h
        }
        fn render(&mut self, events: Vec<egui::Event>) {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(720., self.height),
                )),
                events,
                ..Default::default()
            };
            self.output = self.ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    show(
                        ui,
                        &mut self.c,
                        &mut self.coefficient,
                        Path::new("backup"),
                        true,
                        self.popup,
                    );
                });
            });
            self.output.textures_delta.clear();
        }
        fn texts(&self) -> Vec<(String, egui::Rect)> {
            self.output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Text(t) => Some((
                        t.galley.text().into(),
                        egui::Rect::from_min_size(t.pos, t.galley.size()),
                    )),
                    _ => None,
                })
                .collect()
        }
        fn rect(&self, label: &str) -> egui::Rect {
            self.texts()
                .into_iter()
                .find(|(s, _)| s == label)
                .unwrap_or_else(|| panic!("missing {label}: {:?}", self.texts()))
                .1
        }
        fn click(&mut self, label: &str, button: egui::PointerButton) {
            let pos = self.rect(label).center();
            for pressed in [true, false] {
                self.render(vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button,
                        pressed,
                        modifiers: Default::default(),
                    },
                ]);
            }
            self.render(vec![]);
        }
        fn expand(&mut self) {
            self.click("全体の反応（一時）", egui::PointerButton::Primary);
            for _ in 0..8 {
                self.render(vec![]);
            }
        }
    }
    // Catches accidentally expanding diagnostics or consuming vertical space before explicit opening.
    #[test]
    fn panel_is_compact_and_collapsed_by_default() {
        let h = Harness::new();
        assert!(h.texts().iter().any(|(t, _)| t == "全体の反応（一時）"));
        assert!(!h.texts().iter().any(|(t, _)| t == "適用"));
        assert!(h.c.pending.is_none());
    }
    // Catches reversing the meaning of the slider: left is c40, right is c10.
    #[test]
    fn ease_slider_endpoints_map_to_opposite_coefficients() {
        let mut h = Harness::new();
        h.expand();
        let left = h.rect("反応しにくい");
        let right = h.rect("反応しやすい");
        // Labels surround the slider; click immediately inside its track at both ends.
        for (pos, want) in [
            (egui::pos2(left.right() + 10., left.center().y), 40),
            (egui::pos2(right.left() - 10., right.center().y), 10),
        ] {
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
            assert_eq!(h.coefficient, want);
        }
    }
    // Catches losing the released-key instruction, hover dispatch, or changing the compact controls' geometry.
    #[test]
    fn apply_hover_instructs_release_without_dispatch_or_geometry_change() {
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let mut h = Harness::new();
            h.ctx.set_theme(theme);
            h.ctx
                .style_mut_of(theme, |s| s.interaction.tooltip_delay = 0.0);
            h.expand();
            h.render(vec![egui::Event::PointerGone]);
            let labels = ["適用", "反応しにくい", "反応しやすい"];
            let before = labels.map(|label| h.rect(label));
            let generation = h.c.generation;
            h.render(vec![egui::Event::PointerMoved(before[0].center())]);
            for _ in 0..8 {
                h.render(vec![]);
            }
            assert!(h.texts().iter().any(|(text, _)| text == "キーを離して適用してください。入力が一時停止します。"));
            for (label, rect) in labels.into_iter().zip(before) {
                assert_eq!(h.rect(label), rect, "hover moved {label}");
            }
            assert!(h.c.pending.is_none() && h.c.sensitivity.is_none() && h.c.live);
            assert_eq!(h.coefficient, 24);
            assert_eq!(h.c.generation, generation);
        }
    }
    // Catches keyboard/secondary/popup bypass while proving actual primary-pointer dispatch.
    #[test]
    fn apply_accepts_only_primary_pointer_and_blocks_popup() {
        let mut h = Harness::new();
        h.expand();
        h.click("適用", egui::PointerButton::Secondary);
        assert!(h.c.pending.is_none());
        h.ctx.enable_accesskit();
        h.render(vec![]);
        let node = h
            .output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find(|(_, n)| n.role() == egui::accesskit::Role::Button && n.label() == Some("適用"))
            .unwrap()
            .0;
        let button_id = unsafe { egui::Id::from_high_entropy_bits(node.0) };
        for key in [egui::Key::Enter, egui::Key::Space] {
            h.ctx.memory_mut(|m| m.request_focus(button_id));
            for pressed in [true, false] {
                h.render(vec![egui::Event::Key {
                    key,
                    physical_key: Some(key),
                    pressed,
                    repeat: false,
                    modifiers: Default::default(),
                }]);
            }
            assert_eq!(h.ctx.memory(|m| m.focused()), Some(button_id));
            assert!(h.c.pending.is_none());
        }
        h.render(vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Click,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: node,
                data: None,
            },
        )]);
        assert!(h.c.pending.is_none());
        h.popup = true;
        h.render(vec![]);
        h.click("適用", egui::PointerButton::Primary);
        assert!(h.c.pending.is_none());
        h.popup = false;
        h.render(vec![]);
        h.click("適用", egui::PointerButton::Primary);
        assert!(h.c.pending.is_some() && !h.c.live);
    }
    #[test]
    fn comparison_summary_does_not_guarantee_change_and_details_cover_every_physical_key() {
        let mut h = Harness::new();
        h.expand();
        h.click("適用", egui::PointerButton::Primary);
        h.height = 4000.;
        h.c.process(
            h.c.port
                .receiver
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap(),
        );
        h.render(vec![]);
        assert!(h.texts().iter().any(|(t, _)| t.contains("変化なし")));
        h.click("前後のしきい値・詳細", egui::PointerButton::Primary);
        for _ in 0..8 {
            h.render(vec![]);
        }
        assert!(h.texts().iter().any(|(t, _)| t.contains("DEMO")));
        assert_eq!(
            h.texts().iter().filter(|(t, _)| t.starts_with('#')).count(),
            66
        );
    }
}
