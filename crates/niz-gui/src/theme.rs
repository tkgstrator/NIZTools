use eframe::egui::{self, Color32, RichText};

/// Keep selectable controls' frame allocation identical before and during hover.
/// egui's unframed inactive variant drops the stroke width from its margin.
pub(crate) fn choice<'a>(
    ui: &mut egui::Ui,
    selected: bool,
    label: impl egui::IntoAtoms<'a>,
) -> egui::Response {
    ui.scope(|ui| {
        if !selected {
            let inactive = &mut ui.style_mut().visuals.widgets.inactive;
            inactive.bg_fill = Color32::TRANSPARENT;
            inactive.weak_bg_fill = Color32::TRANSPARENT;
            // Retain the stroke width, including when its color is transparent.
            inactive.bg_stroke.color = Color32::TRANSPARENT;
        }
        ui.add(egui::Button::selectable(selected, label).frame_when_inactive(true))
    })
    .inner
}

pub(crate) fn choice_value<'a, Value: PartialEq>(
    ui: &mut egui::Ui,
    current: &mut Value,
    alternative: Value,
    label: impl egui::IntoAtoms<'a>,
) -> egui::Response {
    let mut response = choice(ui, *current == alternative, label);
    if response.clicked() && *current != alternative {
        *current = alternative;
        response.mark_changed();
    }
    response
}

/// Presentation roles for the effective frame theme. Hardware and editor state
/// never depend on appearance preferences.
#[derive(Clone, Copy)]
pub(crate) struct Palette {
    pub root: Color32,
    pub card: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub control: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub control_border: Color32,
    pub input: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub accent_text: Color32,
    pub warning: Color32,
    pub dirty: Color32,
    pub from_fill: Color32,
    pub from_border: Color32,
    pub from_text: Color32,
    pub to_fill: Color32,
    pub to_border: Color32,
    pub to_text: Color32,
    pub input_fill: Color32,
    pub input_border: Color32,
    pub input_text: Color32,
    pub danger: Color32,
    pub on_danger: Color32,
}
impl Palette {
    pub fn of(ui: &egui::Ui) -> Self {
        Self::for_dark(ui.visuals().dark_mode)
    }
    pub fn for_dark(dark: bool) -> Self {
        let rgb = Color32::from_rgb;
        if dark {
            Self {
                root: rgb(20, 27, 32),
                card: rgb(29, 39, 46),
                border: rgb(56, 72, 82),
                text: rgb(230, 239, 243),
                muted: rgb(169, 187, 197),
                control: rgb(37, 50, 59),
                hover: rgb(46, 65, 74),
                pressed: rgb(58, 79, 89),
                control_border: rgb(148, 174, 188),
                input: rgb(20, 29, 35),
                accent: rgb(24, 102, 91),
                on_accent: Color32::WHITE,
                accent_text: rgb(101, 216, 194),
                warning: rgb(244, 188, 108),
                dirty: rgb(245, 171, 85),
                from_fill: rgb(25, 62, 54),
                from_border: rgb(81, 193, 158),
                from_text: rgb(172, 243, 215),
                to_fill: rgb(67, 51, 25),
                to_border: rgb(233, 185, 81),
                to_text: rgb(255, 219, 149),
                input_fill: rgb(25, 57, 92),
                input_border: rgb(108, 178, 249),
                input_text: rgb(208, 231, 255),
                danger: rgb(209, 57, 68),
                on_danger: Color32::WHITE,
            }
        } else {
            Self {
                root: rgb(240, 244, 246),
                card: Color32::WHITE,
                border: rgb(210, 221, 226),
                text: rgb(34, 48, 56),
                muted: rgb(82, 102, 113),
                control: rgb(243, 247, 249),
                hover: rgb(227, 240, 238),
                pressed: rgb(211, 229, 227),
                control_border: rgb(104, 124, 135),
                input: rgb(249, 251, 252),
                accent: rgb(22, 100, 90),
                on_accent: Color32::WHITE,
                accent_text: rgb(22, 100, 90),
                warning: rgb(146, 76, 15),
                dirty: rgb(169, 77, 14),
                from_fill: rgb(220, 243, 233),
                from_border: rgb(28, 111, 83),
                from_text: rgb(19, 78, 58),
                to_fill: rgb(255, 244, 214),
                to_border: rgb(143, 98, 16),
                to_text: rgb(107, 69, 7),
                input_fill: rgb(218, 236, 255),
                input_border: rgb(33, 96, 158),
                input_text: rgb(18, 57, 98),
                danger: rgb(178, 35, 49),
                on_danger: Color32::WHITE,
            }
        }
    }
    pub fn card(self) -> egui::Frame {
        egui::Frame::new()
            .fill(self.card)
            .stroke(egui::Stroke::new(1., self.border))
            .corner_radius(8.)
            .inner_margin(10.)
    }
    pub fn primary_button(self, label: &str) -> egui::Button<'_> {
        egui::Button::new(RichText::new(label).color(self.on_accent)).fill(self.accent)
    }
    pub fn danger_button(self, label: &str) -> egui::Button<'_> {
        egui::Button::new(RichText::new(label).color(self.on_danger))
            .fill(self.danger)
            .stroke(egui::Stroke::new(1., self.danger))
    }
}

pub(crate) fn install(ctx: &egui::Context) {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let p = Palette::for_dark(theme == egui::Theme::Dark);
        let mut style = theme.default_style();
        style.spacing.item_spacing = egui::vec2(8., 4.);
        style.spacing.interact_size.y = 26.;
        style.spacing.button_padding = egui::vec2(10., 4.);
        for (role, size) in [
            (egui::TextStyle::Body, 14.),
            (egui::TextStyle::Button, 14.),
            (egui::TextStyle::Small, 12.),
            (egui::TextStyle::Heading, 18.),
        ] {
            style
                .text_styles
                .insert(role, egui::FontId::proportional(size));
        }
        let v = &mut style.visuals;
        v.override_text_color = None;
        v.weak_text_color = Some(p.muted);
        // Nested disabled scopes remain legible while state stays visibly muted.
        v.disabled_alpha = 0.8;
        v.panel_fill = p.root;
        v.window_fill = p.card;
        v.window_stroke = egui::Stroke::new(1., p.border);
        v.faint_bg_color = p.control;
        v.extreme_bg_color = p.input;
        v.text_edit_bg_color = Some(p.input);
        v.selection.bg_fill = p.accent;
        v.selection.stroke = egui::Stroke::new(1., p.on_accent);
        v.warn_fg_color = p.warning;
        v.error_fg_color = p.warning;
        v.hyperlink_color = p.accent_text;
        for widget in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            widget.fg_stroke = egui::Stroke::new(1., p.text);
            widget.bg_stroke = egui::Stroke::new(1., p.control_border);
        }
        v.widgets.noninteractive.bg_fill = p.card;
        v.widgets.noninteractive.weak_bg_fill = p.card;
        v.widgets.inactive.bg_fill = p.control;
        v.widgets.inactive.weak_bg_fill = p.control;
        v.widgets.hovered.bg_fill = p.hover;
        v.widgets.hovered.weak_bg_fill = p.hover;
        // egui also uses active.fg_stroke for ordinary strong labels.
        v.widgets.active.bg_fill = p.pressed;
        v.widgets.active.weak_bg_fill = p.pressed;
        v.widgets.open.bg_fill = p.hover;
        v.widgets.open.weak_bg_fill = p.hover;
        ctx.set_style_of(theme, style);
    }
    ctx.set_theme(egui::ThemePreference::System);
}
