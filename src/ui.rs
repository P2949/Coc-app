use super::data::*;
use super::models::*;
use super::occupations::*;
use super::ruleset::*;
use eframe::egui;
use egui::RichText;
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "ui/characteristics.rs"]
mod characteristics;
#[path = "ui/concept.rs"]
mod concept;
#[path = "ui/occupation.rs"]
mod occupation;
#[path = "ui/skills.rs"]
mod skills;
#[path = "ui/story_summary.rs"]
mod story_summary;

pub fn run() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([APP_INITIAL_WIDTH, APP_INITIAL_HEIGHT])
            .with_min_inner_size([APP_MIN_WINDOW_WIDTH, APP_MIN_WINDOW_HEIGHT]),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "CoC7e Investigator Creator",
        options,
        Box::new(|cc| Ok(Box::new(CoC7eApp::new(cc)))),
    )
}

fn adjusted_final_characteristic(raw: i32, delta: i32) -> i32 {
    if raw <= 0 {
        0
    } else {
        (raw + delta).clamp(1, MAX_CREATION_VALUE)
    }
}

fn skill_accepts_personal_points(skill: &str) -> bool {
    skill != "Credit Rating" && skill != "Cthulhu Mythos"
}

fn skill_accepts_occupation_points(skill: &str, allowed: &HashSet<String>) -> bool {
    allowed.contains(skill)
}

fn sanitized_allocation_value(
    allocations: &HashMap<String, i32>,
    skill: &str,
    max_value: i32,
) -> i32 {
    allocations
        .get(skill)
        .copied()
        .unwrap_or(0)
        .clamp(0, max_value.clamp(0, MAX_CREATION_VALUE))
}

impl CoC7eApp {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_dark_theme(&cc.egui_ctx);

        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(0xC0C7_E7E5_1234_5678);

        let occupations = build_occupations();
        let mut startup_validation_errors = skill_constant_validation_errors();
        startup_validation_errors.extend(occupation_validation_errors(&occupations));

