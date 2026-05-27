use super::data::*;
use super::occupations::*;
use super::ruleset::*;

use eframe::egui;
use egui::{Color32, RichText, Stroke};

use std::collections::{HashMap, HashSet};

pub(crate) fn apply_dark_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    visuals.panel_fill = BG;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = BG;
    visuals.faint_bg_color = PANEL_2;
    visuals.hyperlink_color = ACCENT;
    visuals.selection.bg_fill = ACCENT_DIM;

    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(18, 21, 29);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(26, 30, 41);
    visuals.widgets.active.bg_fill = ACCENT_DIM;

    ctx.set_visuals(visuals);
}

pub(crate) fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0_f32, LINE))
        .corner_radius(egui::CornerRadius::same(14))
        .inner_margin(egui::Margin::same(14))
        .show(ui, add)
        .inner
}

pub(crate) fn heading(ui: &mut egui::Ui, title: &str, description: &str) {
    ui.label(RichText::new(title).size(22.0).color(TEXT).strong());
    ui.label(RichText::new(description).color(MUTED));
    ui.add_space(10.0);
}

pub(crate) fn labeled_text(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).small().color(MUTED).strong());

        let width = ui.available_width().clamp(180.0, 320.0);
        ui.add_sized(
            [width, 28.0],
            egui::TextEdit::singleline(value).hint_text(hint),
        );
    });
}

pub(crate) fn labeled_i32(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut i32,
    min: i32,
    max: i32,
    speed: f64,
) -> egui::Response {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).small().color(MUTED).strong());
        ui.add(egui::DragValue::new(value).range(min..=max).speed(speed))
    })
    .inner
}

pub(crate) fn pill(ui: &mut egui::Ui, text: impl Into<String>, color: Color32) {
    let raw_text = text.into();
    let text = RichText::new(raw_text.clone())
        .small()
        .monospace()
        .color(color)
        .strong();

    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(
            color.r(),
            color.g(),
            color.b(),
            24,
        ))
        .stroke(Stroke::new(
            1.0_f32,
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 70),
        ))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            // Status chips are atomic: the chip may wrap to the next row,
            // but the text inside it must never wrap into one-character columns.
            // `truncate()` keeps the chip bounded, and hover text preserves the
            // full label when the available row width is small.
            ui.add(egui::Label::new(text).truncate())
                .on_hover_text(raw_text);
        });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CheckState {
    Pass,
    Warn,
    Fail,
}

pub(crate) fn rule_check(ui: &mut egui::Ui, state: CheckState, text: impl Into<String>) {
    let color = match state {
        CheckState::Pass => GREEN,
        CheckState::Warn => AMBER,
        CheckState::Fail => RED,
    };

    pill(ui, text, color);
}

pub(crate) fn stat_box(ui: &mut egui::Ui, label: &str, value: impl ToString, color: Color32) {
    egui::Frame::new()
        .fill(PANEL_2)
        .stroke(Stroke::new(1.0_f32, LINE))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(105.0);
            ui.label(RichText::new(label).small().monospace().color(MUTED));
            ui.label(
                RichText::new(value.to_string())
                    .size(21.0)
                    .monospace()
                    .strong()
                    .color(color),
            );
        });
}

pub(crate) fn dice_display(ui: &mut egui::Ui, result: &DiceResult, label: Option<&str>) {
    ui.horizontal_wrapped(|ui| {
        if let Some(label) = label {
            ui.label(RichText::new(label).small().monospace().color(MUTED));
        }

        for roll in &result.rolls {
            egui::Frame::new()
                .fill(Color32::from_rgb(16, 19, 27))
                .stroke(Stroke::new(1.0_f32, LINE))
                .corner_radius(egui::CornerRadius::same(7))
                .inner_margin(egui::Margin::same(6))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(roll.to_string())
                            .monospace()
                            .strong()
                            .color(TEXT),
                    );
                });
        }

        if result.plus_six {
            ui.label(RichText::new("+ 6").small().color(MUTED));
        }

        ui.label(
            RichText::new(format!("= {}", result.value))
                .monospace()
                .strong()
                .color(ACCENT),
        );
    });
}

