use super::data::*;
use super::models::*;
use super::ruleset::SKILL_SPECS;
use std::collections::{HashMap, HashSet};

pub(crate) fn skill_accepts_personal_points(skill: &str) -> bool {
    skill != "Credit Rating" && skill != "Cthulhu Mythos"
}

pub(crate) fn skill_accepts_occupation_points(skill: Skill, allowed: &HashSet<Skill>) -> bool {
    allowed.contains(&skill)
}

pub(crate) fn sanitized_allocation_value(
    allocations: &HashMap<Skill, i32>,
    skill: Skill,
    max_value: i32,
) -> i32 {
    allocations
        .get(&skill)
        .copied()
        .unwrap_or(0)
        .clamp(0, max_value.clamp(0, MAX_CREATION_VALUE))
}

fn sanitized_custom_allocation_value(
    allocations: &HashMap<usize, i32>,
    index: usize,
    max_value: i32,
) -> i32 {
    allocations
        .get(&index)
        .copied()
        .unwrap_or(0)
        .clamp(0, max_value.clamp(0, MAX_CREATION_VALUE))
}

fn record_allocation_changes(
    report: &mut SanitizeReport,
    label: &str,
    before: &HashMap<Skill, i32>,
    after: &HashMap<Skill, i32>,
) {
    for (skill, old_value) in before {
        match after.get(skill) {
            Some(new_value) if new_value < old_value => report.clamped_allocations.push(format!(
                "{label} {}: {} → {}",
                skill.name(),
                old_value,
                new_value
            )),
            Some(_) => {}
            None => report
                .removed_allocations
                .push(format!("{label} {}", skill.name())),
        }
    }
}

fn record_custom_allocation_changes(
    report: &mut SanitizeReport,
    label: &str,
    before: &HashMap<usize, i32>,
    after: &HashMap<usize, i32>,
) {
    for (index, old_value) in before {
        match after.get(index) {
            Some(new_value) if new_value < old_value => report.clamped_allocations.push(format!(
                "{label} custom skill slot {}: {} → {}",
                index + 1,
                old_value,
                new_value
            )),
            Some(_) => {}
            None => report
                .removed_allocations
                .push(format!("{label} custom skill slot {}", index + 1)),
        }
    }
}

