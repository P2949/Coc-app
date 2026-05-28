use super::data::*;
use super::occupations::*;
use super::ruleset::CHARACTERISTICS;
use std::collections::{HashMap, HashSet};

impl CoC7eApp {
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

    pub(crate) fn custom_occupation_required_skill_count(&self) -> usize {
        self.custom_occupation.required_skill_count.clamp(
            CUSTOM_OCCUPATION_MIN_SKILL_COUNT,
            CUSTOM_OCCUPATION_SKILL_COUNT,
        )
    }

    pub(crate) fn normalize_custom_occupation_skills(&mut self) {
        self.custom_occupation.required_skill_count = self.custom_occupation_required_skill_count();
        self.custom_occupation
            .skills
            .resize(CUSTOM_OCCUPATION_SKILL_COUNT, String::new());
        self.custom_occupation
            .skills
            .truncate(CUSTOM_OCCUPATION_SKILL_COUNT);
    }

    pub(crate) fn custom_skill_display_name_for_slot(&self, index: usize, skill: Skill) -> String {
        if let Some(label) = self.custom_occupation.skill_slot_labels.get(&index) {
            let trimmed = label.trim();
            if !trimmed.is_empty() {
                return trimmed.to_owned();
            }
        }
        if let Some(label) = self.custom_occupation.skill_labels.get(skill.name()) {
            let trimmed = label.trim();
            if !trimmed.is_empty() {
                return trimmed.to_owned();
            }
        }
        skill.name().to_owned()
    }

    pub(crate) fn custom_occupation_skill_slots(&self) -> Vec<(usize, Skill)> {
        self.custom_occupation
            .skills
            .iter()
            .take(self.custom_occupation_required_skill_count())
            .enumerate()
            .filter_map(|(index, skill)| Skill::from_name(skill.trim()).map(|skill| (index, skill)))
            .collect()
    }

    pub(crate) fn sanitize_custom_occupation(&mut self) {
        self.custom_occupation.credit_min = self.custom_occupation.credit_min.clamp(0, 99);
        self.custom_occupation.credit_max = self.custom_occupation.credit_max.clamp(0, 99);
        self.normalize_custom_occupation_skills();

        for skill in &mut self.custom_occupation.skills {
            let normalized = skill.trim().to_owned();
            if normalized.is_empty() || !OCCUPATION_SELECTABLE_SKILLS.contains(&normalized.as_str())
            {
                skill.clear();
            } else {
                *skill = normalized;
            }
        }

        let required_count = self.custom_occupation_required_skill_count();
        let mut active_slots_by_skill: HashMap<Skill, Vec<usize>> = HashMap::new();
        for (index, skill) in self
            .custom_occupation
            .skills
            .iter()
            .take(required_count)
            .enumerate()
        {
            if let Some(skill) = Skill::from_name(skill.trim()) {
                active_slots_by_skill.entry(skill).or_default().push(index);
            }
        }

        for indices in active_slots_by_skill.values() {
            if indices.len() < 2 {
                continue;
            }

            let mut seen_labels = HashSet::new();
            let has_distinct_slot_labels = indices.iter().all(|index| {
                let label = self
                    .custom_occupation
                    .skill_slot_labels
                    .get(index)
                    .map(|label| label.trim())
                    .unwrap_or_default();
                !label.is_empty() && seen_labels.insert(label.to_owned())
            });

            if !has_distinct_slot_labels {
                for index in indices.iter().skip(1) {
                    self.custom_occupation.skills[*index].clear();
                    self.custom_occupation.skill_slot_labels.remove(index);
                }
            }
        }

        let all_valid_skills: HashSet<Skill> = self
            .custom_occupation
            .skills
            .iter()
            .filter_map(|skill| Skill::from_name(skill.trim()))
            .collect();
        self.custom_occupation
            .skill_labels
            .retain(|skill_name, label| {
                let Some(skill) = Skill::from_name(skill_name.trim()) else {
                    return false;
                };
                all_valid_skills.contains(&skill) && !label.trim().is_empty()
            });
        let normalized_labels: Vec<(String, String)> = self
            .custom_occupation
            .skill_labels
            .iter()
            .map(|(skill, label)| (skill.trim().to_owned(), label.trim().to_owned()))
            .collect();
        self.custom_occupation.skill_labels.clear();
        for (skill, label) in normalized_labels {
            self.custom_occupation.skill_labels.insert(skill, label);
        }

        let valid_labeled_slots: HashSet<usize> = self
            .custom_occupation
            .skills
            .iter()
            .enumerate()
            .filter_map(|(index, skill)| {
                if skill.trim().is_empty() {
                    None
                } else {
                    Some(index)
                }
            })
            .collect();
        self.custom_occupation
            .skill_slot_labels
            .retain(|index, label| valid_labeled_slots.contains(index) && !label.trim().is_empty());
        let normalized_slot_labels: Vec<(usize, String)> = self
            .custom_occupation
            .skill_slot_labels
            .iter()
            .map(|(index, label)| (*index, label.trim().chars().take(64).collect()))
            .collect();
        self.custom_occupation.skill_slot_labels.clear();
        for (index, label) in normalized_slot_labels {
            self.custom_occupation
                .skill_slot_labels
                .insert(index, label);
        }
    }