pub(crate) fn floor_half(value: i32) -> i32 {
    value / 2
}

pub(crate) fn floor_fifth(value: i32) -> i32 {
    value / 5
}

pub(crate) fn snap_to_step(value: i32, step: i32) -> i32 {
    debug_assert!(step > 0);
    ((value + step / 2) / step) * step
}

pub(crate) fn clamp_step_5(value: i32, min: i32, max: i32) -> i32 {
    snap_to_step(value.clamp(min, max), 5).clamp(min, max)
}

pub(crate) fn max_physical_deduction_for_raw(raw: i32) -> i32 {
    ((raw - 1).max(0) / 5) * 5
}

pub(crate) fn empty_deductions_for(_bracket: AgeBracket) -> CharacteristicValues {
    CharacteristicValues::default()
}

pub(crate) fn get_age_bracket_index(age: i32) -> usize {
    debug_assert!(
        (15..=89).contains(&age),
        "age should be UI-clamped to 15..=89"
    );

    if age < AGE_BRACKETS[0].min {
        return 0;
    }

    AGE_BRACKETS
        .iter()
        .position(|bracket| age >= bracket.min && age <= bracket.max)
        .unwrap_or_else(|| AGE_BRACKETS.len() - 1)
}

#[cfg(test)]
pub(crate) fn get_age_bracket(age: i32) -> AgeBracket {
    AGE_BRACKETS[get_age_bracket_index(age)]
}

pub(crate) fn get_damage_bonus(strength: i32, size: i32) -> DamageRow {
    let total = strength + size;

    let first = *DB_BUILD_TABLE.first().expect("damage table is non-empty");
    let last = *DB_BUILD_TABLE.last().expect("damage table is non-empty");

    if total < first.min {
        return first;
    }

    DB_BUILD_TABLE
        .iter()
        .copied()
        .find(|row| total >= row.min && total <= row.max)
        .unwrap_or(last)
}

pub(crate) fn get_movement_rate(
    strength: i32,
    dexterity: i32,
    size: i32,
    age_bracket: AgeBracket,
) -> i32 {
    if strength == 0 || dexterity == 0 || size == 0 {
        return 0;
    }

    let mut mov = 8;

    if strength < size && dexterity < size {
        mov = 7;
    }

    if strength > size && dexterity > size {
        mov = 9;
    }

    (mov - age_bracket.mov_penalty).max(1)
}

pub(crate) fn characteristic_value(chars: &CharacteristicValues, key: Characteristic) -> i32 {
    chars.get_char(key)
}

pub(crate) fn get_base_skill_for(skill_id: Skill, chars: &CharacteristicValues) -> i32 {
    let spec = SKILL_SPECS
        .iter()
        .find(|skill| skill.id == skill_id)
        .expect("skill enum and SKILL_SPECS should stay synchronized");

    match spec.base {
        SkillBase::Fixed(value) => value,
        SkillBase::HalfDex => floor_half(characteristic_value(chars, Characteristic::Dex)),
        SkillBase::Edu => characteristic_value(chars, Characteristic::Edu),
    }
}

pub(crate) fn calculate_derived(
    chars: &CharacteristicValues,
    age_bracket: AgeBracket,
    mythos: i32,
) -> Derived {
    let val = |key: Characteristic| characteristic_value(chars, key);

    let hp = if val(Characteristic::Con) > 0 && val(Characteristic::Siz) > 0 {
        (val(Characteristic::Con) + val(Characteristic::Siz)) / 10
    } else {
        0
    };

    let san = val(Characteristic::Pow);

    let mp = if val(Characteristic::Pow) > 0 {
        floor_fifth(val(Characteristic::Pow))
    } else {
        0
    };

    let mov = get_movement_rate(
        val(Characteristic::Str),
        val(Characteristic::Dex),
        val(Characteristic::Siz),
        age_bracket,
    );

    let dodge = if val(Characteristic::Dex) > 0 {
        floor_half(val(Characteristic::Dex))
    } else {
        0
    };

    let db_row = if val(Characteristic::Str) > 0 && val(Characteristic::Siz) > 0 {
        get_damage_bonus(val(Characteristic::Str), val(Characteristic::Siz))
    } else {
        DamageRow {
            min: 0,
            max: 0,
            db: "—",
            build: 0,
        }
    };

    Derived {
        hp,
        san,
        max_san: (99 - mythos).max(0),
        mp,
        mov,
        dodge,
        db: db_row.db.to_owned(),
        build: db_row.build,
        major_wound: if hp > 0 { (hp + 1) / 2 } else { 0 },
    }
}