impl CoC7eApp {
    pub(crate) fn skill_rows_for(
        &self,
        final_chars: &CharacteristicValues,
        occupation_skill_set: &HashSet<Skill>,
    ) -> Vec<SkillRow> {
        let custom_slots = if self.occupation_id == CUSTOM_OCCUPATION_ID {
            self.custom_occupation_skill_slots()
        } else {
            Vec::new()
        };
        let custom_bases: HashSet<Skill> = custom_slots.iter().map(|(_, skill)| *skill).collect();

        let mut rows: Vec<SkillRow> = SKILL_SPECS
            .iter()
            .filter(|skill| !custom_bases.contains(&skill.id))
            .map(|skill| {
                let base = get_base_skill_for(skill.id, final_chars);
                let occ_add = if skill_accepts_occupation_points(skill.id, occupation_skill_set) {
                    sanitized_allocation_value(
                        &self.allocations.occupation_points,
                        skill.id,
                        MAX_CREATION_VALUE - base,
                    )
                } else {
                    0
                };
                let personal_add = if skill_accepts_personal_points(skill.name) {
                    sanitized_allocation_value(
                        &self.allocations.personal_points,
                        skill.id,
                        MAX_CREATION_VALUE - base - occ_add,
                    )
                } else {
                    0
                };
                SkillRow {
                    id: skill.id,
                    custom_index: None,
                    name: skill.name.to_owned(),
                    base,
                    occ_add,
                    personal_add,
                    total: base + occ_add + personal_add,
                }
            })
            .collect();

        for (index, skill_id) in custom_slots {
            let base = get_base_skill_for(skill_id, final_chars);
            let occ_add = if skill_accepts_occupation_points(skill_id, occupation_skill_set) {
                sanitized_custom_allocation_value(
                    &self.allocations.custom_occupation_points,
                    index,
                    MAX_CREATION_VALUE - base,
                )
            } else {
                0
            };
            let personal_add = if skill_accepts_personal_points(skill_id.name()) {
                sanitized_custom_allocation_value(
                    &self.allocations.custom_personal_points,
                    index,
                    MAX_CREATION_VALUE - base - occ_add,
                )
            } else {
                0
            };
            rows.push(SkillRow {
                id: skill_id,
                custom_index: Some(index),
                name: self.custom_skill_display_name_for_slot(index, skill_id),
                base,
                occ_add,
                personal_add,
                total: base + occ_add + personal_add,
            });
        }

        rows.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.custom_index.cmp(&right.custom_index))
        });
        rows
    }

    pub(crate) fn derived_for(
        &self,
        final_chars: &CharacteristicValues,
        skill_rows: &[SkillRow],
    ) -> Derived {
        let mythos = skill_rows
            .iter()
            .find(|row| row.id == Skill::CthulhuMythos)
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

    pub(crate) fn sanitize_allocations(&mut self) {
        let _ = self.sanitize_allocations_with_report();
    }

    pub(crate) fn sanitize_allocations_with_report(&mut self) -> SanitizeReport {
        let before_occupation = self.allocations.occupation_points.clone();
        let before_personal = self.allocations.personal_points.clone();
        let before_custom_occupation = self.allocations.custom_occupation_points.clone();
        let before_custom_personal = self.allocations.custom_personal_points.clone();
        let math = self.sheet_math();
        let final_chars = math.final_chars.clone();
        let enforce_total_budgets = self.has_all_chars();
        let mut occupation_points = HashMap::new();
        let mut personal_points = HashMap::new();
        let mut custom_occupation_points = HashMap::new();
        let mut custom_personal_points = HashMap::new();
        let mut active_custom_indices = HashSet::new();
        let valid_custom_indices: HashSet<usize> = self
            .custom_occupation
            .skills
            .iter()
            .enumerate()
            .filter_map(|(index, skill)| Skill::from_name(skill.trim()).map(|_| index))
            .collect();
        let mut remaining_occupation = math.occupation_budget.max(0);
        let mut remaining_personal = math.personal_budget.max(0);

        for row in math.skill_rows {
            if let Some(index) = row.custom_index {
                active_custom_indices.insert(index);
            }
            let occ_add = if enforce_total_budgets {
                let value = row.occ_add.min(remaining_occupation).max(0);
                remaining_occupation -= value;
                value
            } else {
                row.occ_add.max(0)
            };

            let personal_cap_after_occ = (MAX_CREATION_VALUE - row.base - occ_add).max(0);
            let personal_add = row.personal_add.min(personal_cap_after_occ).max(0);
            let personal_add = if enforce_total_budgets {
                let value = personal_add.min(remaining_personal);
                remaining_personal -= value;
                value
            } else {
                personal_add
            };

            if occ_add > 0 {
                if let Some(index) = row.custom_index {
                    custom_occupation_points.insert(index, occ_add);
                } else {
                    occupation_points.insert(row.id, occ_add);
                }
            }
            if personal_add > 0 {
                if let Some(index) = row.custom_index {
                    custom_personal_points.insert(index, personal_add);
                } else {
                    personal_points.insert(row.id, personal_add);
                }
            }
        }

        for (index, value) in &before_custom_occupation {
            if !active_custom_indices.contains(index)
                && valid_custom_indices.contains(index)
                && *value > 0
            {
                let Some(skill) = self
                    .custom_occupation
                    .skills
                    .get(*index)
                    .and_then(|skill| Skill::from_name(skill.trim()))
                else {
                    continue;
                };
                let base = get_base_skill_for(skill, &final_chars);
                let value = (*value).clamp(0, (MAX_CREATION_VALUE - base).max(0));
                if value > 0 {
                    custom_occupation_points.insert(*index, value);
                }
            }
        }
        for (index, value) in &before_custom_personal {
            if !active_custom_indices.contains(index)
                && valid_custom_indices.contains(index)
                && *value > 0
            {
                let Some(skill) = self
                    .custom_occupation
                    .skills
                    .get(*index)
                    .and_then(|skill| Skill::from_name(skill.trim()))
                else {
                    continue;
                };
                let base = get_base_skill_for(skill, &final_chars);
                let preserved_occ = custom_occupation_points.get(index).copied().unwrap_or(0);
                let value = (*value).clamp(0, (MAX_CREATION_VALUE - base - preserved_occ).max(0));
                if value > 0 {
                    custom_personal_points.insert(*index, value);
                }
            }
        }

        self.allocations.occupation_points = occupation_points;
        self.allocations.personal_points = personal_points;
        self.allocations.custom_occupation_points = custom_occupation_points;
        self.allocations.custom_personal_points = custom_personal_points;

        let mut report = SanitizeReport::default();
        record_allocation_changes(
            &mut report,
            "occupation",
            &before_occupation,
            &self.allocations.occupation_points,
        );
        record_allocation_changes(
            &mut report,
            "personal",
            &before_personal,
            &self.allocations.personal_points,
        );
        record_custom_allocation_changes(
            &mut report,
            "occupation",
            &before_custom_occupation,
            &self.allocations.custom_occupation_points,
        );
        record_custom_allocation_changes(
            &mut report,
            "personal",
            &before_custom_personal,
            &self.allocations.custom_personal_points,
        );
        report
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
        self.allocations.custom_occupation_points.clear();
    }

    pub(crate) fn clear_personal_allocations(&mut self) {
        self.allocations.personal_points.clear();
        self.allocations.custom_personal_points.clear();
    }

    fn allocation_row_for_instance(
        math: &SheetMath,
        skill: Skill,
        custom_index: Option<usize>,
    ) -> Option<&SkillRow> {
        math.skill_rows
            .iter()
            .find(|row| row.id == skill && row.custom_index == custom_index)
    }

    pub(crate) fn occupation_allocation_max_from(math: &SheetMath, row: &SkillRow) -> i32 {
        if math.selected_occupation.is_none()
            || !skill_accepts_occupation_points(row.id, &math.occupation_skill_set)
        {
            return 0;
        }

        let per_skill_cap = (MAX_CREATION_VALUE - row.base - row.personal_add).max(0);
        let used_without_this = Self::used_occupation_points_from(&math.skill_rows) - row.occ_add;
        let budget_cap = (math.occupation_budget - used_without_this).max(0);

        per_skill_cap.min(budget_cap)
    }

    pub(crate) fn personal_allocation_max_from(math: &SheetMath, row: &SkillRow) -> i32 {
        if !skill_accepts_personal_points(row.id.name()) {
            return 0;
        }

        let per_skill_cap = (MAX_CREATION_VALUE - row.base - row.occ_add).max(0);
        let used_without_this =
            Self::used_personal_points_from(&math.skill_rows) - row.personal_add;
        let budget_cap = (math.personal_budget - used_without_this).max(0);

        per_skill_cap.min(budget_cap)
    }

    pub(crate) fn occupation_allocation_max_for_instance(
        &self,
        skill: Skill,
        custom_index: Option<usize>,
    ) -> i32 {
        let math = self.sheet_math();
        let Some(row) = Self::allocation_row_for_instance(&math, skill, custom_index) else {
            return 0;
        };
        Self::occupation_allocation_max_from(&math, row)
    }

    pub(crate) fn personal_allocation_max_for_instance(
        &self,
        skill: Skill,
        custom_index: Option<usize>,
    ) -> i32 {
        let math = self.sheet_math();
        let Some(row) = Self::allocation_row_for_instance(&math, skill, custom_index) else {
            return 0;
        };
        Self::personal_allocation_max_from(&math, row)
    }

    #[cfg(test)]
    pub(crate) fn set_occupation_allocation_for(&mut self, skill_id: Skill, value: i32) {
        self.set_occupation_allocation_for_instance(skill_id, None, value);
    }

    #[cfg(test)]
    pub(crate) fn set_personal_allocation_for(&mut self, skill_id: Skill, value: i32) {
        self.set_personal_allocation_for_instance(skill_id, None, value);
    }

    pub(crate) fn set_occupation_allocation_for_instance(
        &mut self,
        skill_id: Skill,
        custom_index: Option<usize>,
        value: i32,
    ) {
        let max_value = self.occupation_allocation_max_for_instance(skill_id, custom_index);

        if let Some(index) = custom_index {
            if max_value > 0 {
                set_allocation(
                    &mut self.allocations.custom_occupation_points,
                    index,
                    value,
                    max_value,
                );
            } else {
                self.allocations.custom_occupation_points.remove(&index);
            }
        } else if max_value > 0 {
            set_allocation(
                &mut self.allocations.occupation_points,
                skill_id,
                value,
                max_value,
            );
        } else {
            self.allocations.occupation_points.remove(&skill_id);
        }
    }

    pub(crate) fn set_personal_allocation_for_instance(
        &mut self,
        skill_id: Skill,
        custom_index: Option<usize>,
        value: i32,
    ) {
        let max_value = self.personal_allocation_max_for_instance(skill_id, custom_index);

        if let Some(index) = custom_index {
            if max_value > 0 {
                set_allocation(
                    &mut self.allocations.custom_personal_points,
                    index,
                    value,
                    max_value,
                );
            } else {
                self.allocations.custom_personal_points.remove(&index);
            }
        } else if max_value > 0 {
            set_allocation(
                &mut self.allocations.personal_points,
                skill_id,
                value,
                max_value,
            );
        } else {
            self.allocations.personal_points.remove(&skill_id);
        }
    }

    #[cfg(test)]
    pub(crate) fn set_occupation_allocation(&mut self, skill: &str, value: i32) {
        let Some(skill_id) = Skill::from_name(skill) else {
            return;
        };
        self.set_occupation_allocation_for(skill_id, value);
    }

    #[cfg(test)]
    pub(crate) fn set_personal_allocation(&mut self, skill: &str, value: i32) {
        let Some(skill_id) = Skill::from_name(skill) else {
            return;
        };
        self.set_personal_allocation_for(skill_id, value);
    }
}