    fn occupation_choice_slots<'a>(
        &self,
        occupation: &'a Occupation,
    ) -> Vec<(ChoiceKey, &'a [Skill])> {
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

    fn valid_choice_values_for(&self, occupation: &Occupation) -> Vec<Skill> {
        let mut used = fixed_skill_set_for(occupation);
        let mut out = Vec::new();

        for (key, options) in self.occupation_choice_slots(occupation) {
            let Some(value) = self.occupation_choices.get(&key) else {
                continue;
            };
            let value = value.trim();
            let Some(skill) = Skill::from_name(value) else {
                continue;
            };
            if choice_value_is_valid(options, value) && used.insert(skill) {
                out.push(skill);
            }
        }

        out
    }

    pub(crate) fn prune_occupation_choices_for(&mut self, occupation: &Occupation) {
        let mut used = fixed_skill_set_for(occupation);
        let mut cleaned = HashMap::new();

        for (key, options) in self.occupation_choice_slots(occupation) {
            let Some(value) = self.occupation_choices.get(&key) else {
                continue;
            };
            let value = value.trim();
            let Some(skill) = Skill::from_name(value) else {
                continue;
            };
            if choice_value_is_valid(options, value) && used.insert(skill) {
                cleaned.insert(key, value.to_owned());
            }
        }

        self.occupation_choices = cleaned;
    }

    pub(crate) fn build_custom_occupation(&self) -> Occupation {
        let min = self.custom_occupation.credit_min.clamp(0, 99);
        let max = self.custom_occupation.credit_max.clamp(0, 99);
        let skills: Vec<Skill> = self
            .custom_occupation
            .skills
            .iter()
            .take(self.custom_occupation_required_skill_count())
            .filter_map(|skill| {
                let skill = skill.trim();
                if skill.is_empty() || !OCCUPATION_SELECTABLE_SKILLS.contains(&skill) {
                    None
                } else {
                    Skill::from_name(skill)
                }
            })
            .collect();

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

    pub(crate) fn set_custom_occupation_required_skill_count(&mut self, next: usize) {
        self.custom_occupation.required_skill_count = next.clamp(
            CUSTOM_OCCUPATION_MIN_SKILL_COUNT,
            CUSTOM_OCCUPATION_SKILL_COUNT,
        );
        self.sanitize_custom_occupation();
        self.prune_occupation_allocations();
    }

    pub(crate) fn set_custom_occupation_skill_label_for_slot(
        &mut self,
        index: usize,
        next: String,
    ) -> bool {
        self.normalize_custom_occupation_skills();
        if index >= self.custom_occupation_required_skill_count()
            || Skill::from_name(self.custom_occupation.skills[index].trim()).is_none()
        {
            self.custom_occupation.skill_slot_labels.remove(&index);
            return false;
        }

        let normalized = next.trim();
        if normalized.is_empty() {
            self.custom_occupation.skill_slot_labels.remove(&index);
            return true;
        }

        let normalized = normalized.chars().take(64).collect::<String>();
        self.custom_occupation
            .skill_slot_labels
            .insert(index, normalized);
        true
    }

    #[cfg(test)]
    pub(crate) fn set_custom_occupation_skill_label(&mut self, skill: Skill, next: String) -> bool {
        let Some((index, _)) = self
            .custom_occupation_skill_slots()
            .into_iter()
            .find(|(_, slot_skill)| *slot_skill == skill)
        else {
            self.custom_occupation.skill_labels.remove(skill.name());
            return false;
        };
        self.set_custom_occupation_skill_label_for_slot(index, next)
    }

    pub(crate) fn set_custom_occupation_skill(&mut self, index: usize, next: String) -> bool {
        self.normalize_custom_occupation_skills();
        if index >= CUSTOM_OCCUPATION_SKILL_COUNT {
            return false;
        }

        let normalized = next.trim().to_owned();
        if normalized.is_empty() {
            self.custom_occupation.skill_slot_labels.remove(&index);
            self.custom_occupation.skills[index].clear();
            self.prune_occupation_allocations();
            return true;
        }
        if !OCCUPATION_SELECTABLE_SKILLS.contains(&normalized.as_str()) {
            return false;
        }
        if let Some(old_skill) = Skill::from_name(self.custom_occupation.skills[index].trim())
            && old_skill.name() != normalized.as_str()
        {
            self.custom_occupation.skill_slot_labels.remove(&index);
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
        if Skill::from_name(&normalized)
            .is_some_and(|skill| fixed_skill_set_for(&occupation).contains(&skill))
        {
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

    pub(crate) fn resolved_occupation_skills_for(&self, occupation: &Occupation) -> Vec<Skill> {
        let mut resolved = Vec::new();
        let mut seen = HashSet::new();
        for slot in &occupation.slots {
            if let Slot::Skill(skill) = slot
                && (self.occupation_id == CUSTOM_OCCUPATION_ID || seen.insert(*skill))
            {
                resolved.push(*skill);
            }
        }
        for skill in self.valid_choice_values_for(occupation) {
            if seen.insert(skill) {
                resolved.push(skill);
            }
        }
        resolved
    }

    pub(crate) fn occupation_skill_set_for(
        &self,
        occupation: Option<&Occupation>,
    ) -> HashSet<Skill> {
        occupation.map_or_else(HashSet::new, |occupation| {
            let mut set: HashSet<Skill> = self
                .resolved_occupation_skills_for(occupation)
                .into_iter()
                .collect();
            set.insert(Skill::CreditRating);
            set
        })
    }

    pub(crate) fn unresolved_choice_count_for(&self, occupation: &Occupation) -> usize {
        let mut used = fixed_skill_set_for(occupation);
        let mut unresolved = 0;

        for (key, options) in self.occupation_choice_slots(occupation) {
            let Some(value) = self.occupation_choices.get(&key) else {
                unresolved += 1;
                continue;
            };
            let value = value.trim();
            let Some(skill) = Skill::from_name(value) else {
                unresolved += 1;
                continue;
            };
            if !choice_value_is_valid(options, value) || !used.insert(skill) {
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
        // but the creator still requires the configured number of custom skill slots to be filled.
        if self.occupation_id == CUSTOM_OCCUPATION_ID {
            self.custom_occupation_required_skill_count()
        } else {
            self.occupation_slot_count_for(occupation)
        }
    }

    pub(crate) fn unique_occupation_shortfall_for(&self, occupation: &Occupation) -> usize {
        self.required_occupation_skill_count_for(occupation)
            .saturating_sub(self.resolved_occupation_skills_for(occupation).len())
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

    pub(crate) fn sanitize_state(&mut self) {
        let _ = self.sanitize_state_with_report();
    }

    pub(crate) fn sanitize_state_with_report(&mut self) -> SanitizeReport {
        let custom_before = self.custom_occupation.clone();
        let chars_before = self.chars.clone();
        let char_rolls_before: HashSet<String> = self.char_rolls.keys().cloned().collect();
        let luck_before = self.luck_state.clone();
        let edu_checks_before = self.edu_check_rolls.clone();
        let edu_bonus_before = self.edu_bonus;
        let age_deductions_before = self.age_deductions.clone();
        let formula_before = self.formula_key;
        let occupation_choices_before = self.occupation_choices.clone();

        self.sanitize_custom_occupation();
        let selected_occupation = self.selected_occupation();

        if let Some(occupation) = selected_occupation.as_ref() {
            self.normalize_formula_key_for(Some(occupation));
            self.prune_occupation_choices_for(occupation);
        } else {
            self.occupation_choices.clear();
            self.formula_key = FormulaKey::Edu4;
        }

        self.sanitize_characteristics();
        self.sanitize_luck_state();
        self.sanitize_edu_age_checks();
        self.sanitize_age_deductions();
        let mut report = self.sanitize_allocations_with_report();

        for def in CHARACTERISTICS {
            let before = chars_before.get_char(def.key);
            let after = self.chars.get_char(def.key);
            if before != after {
                report
                    .clamped_characteristics
                    .push(format!("{}: {before} → {after}", def.key.key()));
            }
        }
        for key in char_rolls_before {
            if !self.char_rolls.contains_key(&key) {
                report.removed_characteristic_rolls.push(key);
            }
        }
        report.reset_luck = luck_before != self.luck_state && self.luck_state.value.is_none();
        report.normalized_edu_checks =
            edu_checks_before != self.edu_check_rolls || edu_bonus_before != self.edu_bonus;
        report.normalized_age_deductions = age_deductions_before != self.age_deductions;
        report.normalized_formula = formula_before != self.formula_key;
        report.normalized_custom_occupation = custom_before.name.trim()
            != self.custom_occupation.name.trim()
            || custom_before.credit_min != self.custom_occupation.credit_min
            || custom_before.credit_max != self.custom_occupation.credit_max
            || custom_before.required_skill_count != self.custom_occupation.required_skill_count
            || custom_before.skills != self.custom_occupation.skills
            || custom_before.skill_labels != self.custom_occupation.skill_labels
            || custom_before.skill_slot_labels != self.custom_occupation.skill_slot_labels;

        for (key, value) in occupation_choices_before {
            if !self.occupation_choices.contains_key(&key) {
                report
                    .removed_occupation_choices
                    .push(format!("{}[{}] = {}", key.id, key.index, value));
            }
        }
        report
    }
}