pub(crate) fn set_allocation(
    map: &mut HashMap<Skill, i32>,
    skill: Skill,
    value: i32,
    max_value: i32,
) {
    let value = value.clamp(0, max_value.clamp(0, MAX_CREATION_VALUE));

    if value == 0 {
        map.remove(&skill);
    } else {
        map.insert(skill, value);
    }
}

pub(crate) fn get_credit_tier(credit_rating: i32) -> &'static str {
    match credit_rating {
        i32::MIN..=0 => "Penniless",
        1..=9 => "Poor",
        10..=49 => "Average",
        50..=89 => "Wealthy",
        90..=98 => "Rich",
        _ => "Super Rich",
    }
}

#[cfg(windows)]
pub(crate) const LINE_SEP: &str = "\r\n";

#[cfg(not(windows))]
pub(crate) const LINE_SEP: &str = "\n";

pub(crate) fn push_line(out: &mut String, line: impl AsRef<str>) {
    out.push_str(line.as_ref());
    out.push_str(LINE_SEP);
}

pub(crate) fn push_blank_line(out: &mut String) {
    out.push_str(LINE_SEP);
}

fn sorted_skill_name_delta<'a>(left: &HashSet<&'a str>, right: &HashSet<&'a str>) -> Vec<&'a str> {
    let mut delta: Vec<&str> = left.difference(right).copied().collect();
    delta.sort_unstable();
    delta
}

fn format_skill_name_delta(names: &[&str]) -> String {
    if names.is_empty() {
        "none".to_owned()
    } else {
        names.join(", ")
    }
}

pub(crate) fn typed_skill_list_validation_errors(
    label: &str,
    string_list: &[&str],
    typed_list: &[Skill],
) -> Vec<String> {
    let mut errors = Vec::new();
    let string_names: HashSet<&str> = string_list.iter().copied().collect();
    let typed_names: HashSet<&str> = typed_list.iter().map(|skill| skill.name()).collect();

    if string_list.len() != string_names.len() {
        errors.push(format!(
            "{label} string skill list contains duplicate entries"
        ));
    }

    if typed_list.len() != typed_names.len() {
        errors.push(format!(
            "{label} typed skill list contains duplicate entries"
        ));
    }

    if typed_names != string_names {
        let missing = sorted_skill_name_delta(&string_names, &typed_names);
        let extra = sorted_skill_name_delta(&typed_names, &string_names);
        errors.push(format!(
            "{label} typed skill list must match its string skill list \
             (missing from typed list: {}; extra in typed list: {})",
            format_skill_name_delta(&missing),
            format_skill_name_delta(&extra)
        ));
    }

    errors
}