        let mut app = Self::fresh(occupations, seed | 1);
        app.startup_validation_errors = startup_validation_errors;
        app
    }

    pub(crate) fn fresh(occupations: Vec<Occupation>, rng_state: u64) -> Self {
        let age_index = get_age_bracket_index(30);

        Self {
            step: 1,
            concept: Concept::default(),
            char_method: CharMethod::Roll,
            chars: CharacteristicValues::default(),
            char_rolls: HashMap::new(),
            luck_state: LuckState::default(),
            age_deductions: empty_deductions_for(AGE_BRACKETS[age_index]),
            edu_bonus: 0,
            edu_check_rolls: Vec::new(),
            occupation_id: String::new(),
            formula_key: FormulaKey::Edu4,
            occupation_choices: HashMap::new(),
            custom_occupation: CustomOccupation::default(),
            allocations: AllocationState::default(),
            backstory: HashMap::new(),
            import_json_text: String::new(),
            save_load_message: None,
            occupations,
            startup_validation_errors: Vec::new(),
            last_age_bracket_index: age_index,
            frame_max_reachable_step: 2,
            rng: AppRng::seeded(rng_state),
        }
    }

    pub(crate) fn save_file(&self) -> InvestigatorSaveFile {
        let mut occupation_choices: Vec<SavedOccupationChoice> = self
            .occupation_choices
            .iter()
            .map(|(key, value)| SavedOccupationChoice {
                id: key.id.clone(),
                index: key.index,
                value: value.clone(),
            })
            .collect();
        occupation_choices.sort_by(|left, right| {
            left.id
                .cmp(&right.id)
                .then_with(|| left.index.cmp(&right.index))
        });

        InvestigatorSaveFile {
            version: INVESTIGATOR_SAVE_VERSION,
            concept: self.concept.clone(),
            char_method: self.char_method,
            chars: self.chars.clone(),
            char_rolls: self.char_rolls.clone(),
            luck_state: self.luck_state.clone(),
            age_deductions: self.age_deductions.clone(),
            edu_bonus: self.edu_bonus,
            edu_check_rolls: self.edu_check_rolls.clone(),
            occupation_id: self.occupation_id.clone(),
            formula_key: self.formula_key,
            occupation_choices,
            custom_occupation: self.custom_occupation.clone(),
            allocations: self.allocations.clone(),
            backstory: self.backstory.clone(),
        }
    }

    pub(crate) fn export_json_save(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.save_file())
    }

    pub(crate) fn import_json_save(&mut self, input: &str) -> Result<(), String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err("paste a JSON save before loading".to_owned());
        }

        let save: InvestigatorSaveFile = serde_json::from_str(trimmed)
            .map_err(|error| format!("could not parse JSON save: {error}"))?;

        if save.version != INVESTIGATOR_SAVE_VERSION {
            return Err(format!(
                "unsupported save version {}; this app supports version {INVESTIGATOR_SAVE_VERSION}",
                save.version
            ));
        }

        self.concept = save.concept;
        self.concept.age = self.concept.age.clamp(15, 89);
        self.last_age_bracket_index = get_age_bracket_index(self.concept.age);
        self.char_method = save.char_method;
        self.chars = save.chars;
        self.char_rolls = save.char_rolls;
        self.luck_state = save.luck_state;
        self.age_deductions = save.age_deductions;
        self.edu_bonus = save.edu_bonus.clamp(0, MAX_CREATION_VALUE);
        self.edu_check_rolls = save.edu_check_rolls;
        self.occupation_id = if save.occupation_id == CUSTOM_OCCUPATION_ID
            || self
                .occupations
                .iter()
                .any(|occupation| occupation.name == save.occupation_id)
        {
            save.occupation_id
        } else {
            String::new()
        };
        self.formula_key = save.formula_key;
        self.occupation_choices = save
            .occupation_choices
            .into_iter()
            .filter_map(|choice| {
                let value = choice.value.trim().to_owned();
                if value.is_empty() {
                    None
                } else {
                    Some((ChoiceKey::new(choice.id, choice.index), value))
                }
            })
            .collect();
        self.custom_occupation = save.custom_occupation;
        self.allocations = save.allocations;
        let allowed_backstory: HashSet<&str> = BACKSTORY_CATEGORIES.iter().copied().collect();
        self.backstory = save
            .backstory
            .into_iter()
            .filter_map(|(category, value)| {
                let category = category.trim();
                if !allowed_backstory.contains(category) || value.trim().is_empty() {
                    None
                } else {
                    Some((category.to_owned(), value))
                }
            })
            .collect();

        let bracket = self.age_bracket();
        self.edu_check_rolls.truncate(bracket.edu_checks);
        self.luck_state.value = self.luck_state.value.map(|value| value.clamp(1, 99));
        self.sanitize_state();
        self.refresh_reachability();
        Ok(())
    }

    pub(crate) fn sync_age_bracket(&mut self) {
        let index = get_age_bracket_index(self.concept.age);
        if self.last_age_bracket_index != index {
            self.age_deductions = empty_deductions_for(AGE_BRACKETS[index]);
            self.edu_bonus = 0;
            self.edu_check_rolls.clear();
            self.luck_state.value = None;
            self.luck_state.rolls.clear();
            self.last_age_bracket_index = index;
        }
    }

    pub(crate) fn set_age(&mut self, age: i32) {
        self.concept.age = age.clamp(15, 89);
        self.sync_age_bracket();
        self.sanitize_age_deductions();
        self.refresh_reachability();
    }

    pub(crate) fn age_bracket(&self) -> AgeBracket {
        AGE_BRACKETS[self.last_age_bracket_index]
    }

    pub(crate) fn roll_die(&mut self, sides: u32) -> u32 {
        self.rng.roll_inclusive(sides)
    }

    pub(crate) fn roll_characteristic(&mut self, def: CharacteristicDef) -> DiceResult {
        match def.dice {
            DiceKind::ThreeD6 => {
                let rolls = vec![self.roll_die(6), self.roll_die(6), self.roll_die(6)];
                let raw = rolls.iter().sum::<u32>();
                DiceResult {
                    rolls,
                    plus_six: false,
                    value: (raw as i32) * 5,
                    kept: None,
                }
            }
            DiceKind::TwoD6Plus6 => {
                let rolls = vec![self.roll_die(6), self.roll_die(6)];
                let raw = rolls.iter().sum::<u32>() + 6;
                DiceResult {
                    rolls,
                    plus_six: true,
                    value: (raw as i32) * 5,
                    kept: None,
                }
            }
        }
    }

    pub(crate) fn roll_luck_attempt(&mut self) -> DiceResult {
        let rolls = vec![self.roll_die(6), self.roll_die(6), self.roll_die(6)];
        let raw = rolls.iter().sum::<u32>();
        DiceResult {
            rolls,
            plus_six: false,
            value: (raw as i32) * 5,
            kept: None,
        }
    }

    pub(crate) fn roll_luck(&mut self) {
        let bracket = self.age_bracket();
        let mut attempts = vec![self.roll_luck_attempt()];
        if bracket.luck_advantage {
            attempts.push(self.roll_luck_attempt());
        }

        let best_index = attempts
            .iter()
            .enumerate()
            .max_by_key(|(_, roll)| roll.value)
            .map(|(index, _)| index)
            .unwrap_or(0);

        for (index, roll) in attempts.iter_mut().enumerate() {
            roll.kept = Some(index == best_index);
        }

        self.luck_state.value = attempts.get(best_index).map(|roll| roll.value);
        self.luck_state.rolls = attempts;
    }

    pub(crate) fn clear_edu_age_checks(&mut self) {
        self.edu_bonus = 0;
        self.edu_check_rolls.clear();
    }

    pub(crate) fn set_char_value(&mut self, key: &str, value: i32) {
        if let Some(def) = CHARACTERISTICS.iter().find(|item| item.key.key() == key) {
            let next_value = clamp_step_5(value, def.min, def.max);
            let old_value = Some(self.chars.get_char(def.key));
            let had_roll = self.char_rolls.remove(key).is_some();

            if old_value == Some(next_value) {
                if had_roll {
                    self.char_method = CharMethod::Mixed;
                    if key == "EDU" {
                        self.clear_edu_age_checks();
                    }
                }
                return;
            }

            self.chars.set_char(def.key, next_value);

            if had_roll || matches!(self.char_method, CharMethod::Roll | CharMethod::QuickArray) {
                self.char_method = CharMethod::Mixed;
            }

            if key == "EDU" {
                self.clear_edu_age_checks();
            }
            self.sanitize_age_deductions();
        }
    }

    pub(crate) fn char_value(&self, key: &str) -> i32 {
        Characteristic::from_key(key).map_or(0, |id| self.chars.get_char(id))
    }

    pub(crate) fn final_chars(&self) -> CharacteristicValues {
        let bracket = self.age_bracket();
        let mut out = self.chars.clone();

        for key in bracket.physical_from {
            let value = out.get_char(*key);
            let deduction = self.effective_physical_deduction_for(*key);
            out.set_char(*key, adjusted_final_characteristic(value, -deduction));
        }

        let app = out.get_char(Characteristic::App);
        out.set_char(
            Characteristic::App,
            adjusted_final_characteristic(app, -bracket.app_penalty),
        );

        let edu = out.get_char(Characteristic::Edu);
        out.set_char(
            Characteristic::Edu,
            adjusted_final_characteristic(edu, -bracket.edu_penalty + self.edu_bonus),
        );

        out
    }

    pub(crate) fn selected_occupation(&self) -> Option<Occupation> {
        if self.occupation_id == CUSTOM_OCCUPATION_ID {
            return Some(self.build_custom_occupation());
        }
        self.occupations
            .iter()
            .find(|occ| occ.name == self.occupation_id)
            .cloned()
    }

    pub(crate) fn selected_occupation_name(&self) -> String {
        if self.occupation_id == CUSTOM_OCCUPATION_ID {
            let trimmed = self.custom_occupation.name.trim();
            if trimmed.is_empty() {
                "Custom Occupation".to_owned()
            } else {
                trimmed.to_owned()
            }
        } else if self
            .occupations
            .iter()
            .any(|occupation| occupation.name == self.occupation_id)
        {
            self.occupation_id.clone()
        } else {
            "No occupation".to_owned()
        }
    }

    pub(crate) fn normalize_custom_occupation_skills(&mut self) {
        self.custom_occupation
            .skills
            .resize(CUSTOM_OCCUPATION_SKILL_COUNT, String::new());
        self.custom_occupation
            .skills
            .truncate(CUSTOM_OCCUPATION_SKILL_COUNT);
    }

    fn occupation_choice_slots<'a>(
        &self,
        occupation: &'a Occupation,
    ) -> Vec<(ChoiceKey, &'a [String])> {
        let mut slots = Vec::new();

        for slot in &occupation.slots {
            if let Slot::Choice {
                id, options, count, ..
            } = slot
            {
                for index in 0..*count {
                    slots.push((ChoiceKey::new(id.clone(), index), options.as_slice()));
                }
            }
        }

        slots
    }

    fn valid_choice_values_for(&self, occupation: &Occupation) -> Vec<String> {
        let mut used: HashSet<String> = fixed_skill_set_for(occupation)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut out = Vec::new();

        for (key, options) in self.occupation_choice_slots(occupation) {
            let Some(value) = self.occupation_choices.get(&key) else {
                continue;
            };
            let value = value.trim();
            if choice_value_is_valid(options, value) && used.insert(value.to_owned()) {
                out.push(value.to_owned());
            }
        }

        out
    }

    pub(crate) fn prune_occupation_choices_for(&mut self, occupation: &Occupation) {
        let mut used: HashSet<String> = fixed_skill_set_for(occupation)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut cleaned = HashMap::new();

        for (key, options) in self.occupation_choice_slots(occupation) {
            let Some(value) = self.occupation_choices.get(&key) else {
                continue;
            };
            let value = value.trim();
            if choice_value_is_valid(options, value) && used.insert(value.to_owned()) {
                cleaned.insert(key, value.to_owned());
            }
        }

        self.occupation_choices = cleaned;
    }

    pub(crate) fn build_custom_occupation(&self) -> Occupation {
        let min = self.custom_occupation.credit_min.clamp(0, 99);
        let max = self.custom_occupation.credit_max.clamp(0, 99);
        let skills = unique_strings(
            self.custom_occupation
                .skills
                .iter()
                .take(CUSTOM_OCCUPATION_SKILL_COUNT)
                .map(|skill| skill.trim().to_owned())
                .filter(|skill| {
                    !skill.is_empty() && OCCUPATION_SELECTABLE_SKILLS.contains(&skill.as_str())
                }),
        );

        Occupation {
            name: self.selected_occupation_name(),
            credit: (min.min(max), min.max(max)),
            formula_keys: vec![self.custom_occupation.formula_key],
            slots: skills.into_iter().map(Slot::Skill).collect(),
        }
    }

    pub(crate) fn set_occupation(&mut self, next_id: String) {
        self.occupation_id = if next_id == CUSTOM_OCCUPATION_ID
            || self
                .occupations
                .iter()
                .any(|occupation| occupation.name == next_id)
        {
            next_id
        } else {
            String::new()
        };
        self.formula_key = if self.occupation_id == CUSTOM_OCCUPATION_ID {
            self.custom_occupation.formula_key
        } else {
            self.occupations
                .iter()
                .find(|occ| occ.name == self.occupation_id)
                .and_then(|occ| occ.formula_keys.first().copied())
                .unwrap_or(FormulaKey::Edu4)
        };
        self.occupation_choices.clear();
        self.allocations.occupation_points.clear();
    }

    pub(crate) fn set_formula_key(&mut self, next: FormulaKey) {
        let selected_occupation = self.selected_occupation();
        self.formula_key = match selected_occupation.as_ref() {
            Some(occupation) if occupation.formula_keys.contains(&next) => next,
            Some(occupation) => occupation
                .formula_keys
                .first()
                .copied()
                .unwrap_or(FormulaKey::Edu4),
            None => FormulaKey::Edu4,
        };
    }

    pub(crate) fn set_custom_formula_key(&mut self, next: FormulaKey) {
        self.custom_occupation.formula_key = next;
        if self.occupation_id == CUSTOM_OCCUPATION_ID {
            self.formula_key = next;
        }
    }

    pub(crate) fn set_custom_occupation_name(&mut self, next: String) {
        self.custom_occupation.name = next;
    }

    pub(crate) fn set_custom_occupation_credit_min(&mut self, next: i32) {
        self.custom_occupation.credit_min = next.clamp(0, 99);
    }

    pub(crate) fn set_custom_occupation_credit_max(&mut self, next: i32) {
        self.custom_occupation.credit_max = next.clamp(0, 99);
    }

    pub(crate) fn set_custom_occupation_skill(&mut self, index: usize, next: String) -> bool {
        self.normalize_custom_occupation_skills();
        if index >= CUSTOM_OCCUPATION_SKILL_COUNT {
            return false;
        }

        let normalized = next.trim().to_owned();
        if normalized.is_empty() {
            self.custom_occupation.skills[index].clear();
            self.prune_occupation_allocations();
            return true;
        }
        if !OCCUPATION_SELECTABLE_SKILLS.contains(&normalized.as_str()) {
            return false;
        }
        if self
            .custom_occupation
            .skills
            .iter()
            .enumerate()
            .any(|(other_index, value)| other_index != index && value.trim() == normalized.as_str())
        {
            return false;
        }

        self.custom_occupation.skills[index] = normalized;
        self.prune_occupation_allocations();
        true
    }

    pub(crate) fn set_occupation_choice(&mut self, key: ChoiceKey, next: String) -> bool {
        let Some(occupation) = self.selected_occupation() else {
            return false;
        };
        let normalized = next.trim().to_owned();
        if normalized.is_empty() {
            self.occupation_choices.remove(&key);
            self.prune_occupation_allocations();
            return true;
        }

        let Some((_, options)) = self
            .occupation_choice_slots(&occupation)
            .into_iter()
            .find(|(choice_key, _)| choice_key == &key)
        else {
            return false;
        };

        if !choice_value_is_valid(options, &normalized) {
            return false;
        }
        if fixed_skill_set_for(&occupation).contains(normalized.as_str()) {
            return false;
        }
        if self
            .occupation_choices
            .iter()
            .any(|(choice_key, value)| choice_key != &key && value.trim() == normalized)
        {
            return false;
        }

        self.occupation_choices.insert(key, normalized);
        self.prune_occupation_allocations();
        true
    }

    pub(crate) fn resolved_occupation_skills_for(&self, occupation: &Occupation) -> Vec<String> {
        let mut resolved = Vec::new();
        for slot in &occupation.slots {
            if let Slot::Skill(name) = slot {
                resolved.push(name.clone());
            }
        }
        resolved.extend(self.valid_choice_values_for(occupation));
        unique_strings(resolved)
    }

    pub(crate) fn occupation_skill_set_for(
        &self,
        occupation: Option<&Occupation>,
    ) -> HashSet<String> {
        occupation.map_or_else(HashSet::new, |occupation| {
            let mut set: HashSet<String> = self
                .resolved_occupation_skills_for(occupation)
                .into_iter()
                .collect();
            set.insert("Credit Rating".to_owned());
            set
        })
    }

    pub(crate) fn unresolved_choice_count_for(&self, occupation: &Occupation) -> usize {
        let mut used: HashSet<String> = fixed_skill_set_for(occupation)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut unresolved = 0;

        for (key, options) in self.occupation_choice_slots(occupation) {
            let Some(value) = self.occupation_choices.get(&key) else {
                unresolved += 1;
                continue;
            };
            let value = value.trim();
            if !choice_value_is_valid(options, value) || !used.insert(value.to_owned()) {
                unresolved += 1;
            }
        }

        unresolved
    }

    pub(crate) fn occupation_slot_count_for(&self, occupation: &Occupation) -> usize {
        occupation
            .slots
            .iter()
            .map(|slot| match slot {
                Slot::Skill(_) => 1,
                Slot::Choice { count, .. } => *count,
            })
            .sum()
    }

    pub(crate) fn required_occupation_skill_count_for(&self, occupation: &Occupation) -> usize {
        // A custom occupation's built Occupation only contains filled unique skills,
        // but the creator still requires all eight custom skill slots to be filled.
        if self.occupation_id == CUSTOM_OCCUPATION_ID {
            CUSTOM_OCCUPATION_SKILL_COUNT
        } else {
            self.occupation_slot_count_for(occupation)
        }
    }

    pub(crate) fn unique_occupation_shortfall_for(&self, occupation: &Occupation) -> usize {
        self.required_occupation_skill_count_for(occupation)
            .saturating_sub(self.resolved_occupation_skills_for(occupation).len())
    }

    pub(crate) fn skill_rows_for(
        &self,
        final_chars: &CharacteristicValues,
        occupation_skill_set: &HashSet<String>,
    ) -> Vec<SkillRow> {
        SKILL_SPECS
            .iter()
            .map(|skill| {
                let base = get_base_skill(skill.name, final_chars);
                let occ_add = if skill_accepts_occupation_points(skill.name, occupation_skill_set) {
                    sanitized_allocation_value(
                        &self.allocations.occupation_points,
                        skill.name,
                        MAX_CREATION_VALUE - base,
                    )
                } else {
                    0
                };
                let personal_add = if skill_accepts_personal_points(skill.name) {
                    sanitized_allocation_value(
                        &self.allocations.personal_points,
                        skill.name,
                        MAX_CREATION_VALUE - base - occ_add,
                    )
                } else {
                    0
                };
                SkillRow {
                    name: skill.name.to_owned(),
                    base,
                    occ_add,
                    personal_add,
                    total: base + occ_add + personal_add,
                }
            })
            .collect()
    }

    pub(crate) fn derived_for(
        &self,
        final_chars: &CharacteristicValues,
        skill_rows: &[SkillRow],
    ) -> Derived {
        let mythos = skill_rows
            .iter()
            .find(|row| row.name == "Cthulhu Mythos")
            .map(|row| row.total)
            .unwrap_or(0);
        calculate_derived(final_chars, self.age_bracket(), mythos)
    }

    pub(crate) fn sheet_math(&self) -> SheetMath {
        self.sheet_math_from(self.final_chars())
    }

    pub(crate) fn sheet_math_from(&self, final_chars: CharacteristicValues) -> SheetMath {
        let selected_occupation = self.selected_occupation();
        let occupation_skill_set = self.occupation_skill_set_for(selected_occupation.as_ref());
        let skill_rows = self.skill_rows_for(&final_chars, &occupation_skill_set);
        let derived = self.derived_for(&final_chars, &skill_rows);
        let credit_range = selected_occupation
            .as_ref()
            .map_or((0, 99), |occupation| occupation.credit);
        let unresolved_choices = selected_occupation
            .as_ref()
            .map_or(0, |occupation| self.unresolved_choice_count_for(occupation));
        let occupation_shortfall = selected_occupation.as_ref().map_or(0, |occupation| {
            self.unique_occupation_shortfall_for(occupation)
        });
        let occupation_budget = self
            .active_formula_key_for(selected_occupation.as_ref())
            .calculate(&final_chars);
        let personal_budget = self.personal_budget_for(&final_chars);
        let credit_rating = self.credit_rating_for_with_skills(&final_chars, &occupation_skill_set);

        SheetMath {
            final_chars,
            skill_rows,
            derived,
            selected_occupation,
            credit_range,
            unresolved_choices,
            occupation_shortfall,
            occupation_skill_set,
            occupation_budget,
            personal_budget,
            credit_rating,
        }
    }

    pub(crate) fn point_buy_spent(&self) -> i32 {
        CHARACTERISTICS
            .iter()
            .map(|def| self.chars.get_char(def.key))
            .sum()
    }

    pub(crate) fn has_all_chars(&self) -> bool {
        CHARACTERISTICS
            .iter()
            .all(|def| self.chars.get_char(def.key) > 0)
    }

    pub(crate) fn assigned_physical_deduction_total(&self) -> i32 {
        self.age_bracket()
            .physical_from
            .iter()
            .map(|key| self.age_deductions.get_char(*key))
            .sum()
    }

    pub(crate) fn effective_physical_deduction_for(&self, key: Characteristic) -> i32 {
        let assigned = self.age_deductions.get_char(key);
        let max_effective = max_physical_deduction_for_raw(self.chars.get_char(key));
        clamp_step_5(assigned, 0, max_effective)
    }

    pub(crate) fn physical_deduction_total(&self) -> i32 {
        self.age_bracket()
            .physical_from
            .iter()
            .map(|key| self.effective_physical_deduction_for(*key))
            .sum()
    }

    pub(crate) fn max_possible_physical_deduction(&self) -> i32 {
        self.age_bracket()
            .physical_from
            .iter()
            .map(|key| max_physical_deduction_for_raw(self.chars.get_char(*key)))
            .sum()
    }

    pub(crate) fn physical_deduction_is_possible(&self) -> bool {
        self.max_possible_physical_deduction() >= self.age_bracket().physical_deduct
    }

    pub(crate) fn physical_deduction_is_complete(&self) -> bool {
        self.physical_deduction_total() == self.age_bracket().physical_deduct
    }

    pub(crate) fn max_age_deduction_for(&self, key: Characteristic) -> i32 {
        let bracket = self.age_bracket();
        if !bracket.physical_from.contains(&key) || bracket.physical_deduct == 0 {
            return 0;
        }

        let current_effective = self.effective_physical_deduction_for(key);
        let other_effective_total = self.physical_deduction_total() - current_effective;
        let remaining_effective = (bracket.physical_deduct - other_effective_total).max(0);
        let max_effective_for_key = max_physical_deduction_for_raw(self.chars.get_char(key));
        remaining_effective.min(max_effective_for_key).max(0)
    }

    pub(crate) fn set_age_deduction(&mut self, key: Characteristic, value: i32) {
        let max_for_key = self.max_age_deduction_for(key);
        let snapped = clamp_step_5(value, 0, max_for_key);
        self.age_deductions.set_char(key, snapped);
    }

    pub(crate) fn sanitize_characteristics(&mut self) {
        for def in CHARACTERISTICS {
            let raw = self.chars.get_char(def.key);
            let sanitized = if raw <= 0 {
                0
            } else {
                clamp_step_5(raw, def.min, def.max)
            };
            self.chars.set_char(def.key, sanitized);
        }

        let chars = self.chars.clone();
        self.char_rolls.retain(|key, roll| {
            Characteristic::from_key(key)
                .map(|characteristic| {
                    let value = chars.get_char(characteristic);
                    value > 0 && value == roll.value
                })
                .unwrap_or(false)
        });
    }

    pub(crate) fn sanitize_age_deductions(&mut self) {
        let bracket = self.age_bracket();
        let allowed: HashSet<Characteristic> = bracket.physical_from.iter().copied().collect();

        for def in CHARACTERISTICS {
            if !allowed.contains(&def.key) || bracket.physical_deduct == 0 {
                self.age_deductions.set_char(def.key, 0);
                continue;
            }

            let max_for_key = max_physical_deduction_for_raw(self.chars.get_char(def.key));
            let value = self.age_deductions.get_char(def.key);
            self.age_deductions
                .set_char(def.key, clamp_step_5(value, 0, max_for_key));
        }

        let mut excess = (self.physical_deduction_total() - bracket.physical_deduct).max(0);
        for key in bracket.physical_from.iter().rev() {
            if excess == 0 {
                break;
            }
            let current = self.age_deductions.get_char(*key);
            let reduction = current.min(excess);
            self.age_deductions.set_char(*key, current - reduction);
            excess -= reduction;
        }
    }

    pub(crate) fn edu_age_checks_complete(&self) -> bool {
        let bracket = self.age_bracket();
        bracket.edu_checks == 0 || self.edu_check_rolls.len() == bracket.edu_checks
    }

    pub(crate) fn active_formula_key_for(&self, occupation: Option<&Occupation>) -> FormulaKey {
        match occupation {
            Some(occupation) if occupation.formula_keys.contains(&self.formula_key) => {
                self.formula_key
            }
            Some(occupation) => occupation
                .formula_keys
                .first()
                .copied()
                .unwrap_or(FormulaKey::Edu4),
            None => FormulaKey::Edu4,
        }
    }

    pub(crate) fn normalize_formula_key_for(&mut self, occupation: Option<&Occupation>) {
        self.formula_key = self.active_formula_key_for(occupation);
    }

    pub(crate) fn sanitize_allocations(&mut self) {
        let skill_rows = self.sheet_math().skill_rows;
        let mut occupation_points = HashMap::new();
        let mut personal_points = HashMap::new();

        for row in skill_rows {
            if row.occ_add > 0 {
                occupation_points.insert(row.name.clone(), row.occ_add);
            }
            if row.personal_add > 0 {
                personal_points.insert(row.name, row.personal_add);
            }
        }

        self.allocations.occupation_points = occupation_points;
        self.allocations.personal_points = personal_points;
    }

    pub(crate) fn sanitize_state(&mut self) {
        self.normalize_custom_occupation_skills();
        let selected_occupation = self.selected_occupation();

        if let Some(occupation) = selected_occupation.as_ref() {
            self.normalize_formula_key_for(Some(occupation));
            self.prune_occupation_choices_for(occupation);
        } else {
            self.occupation_choices.clear();
            self.formula_key = FormulaKey::Edu4;
        }

        self.sanitize_characteristics();
        self.sanitize_age_deductions();
        self.sanitize_allocations();
    }

    pub(crate) fn personal_budget_for(&self, final_chars: &CharacteristicValues) -> i32 {
        final_chars.get_char(Characteristic::Int) * 2
    }

    pub(crate) fn used_occupation_points_from(skill_rows: &[SkillRow]) -> i32 {
        skill_rows.iter().map(|row| row.occ_add).sum()
    }

    #[cfg(test)]
    pub(crate) fn used_occupation_points(&self) -> i32 {
        Self::used_occupation_points_from(&self.sheet_math().skill_rows)
    }

    pub(crate) fn used_personal_points_from(skill_rows: &[SkillRow]) -> i32 {
        skill_rows.iter().map(|row| row.personal_add).sum()
    }

    #[cfg(test)]
    pub(crate) fn used_personal_points(&self) -> i32 {
        Self::used_personal_points_from(&self.sheet_math().skill_rows)
    }

    pub(crate) fn clear_occupation_allocations(&mut self) {
        self.allocations.occupation_points.clear();
    }

    pub(crate) fn clear_personal_allocations(&mut self) {
        self.allocations.personal_points.clear();
    }

    fn allocation_row_for<'a>(math: &'a SheetMath, skill: &str) -> Option<&'a SkillRow> {
        math.skill_rows.iter().find(|row| row.name == skill)
    }

    pub(crate) fn occupation_allocation_max_for(&self, skill: &str) -> i32 {
        let math = self.sheet_math();
        let Some(row) = Self::allocation_row_for(&math, skill) else {
            return 0;
        };
        if math.selected_occupation.is_none()
            || !skill_accepts_occupation_points(skill, &math.occupation_skill_set)
        {
            return 0;
        }

        (MAX_CREATION_VALUE - row.base - row.personal_add).max(0)
    }

    pub(crate) fn personal_allocation_max_for(&self, skill: &str) -> i32 {
        let math = self.sheet_math();
        let Some(row) = Self::allocation_row_for(&math, skill) else {
            return 0;
        };
        if !skill_accepts_personal_points(skill) {
            return 0;
        }

        (MAX_CREATION_VALUE - row.base - row.occ_add).max(0)
    }

    pub(crate) fn set_occupation_allocation(&mut self, skill: &str, value: i32) {
        let max_value = self.occupation_allocation_max_for(skill);
        if max_value > 0 {
            set_allocation(
                &mut self.allocations.occupation_points,
                skill,
                value,
                max_value,
            );
        } else {
            self.allocations.occupation_points.remove(skill);
        }
    }

    pub(crate) fn set_personal_allocation(&mut self, skill: &str, value: i32) {
        let max_value = self.personal_allocation_max_for(skill);
        if max_value > 0 {
            set_allocation(
                &mut self.allocations.personal_points,
                skill,
                value,
                max_value,
            );
        } else {
            self.allocations.personal_points.remove(skill);
        }
    }

    #[cfg(test)]
    pub(crate) fn credit_rating(&self) -> i32 {
        let final_chars = self.final_chars();
        let selected_occupation = self.selected_occupation();
        let occupation_skill_set = self.occupation_skill_set_for(selected_occupation.as_ref());
        self.credit_rating_for_with_skills(&final_chars, &occupation_skill_set)
    }

    pub(crate) fn credit_rating_for_with_skills(
        &self,
        final_chars: &CharacteristicValues,
        occupation_skill_set: &HashSet<String>,
    ) -> i32 {
        let occupation_add =
            if skill_accepts_occupation_points("Credit Rating", occupation_skill_set) {
                sanitized_allocation_value(
                    &self.allocations.occupation_points,
                    "Credit Rating",
                    MAX_CREATION_VALUE - get_base_skill("Credit Rating", final_chars),
                )
            } else {
                0
            };

        get_base_skill("Credit Rating", final_chars) + occupation_add
    }

    pub(crate) fn summary_blockers_for(&self, math: &SheetMath) -> Vec<String> {
        let mut blockers = Vec::new();
        let bracket = self.age_bracket();
        let physical_total = self.physical_deduction_total();
        let used_occ = Self::used_occupation_points_from(&math.skill_rows);
        let used_personal = Self::used_personal_points_from(&math.skill_rows);
        let has_occupation = math.selected_occupation.is_some();
        let credit = math.credit_rating;
        let (credit_min, credit_max) = math.credit_range;

        if !self.has_all_chars() {
            blockers.push("missing characteristics".to_owned());
        }
        if physical_total != bracket.physical_deduct {
            if self.physical_deduction_is_possible() {
                blockers.push(format!(
                    "age deductions {physical_total}/{}",
                    bracket.physical_deduct
                ));
            } else {
                blockers.push(format!(
                    "age deductions impossible: requires {}, current STR/CON/DEX can absorb only {}",
                    bracket.physical_deduct,
                    self.max_possible_physical_deduction()
                ));
            }
        }
        if !self.edu_age_checks_complete() {
            blockers.push(format!(
                "EDU age checks {}/{}",
                self.edu_check_rolls.len(),
                bracket.edu_checks
            ));
        }
        if self.char_method == CharMethod::PointBuy && self.point_buy_spent() != POINT_BUY_BUDGET {
            blockers.push(format!(
                "point-buy budget {}/{POINT_BUY_BUDGET}",
                self.point_buy_spent()
            ));
        }
        if self.luck_state.value.is_none() {
            blockers.push("Luck not rolled".to_owned());
        }
        if !has_occupation {
            blockers.push("occupation missing".to_owned());
        } else {
            if math.unresolved_choices > 0 {
                blockers.push(format!(
                    "occupation choices unresolved: {}",
                    math.unresolved_choices
                ));
            }
            if math.occupation_shortfall > 0 {
                blockers.push(format!(
                    "occupation skill shortfall: {}",
                    math.occupation_shortfall
                ));
            }
        }
        if used_occ != math.occupation_budget {
            blockers.push(format!(
                "occupation points {used_occ}/{}",
                math.occupation_budget
            ));
        }
        if used_personal != math.personal_budget {
            blockers.push(format!(
                "personal points {used_personal}/{}",
                math.personal_budget
            ));
        }
        if has_occupation && (credit < credit_min || credit > credit_max) {
            blockers.push(format!(
                "Credit Rating {credit}% outside {credit_min}–{credit_max}"
            ));
        }
        if math
            .skill_rows
            .iter()
            .any(|row| row.total > MAX_CREATION_VALUE)
        {
            blockers.push("skill total above 99".to_owned());
        }

        blockers
    }

    pub(crate) fn apply_characteristic_preset(
        &mut self,
        method: CharMethod,
        preset: &[(&str, i32)],
    ) {
        self.char_method = method;
        let mut chars = CharacteristicValues::default();
        for (key, value) in preset {
            let def = CHARACTERISTICS
                .iter()
                .find(|def| def.key.key() == *key)
                .unwrap_or_else(|| panic!("unknown characteristic key in preset: {key}"));
            chars.set_char(def.key, clamp_step_5(*value, def.min, def.max));
        }
        self.chars = chars;
        self.char_rolls.clear();
        self.clear_edu_age_checks();
        self.sanitize_age_deductions();
    }

    pub(crate) fn roll_all_characteristics(&mut self) {
        let mut chars = CharacteristicValues::default();
        let mut rolls = HashMap::new();

        for def in CHARACTERISTICS {
            let result = self.roll_characteristic(*def);
            chars.set_char(def.key, result.value);
            rolls.insert(def.key.key().to_owned(), result);
        }

        self.char_method = CharMethod::Roll;
        self.chars = chars;
        self.char_rolls = rolls;
        self.clear_edu_age_checks();
        self.sanitize_age_deductions();
    }

    pub(crate) fn roll_single_characteristic(&mut self, key: &str) {
        if let Some(def) = CHARACTERISTICS.iter().find(|def| def.key.key() == key) {
            let result = self.roll_characteristic(*def);
            self.chars.set_char(def.key, result.value);
            self.char_rolls.insert(key.to_owned(), result);

            if self.char_method != CharMethod::Roll {
                self.char_method = CharMethod::Mixed;
            }
            if key == "EDU" {
                self.clear_edu_age_checks();
            }
            self.sanitize_age_deductions();
        }
    }

    #[cfg(test)]
    pub(crate) fn apply_edu_age_check_rolls(&mut self, scripted_rolls: &[(i32, i32)]) {
        let bracket = self.age_bracket();
        let raw_edu = self.char_value("EDU");
        if raw_edu == 0 || bracket.edu_checks == 0 {
            return;
        }

        let starting_edu = (raw_edu - bracket.edu_penalty).clamp(1, MAX_CREATION_VALUE);
        let mut current_edu = starting_edu;
        let mut rolls = Vec::new();

        for &(d100, improvement_gain) in scripted_rolls.iter().take(bracket.edu_checks) {
            let d100 = d100.clamp(1, 100);
            let improved = d100 > current_edu;
            let gain = if improved {
                improvement_gain.clamp(1, 10)
            } else {
                0
            };
            current_edu = (current_edu + gain).clamp(1, MAX_CREATION_VALUE);
            rolls.push(EduCheckRoll {
                d100,
                improved,
                gain,
                resulting_edu: current_edu,
            });
        }

        self.edu_bonus = (current_edu - starting_edu).max(0);
        self.edu_check_rolls = rolls;
    }

    pub(crate) fn roll_edu_age_checks(&mut self) {
        let bracket = self.age_bracket();
        let raw_edu = self.char_value("EDU");
        if raw_edu == 0 || bracket.edu_checks == 0 {
            return;
        }

        let starting_edu = (raw_edu - bracket.edu_penalty).clamp(1, MAX_CREATION_VALUE);
        let mut current_edu = starting_edu;
        let mut rolls = Vec::new();

        for _ in 0..bracket.edu_checks {
            let d100 = self.roll_die(100) as i32;
            let improved = d100 > current_edu;
            let gain = if improved {
                self.roll_die(10) as i32
            } else {
                0
            };

            current_edu = (current_edu + gain).clamp(1, MAX_CREATION_VALUE);
            rolls.push(EduCheckRoll {
                d100,
                improved,
                gain,
                resulting_edu: current_edu,
            });
        }

        self.edu_bonus = (current_edu - starting_edu).max(0);
        self.edu_check_rolls = rolls;
    }

    pub(crate) fn prune_occupation_allocations(&mut self) {
        self.sanitize_allocations();
    }

    pub(crate) fn prune_personal_allocations(&mut self) {
        self.sanitize_allocations();
    }

    pub(crate) fn apply_quick_skill_package(&mut self) {
        let Some(occupation) = self.selected_occupation() else {
            return;
        };

        if self.unresolved_choice_count_for(&occupation) > 0
            || self.unique_occupation_shortfall_for(&occupation) > 0
        {
            return;
        }

        let package_values = [70, 60, 60, 50, 50, 50, 40, 40];
        let final_chars = self.final_chars();
        let occupation_budget = self
            .active_formula_key_for(Some(&occupation))
            .calculate(&final_chars);
        let mut remaining_budget = occupation_budget.max(0);
        let mut skill_order = self.resolved_occupation_skills_for(&occupation);
        skill_order.sort_by(|left, right| {
            get_base_skill(left, &final_chars)
                .cmp(&get_base_skill(right, &final_chars))
                .then_with(|| left.cmp(right))
        });

        let mut next = HashMap::new();
        if occupation.credit.0 > 0 && remaining_budget > 0 {
            let credit_base = get_base_skill("Credit Rating", &final_chars);
            let credit_cap = (MAX_CREATION_VALUE - credit_base).max(0);
            let credit_add = occupation.credit.0.min(remaining_budget).min(credit_cap);

            if credit_add > 0 {
                next.insert("Credit Rating".to_owned(), credit_add);
                remaining_budget -= credit_add;
            }
        }

        for (skill, target) in skill_order.iter().zip(package_values) {
            if remaining_budget <= 0 {
                break;
            }

            let base = get_base_skill(skill, &final_chars);
            let personal_add = sanitized_allocation_value(
                &self.allocations.personal_points,
                skill,
                MAX_CREATION_VALUE - base,
            );
            let current_add = next.get(skill).copied().unwrap_or(0);
            let skill_cap = (MAX_CREATION_VALUE - base - personal_add - current_add).max(0);
            let target_add = (target - base - current_add).max(0);
            let add = target_add.min(skill_cap).min(remaining_budget);

            if add > 0 {
                *next.entry(skill.clone()).or_insert(0) += add;
                remaining_budget -= add;
            }
        }

        for skill in &skill_order {
            if remaining_budget <= 0 {
                break;
            }

            let base = get_base_skill(skill, &final_chars);
            let personal_add = sanitized_allocation_value(
                &self.allocations.personal_points,
                skill,
                MAX_CREATION_VALUE - base,
            );
            let current_add = next.get(skill).copied().unwrap_or(0);
            let skill_cap = (MAX_CREATION_VALUE - base - personal_add - current_add).max(0);
            let add = skill_cap.min(remaining_budget);

            if add > 0 {
                *next.entry(skill.clone()).or_insert(0) += add;
                remaining_budget -= add;
            }
        }

        self.allocations.occupation_points = next;
        self.sanitize_allocations();
    }

    pub(crate) fn reset_investigator(&mut self) {
        let occupations = std::mem::take(&mut self.occupations);
        let startup_validation_errors = std::mem::take(&mut self.startup_validation_errors);
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(0xC0C7_E7E5_1234_5678);
        *self = Self::fresh(occupations, seed | 1);
        self.startup_validation_errors = startup_validation_errors;
    }

    pub(crate) fn max_reachable_step(&self) -> usize {
        let selected = self.selected_occupation();

        if !self.has_all_chars() {
            return 2;
        }

        let Some(occupation) = selected.as_ref() else {
            return 3;
        };

        let unresolved = self.unresolved_choice_count_for(occupation);
        let shortfall = self.unique_occupation_shortfall_for(occupation);

        if unresolved > 0 || shortfall > 0 {
            return 4;
        }

        // Step 5, Backstory, is intentionally optional.
        // Step 6 usually unlocks once required physical age deductions and EDU
        // checks are resolved. If the age deduction cannot be satisfied with the
        // current STR/CON/DEX values, Summary still unlocks so the blocker can
        // explain the impossible state instead of leaving the user at a dead end.
        // Remaining allocation, credit, Luck, and skill-cap issues are surfaced
        // as rule checks on the Summary page.
        let physical_ready_for_summary =
            self.physical_deduction_is_complete() || !self.physical_deduction_is_possible();
        if physical_ready_for_summary && self.edu_age_checks_complete() {
            6
        } else {
            5
        }
    }

    pub(crate) fn refresh_reachability(&mut self) {
        self.frame_max_reachable_step = self.max_reachable_step();
        if self.step > self.frame_max_reachable_step {
            self.step = self.frame_max_reachable_step;
        }
    }

    pub(crate) fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(12.0);
            ui.label(
                RichText::new("Chaosium · Call of Cthulhu 7th Edition")
                    .size(11.0)
                    .color(FAINT)
                    .strong(),
            );
            ui.label(
                RichText::new("Investigator Creator")
                    .size(32.0)
                    .color(TEXT)
                    .strong(),
            );
            ui.label(
                RichText::new("Rules-aware character creation helper")
                    .size(14.0)
                    .color(MUTED),
            );
            ui.add_space(10.0);
        });
    }

    pub(crate) fn step_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            let max_reachable = self.frame_max_reachable_step;
            for (index, label) in STEPS.iter().enumerate() {
                let step_num = index + 1;
                let selected = self.step == step_num;
                let enabled = step_num <= max_reachable;
                let text = if step_num < self.step {
                    format!("✓ {label}")
                } else {
                    format!("{step_num}. {label}")
                };
                let response = ui.selectable_label(
                    selected,
                    RichText::new(text).color(if selected {
                        ACCENT
                    } else if enabled {
                        MUTED
                    } else {
                        FAINT
                    }),
                );
                if enabled && response.clicked() {
                    self.step = step_num;
                }
            }
        });
        ui.add_space(10.0);
    }

    pub(crate) fn save_load_panel(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Save / Load JSON")
            .default_open(false)
            .show(ui, |ui| {
                card(ui, |ui| {
                    ui.label(
                        RichText::new(
                            "Copy a JSON save to preserve editable investigator state, or paste one here to load it back into the creator.",
                        )
                        .small()
                        .color(MUTED),
                    );
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Copy JSON save").clicked() {
                            match self.export_json_save() {
                                Ok(json) => {
                                    ui.ctx().copy_text(json);
                                    self.save_load_message =
                                        Some("Copied JSON save to clipboard.".to_owned());
                                }
                                Err(error) => {
                                    self.save_load_message =
                                        Some(format!("Could not build JSON save: {error}"));
                                }
                            }
                        }

                        let load_response = ui.add_enabled(
                            !self.import_json_text.trim().is_empty(),
                            egui::Button::new("Load JSON save"),
                        );
                        if load_response.clicked() {
                            let input = self.import_json_text.clone();
                            match self.import_json_save(&input) {
                                Ok(()) => {
                                    self.import_json_text.clear();
                                    self.save_load_message = Some("Loaded JSON save.".to_owned());
                                }
                                Err(error) => {
                                    self.save_load_message = Some(error);
                                }
                            }
                        } else if self.import_json_text.trim().is_empty() {
                            load_response.on_hover_text("Paste a JSON save before loading.");
                        }
                    });
                    if let Some(message) = &self.save_load_message {
                        ui.label(RichText::new(message).small().color(AMBER));
                    }
                    ui.add_sized(
                        [ui.available_width(), 120.0],
                        egui::TextEdit::multiline(&mut self.import_json_text)
                            .hint_text("Paste JSON save here, then press Load JSON save"),
                    );
                });
            });
        ui.add_space(8.0);
    }

    pub(crate) fn navigation(&mut self, ui: &mut egui::Ui) {
        self.refresh_reachability();
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.step < STEPS.len()
                    && ui
                        .add_enabled(
                            self.step < self.frame_max_reachable_step,
                            egui::Button::new("Continue →"),
                        )
                        .clicked()
                {
                    self.step = (self.step + 1).min(STEPS.len());
                }
                if self.step > 1 && ui.button("← Back").clicked() {
                    self.step = self.step.saturating_sub(1).max(1);
                }
            });
        });
    }

    pub(crate) fn render_startup_validation_error(&self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            ui.label(
                RichText::new("Internal ruleset validation failed")
                    .size(22.0)
                    .color(RED)
                    .strong(),
            );
            ui.label(
                RichText::new(
                    "The built-in skill or occupation data is inconsistent, so normal character creation has been disabled instead of panicking during startup.",
                )
                .color(MUTED),
            );
            ui.add_space(8.0);
            for error in &self.startup_validation_errors {
                ui.label(RichText::new(format!("• {error}")).small().color(AMBER));
            }
        });
    }
}

impl eframe::App for CoC7eApp {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.startup_validation_errors.is_empty() {
            return;
        }

        self.sync_age_bracket();
        self.sanitize_state();
        self.refresh_reachability();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG))
            .show_inside(ui, |ui| {
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .min_scrolled_width(APP_CONTENT_WIDTH)
                    .show(ui, |ui| {
                        ui.set_min_width(APP_CONTENT_WIDTH);
                        ui.set_max_width(APP_CONTENT_WIDTH);

                        self.top_bar(ui);

                        if !self.startup_validation_errors.is_empty() {
                            self.render_startup_validation_error(ui);
                            return;
                        }

                        self.step_bar(ui);
                        self.save_load_panel(ui);

                        match self.step {
                            1 => self.render_concept(ui),
                            2 => self.render_characteristics(ui),
                            3 => self.render_occupation(ui),
                            4 => self.render_skills(ui),
                            5 => self.render_backstory(ui),
                            6 => self.render_summary(ui),
                            _ => self.step = 1,
                        }
                    });
            });
    }
}
