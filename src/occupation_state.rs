use super::data::*;
use super::models::*;
use super::occupations::*;
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

    pub(crate) fn custom_skill_display_name(&self, skill: Skill) -> String {
        if self.occupation_id == CUSTOM_OCCUPATION_ID {
            let key = skill.name();
            if let Some(label) = self.custom_occupation.skill_labels.get(key) {
                let trimmed = label.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_owned();
                }
            }
        }
        skill.name().to_owned()
    }

    fn selected_custom_skill_set(&self) -> HashSet<Skill> {
        self.custom_occupation
            .skills
            .iter()
            .take(self.custom_occupation_required_skill_count())
            .filter_map(|skill| Skill::from_name(skill.trim()))
            .collect()
    }

    pub(crate) fn sanitize_custom_occupation(&mut self) {
        self.custom_occupation.credit_min = self.custom_occupation.credit_min.clamp(0, 99);
        self.custom_occupation.credit_max = self.custom_occupation.credit_max.clamp(0, 99);
        self.normalize_custom_occupation_skills();

        let required_count = self.custom_occupation_required_skill_count();
        let mut seen = HashSet::new();
        for (index, skill) in self.custom_occupation.skills.iter_mut().enumerate() {
            let normalized = skill.trim().to_owned();
            if index >= required_count
                || normalized.is_empty()
                || !OCCUPATION_SELECTABLE_SKILLS.contains(&normalized.as_str())
                || !seen.insert(normalized.clone())
            {
                skill.clear();
            } else {
                *skill = normalized;
            }
        }

        let selected = self.selected_custom_skill_set();
        self.custom_occupation
            .skill_labels
            .retain(|skill_name, label| {
                let Some(skill) = Skill::from_name(skill_name.trim()) else {
                    return false;
                };
                selected.contains(&skill) && !label.trim().is_empty()
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
        let skills = unique_strings(
            self.custom_occupation
                .skills
                .iter()
                .take(self.custom_occupation_required_skill_count())
                .map(|skill| skill.trim().to_owned())
                .filter(|skill| {
                    !skill.is_empty() && OCCUPATION_SELECTABLE_SKILLS.contains(&skill.as_str())
                }),
        );

        Occupation {
            name: self.selected_occupation_name(),
            credit: (min.min(max), min.max(max)),
            formula_keys: vec![self.custom_occupation.formula_key],
            slots: skills
                .into_iter()
                .filter_map(|skill| Skill::from_name(&skill).map(Slot::Skill))
                .collect(),
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

    pub(crate) fn set_custom_occupation_skill_label(&mut self, skill: Skill, next: String) -> bool {
        if !self.selected_custom_skill_set().contains(&skill) {
            self.custom_occupation.skill_labels.remove(skill.name());
            return false;
        }

        let normalized = next.trim();
        if normalized.is_empty() || normalized == skill.name() {
            self.custom_occupation.skill_labels.remove(skill.name());
            return true;
        }

        let normalized = normalized.chars().take(64).collect::<String>();
        self.custom_occupation
            .skill_labels
            .insert(skill.name().to_owned(), normalized);
        true
    }

    pub(crate) fn set_custom_occupation_skill(&mut self, index: usize, next: String) -> bool {
        self.normalize_custom_occupation_skills();
        if index >= CUSTOM_OCCUPATION_SKILL_COUNT {
            return false;
        }

        let normalized = next.trim().to_owned();
        if normalized.is_empty() {
            if let Some(old_skill) = Skill::from_name(self.custom_occupation.skills[index].trim()) {
                self.custom_occupation.skill_labels.remove(old_skill.name());
            }
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

        if let Some(old_skill) = Skill::from_name(self.custom_occupation.skills[index].trim())
            && old_skill.name() != normalized.as_str()
        {
            self.custom_occupation.skill_labels.remove(old_skill.name());
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
                && seen.insert(*skill)
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
        let label_count_before = self.custom_occupation.skill_labels.len();
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
        let removed_labels =
            label_count_before.saturating_sub(self.custom_occupation.skill_labels.len());
        for _ in 0..removed_labels {
            report
                .removed_unknown_skills
                .push("custom occupation skill label".to_owned());
        }
        report
    }
}
