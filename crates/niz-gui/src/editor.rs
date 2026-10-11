use niz_core::{Definition, Profile, key_name};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Reassign,
    Swap,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub layer: usize,
    pub from: usize,
    pub to_position: usize,
    pub to_code: u8,
    pub mode: Mode,
}
impl Default for Selection {
    fn default() -> Self {
        Self {
            layer: 0,
            from: 1,
            to_position: 2,
            to_code: 15,
            mode: Mode::Reassign,
        }
    }
}
pub struct Editor {
    pub baseline: Profile,
    pub draft: Profile,
    pub selection: Selection,
    output_ready: bool,
    undo: Vec<Profile>,
    redo: Vec<Profile>,
}
impl Editor {
    pub fn new(p: Profile) -> Self {
        let mut editor = Self {
            baseline: p.clone(),
            draft: p,
            selection: Selection::default(),
            output_ready: false,
            undo: vec![],
            redo: vec![],
        };
        editor.load_selected_output();
        editor
    }
    pub fn edit(&mut self) -> Result<(), String> {
        let p = self.proposed()?;
        self.commit(p);
        Ok(())
    }
    /// Selecting a slot opens its current draft setting without changing raw records or history.
    pub fn select_from(&mut self, slot: usize) -> bool {
        if slot >= 66 || self.selection.from == slot {
            return false;
        }
        self.selection.from = slot;
        self.load_selected_output();
        true
    }
    pub fn select_layer(&mut self, layer: usize) -> bool {
        if layer >= 3 || layer >= self.draft.group_count() || self.selection.layer == layer {
            return false;
        }
        self.selection.layer = layer;
        self.load_selected_output();
        true
    }
    fn load_selected_output(&mut self) {
        self.output_ready = false;
        if let Ok(definition) = self
            .draft
            .definition(self.selection.layer * 66 + self.selection.from)
            && definition.kind == 0
            && let [code] = definition.keys.as_slice()
            && selectable_output(*code)
        {
            self.selection.to_code = *code;
            self.output_ready = true;
        }
    }
    pub fn choose_output(&mut self, code: u8) {
        if selectable_output(code) {
            self.selection.to_code = code;
            self.output_ready = true;
        }
    }
    pub fn output_ready(&self) -> bool {
        self.output_ready
    }
    pub fn proposed(&self) -> Result<Profile, String> {
        if self.selection.mode == Mode::Reassign && !self.output_ready {
            return Err(
                "The current action is preserved. Choose an output explicitly to replace it."
                    .into(),
            );
        }
        regenerate(&self.draft, self.selection)
    }
    fn commit(&mut self, p: Profile) {
        if p.records != self.draft.records {
            self.undo.push(self.draft.clone());
            self.replace_draft(p);
            self.redo.clear();
        }
    }
    /// Record transitions reopen the selected setting only when its raw data changed.
    /// Unrelated edits leave a pending output choice intact.
    fn replace_draft(&mut self, p: Profile) {
        let index = self.selection.layer * 66 + self.selection.from;
        let selected_changed = self.draft.records.get(index) != p.records.get(index);
        self.draft = p;
        if selected_changed {
            self.load_selected_output();
        }
    }
    pub fn dirty(&self) -> bool {
        self.baseline.records != self.draft.records
    }
    pub fn undo(&mut self) -> bool {
        if let Some(p) = self.undo.pop() {
            self.redo.push(self.draft.clone());
            self.replace_draft(p);
            true
        } else {
            false
        }
    }
    pub fn redo(&mut self) -> bool {
        if let Some(p) = self.redo.pop() {
            self.undo.push(self.draft.clone());
            self.replace_draft(p);
            true
        } else {
            false
        }
    }
    pub fn import(&mut self, p: &Profile) -> Result<(), String> {
        let merged = self.baseline.merge_import(p).map_err(|e| e.to_string())?;
        self.commit(merged);
        Ok(())
    }
    pub fn preview(&self) -> Result<Vec<usize>, String> {
        self.baseline
            .differences(&self.draft)
            .map_err(|e| e.to_string())
    }
    pub fn accept_applied(&mut self, p: Profile) {
        self.baseline = p.clone();
        self.replace_draft(p);
        self.undo.clear();
        self.redo.clear();
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}
fn selectable_output(code: u8) -> bool {
    (1..=74).contains(&code) || [156, 166].contains(&code)
}
pub fn regenerate(p: &Profile, s: Selection) -> Result<Profile, String> {
    if s.layer >= 3 || s.from >= 66 || (s.mode == Mode::Swap && s.to_position >= 66) {
        return Err("Select a physical key on one of the three layers".into());
    }
    let mut draft = p.clone();
    let from = s.layer * 66 + s.from;
    match s.mode {
        Mode::Reassign => {
            if !((1..=177).contains(&s.to_code) || s.to_code == 199) {
                return Err("Unknown output action".into());
            }
            draft
                .set_definition(
                    from,
                    &Definition {
                        kind: 0,
                        keys: vec![s.to_code],
                        interval: 0,
                        cycles: 1,
                        custom_delay: false,
                        delays: vec![],
                    },
                )
                .map_err(|e| e.to_string())?;
        }
        Mode::Swap => draft
            .swap_keys(from, s.layer * 66 + s.to_position)
            .map_err(|e| e.to_string())?,
    }
    draft
        .validate_changes_against(p)
        .map_err(|e| e.to_string())?;
    Ok(draft)
}
pub fn action_label(p: &Profile, index: usize) -> String {
    match p.definition(index) {
        Ok(d) if d.kind == 0 && d.keys.is_empty() => "Unassigned".into(),
        Ok(d) if d.kind == 0 && d.keys.len() == 1 => {
            let code = d.keys[0];
            if (1..=177).contains(&code) || code == 199 {
                key_name(code).into()
            } else {
                format!("Preserved unknown ({code})")
            }
        }
        Ok(_) => "Advanced assignment (preserved)".into(),
        Err(_) => "Unsupported (preserved)".into(),
    }
}
/// Independent worker-side gate: only the exact operands regenerated from its own baseline.
/// Metadata is checked separately; raw padding, extra edits, imported identity and Fn columns fail closed.
pub fn exact_trial(baseline: &Profile, draft: &Profile, s: Selection) -> Result<Profile, String> {
    if baseline.firmware != "66EC(XRGB)BLe;V1.0.48;V1.0;"
        || baseline.identity
            != (niz_core::DeviceIdentity {
                vendor_id: 0x0483,
                product_id: 0x502a,
                usage_page: 0x8c,
                usage: 1,
            })
        || baseline.group_count() != 3
    {
        return Err(
            "Bounded hardware trials require the exact observed XRGB V1.0.48 target".into(),
        );
    }
    baseline.differences(draft).map_err(|e| e.to_string())?;
    if s.layer >= 3 || s.from >= 66 || (s.mode == Mode::Swap && s.to_position >= 66) {
        return Err("Invalid physical selection".into());
    }
    let mut indices = vec![s.layer * 66 + s.from];
    if s.mode == Mode::Swap {
        indices.push(s.layer * 66 + s.to_position);
    } else if !(1..=74).contains(&s.to_code) {
        return Err("Hardware trials support only basic output codes 1..74".into());
    }
    for index in indices {
        let d = baseline.definition(index).map_err(|e| e.to_string())?;
        if d.kind != 0 || d.keys.len() != 1 || !(1..=74).contains(&d.keys[0]) {
            return Err("The source must be a basic single key".into());
        }
        for layer in 0..3 {
            if baseline
                .definition(layer * 66 + index % 66)
                .map_err(|e| e.to_string())?
                .keys
                .iter()
                .any(|k| [156, 166].contains(k))
            {
                return Err("Fn-bearing physical columns are not qualified for trials".into());
            }
        }
    }
    let expected = regenerate(baseline, s)?;
    if expected.records != draft.records {
        return Err("The draft contains changes beyond the selected trial. Undo extra changes or save and reread before a trial.".into());
    }
    Ok(expected)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerGuide {
    CapturedShortcut,
    CustomMapping,
    UnverifiedTarget,
}
pub fn trigger_guide(profile: Option<&Profile>, live: bool) -> TriggerGuide {
    let Some(p) = profile else {
        return TriggerGuide::UnverifiedTarget;
    };
    if !live
        || p.firmware != "66EC(XRGB)BLe;V1.0.48;V1.0;"
        || p.identity
            != (niz_core::DeviceIdentity {
                vendor_id: 0x0483,
                product_id: 0x502a,
                usage_page: 0x8c,
                usage: 1,
            })
        || p.group_count() != 3
    {
        return TriggerGuide::UnverifiedTarget;
    }
    match p.definition(66 + 7) {
        Ok(d) if d.kind == 0 && d.keys == [149] => TriggerGuide::CapturedShortcut,
        _ => TriggerGuide::CustomMapping,
    }
}