pub(crate) fn skill_constant_validation_errors() -> Vec<String> {
    let mut errors = Vec::new();
    let spec_names: HashSet<&str> = SKILL_SPECS.iter().map(|skill| skill.name).collect();
    let all_names: HashSet<&str> = ALL_SKILL_NAMES.iter().copied().collect();

    if ALL_SKILL_NAMES.len() != all_names.len() {
        errors.push("ALL_SKILL_NAMES contains duplicate entries".to_owned());
    }

    if SKILL_SPECS.len() != spec_names.len() {
        errors.push("SKILL_SPECS contains duplicate skill names".to_owned());
    }

    let spec_ids: HashSet<Skill> = SKILL_SPECS.iter().map(|skill| skill.id).collect();
    if SKILL_SPECS.len() != spec_ids.len() {
        errors.push("SKILL_SPECS contains duplicate skill ids".to_owned());
    }

    if all_names != spec_names {
        errors.push("ALL_SKILL_NAMES must match SKILL_SPECS exactly".to_owned());
    }

    for spec in SKILL_SPECS {
        if spec.id.name() != spec.name {
            errors.push(format!(
                "Skill enum variant `{}` does not match SKILL_SPECS entry `{}`",
                spec.id.name(),
                spec.name
            ));
        }

        if Skill::from_name(spec.name) != Some(spec.id) {
            errors.push(format!(
                "Skill::from_name does not resolve SKILL_SPECS entry `{}`",
                spec.name
            ));
        }
    }

    let selectable_expected: HashSet<&str> = spec_names
        .iter()
        .copied()
        .filter(|skill| *skill != "Credit Rating" && *skill != "Cthulhu Mythos")
        .collect();

    let selectable_actual: HashSet<&str> = OCCUPATION_SELECTABLE_SKILLS.iter().copied().collect();

    if OCCUPATION_SELECTABLE_SKILLS.len() != selectable_actual.len() {
        errors.push("OCCUPATION_SELECTABLE_SKILLS contains duplicates".to_owned());
    }

    if selectable_actual != selectable_expected {
        errors.push(
            "OCCUPATION_SELECTABLE_SKILLS must contain every non-Mythos, non-Credit skill"
                .to_owned(),
        );
    }

    errors.extend(typed_skill_list_validation_errors(
        "OCCUPATION_SELECTABLE_SKILL_OPTIONS",
        OCCUPATION_SELECTABLE_SKILLS,
        OCCUPATION_SELECTABLE_SKILL_OPTIONS,
    ));
    errors.extend(typed_skill_list_validation_errors(
        "ART_SKILL_OPTIONS",
        ART_SKILLS,
        ART_SKILL_OPTIONS,
    ));
    errors.extend(typed_skill_list_validation_errors(
        "SCIENCE_SKILL_OPTIONS",
        SCIENCE_SKILLS,
        SCIENCE_SKILL_OPTIONS,
    ));
    errors.extend(typed_skill_list_validation_errors(
        "INTERPERSONAL_SKILL_OPTIONS",
        INTERPERSONAL_SKILLS,
        INTERPERSONAL_SKILL_OPTIONS,
    ));
    errors.extend(typed_skill_list_validation_errors(
        "FIREARMS_SKILL_OPTIONS",
        FIREARMS_SKILLS,
        FIREARMS_SKILL_OPTIONS,
    ));

    for skill in ART_SKILLS {
        if !spec_names.contains(skill) {
            errors.push(format!("unknown art skill: {skill}"));
        }
        if !skill.starts_with("Art/Craft") {
            errors.push(format!("non-art skill in ART_SKILLS: {skill}"));
        }
    }

    for skill in SCIENCE_SKILLS {
        if !spec_names.contains(skill) {
            errors.push(format!("unknown science skill: {skill}"));
        }
        if !skill.starts_with("Science") {
            errors.push(format!("non-science skill in SCIENCE_SKILLS: {skill}"));
        }
    }

    for skill in INTERPERSONAL_SKILLS {
        if !spec_names.contains(skill) {
            errors.push(format!("unknown interpersonal skill: {skill}"));
        }
    }

    for skill in FIREARMS_SKILLS {
        if !spec_names.contains(skill) {
            errors.push(format!("unknown firearms skill: {skill}"));
        }
        if !skill.starts_with("Firearms") {
            errors.push(format!("non-firearms skill in FIREARMS_SKILLS: {skill}"));
        }
    }

    errors
}

#[cfg(test)]
pub(crate) fn validate_skill_constants() {
    let errors = skill_constant_validation_errors();
    assert!(
        errors.is_empty(),
        "skill constant validation failed:\n{}",
        errors.join("\n")
    );
}

pub(crate) fn unique_strings<I>(values: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for value in values {
        let value = value.trim().to_owned();

        if !value.is_empty() && seen.insert(value.clone()) {
            out.push(value);
        }
    }

    out
}
