use super::data::*;
use super::models::*;
use super::ruleset::SKILL_SPECS;
use std::collections::HashMap;

pub(crate) fn skill_accepts_personal_points(skill: &str) -> bool {
    skill != "Credit Rating" && skill != "Cthulhu Mythos"
}

pub(crate) fn skill_accepts_occupation_points(
    skill: Skill,
    allowed: &std::collections::HashSet<Skill>,
) -> bool {
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

impl CoC7eApp {
    pub(crate) fn skill_rows_for(
        &self,
        final_chars: &CharacteristicValues,
        occupation_skill_set: &std::collections::HashSet<Skill>,
    ) -> Vec<SkillRow> {
        SKILL_SPECS
            .iter()
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
        let math = self.sheet_math();
        let enforce_total_budgets = self.has_all_chars();
        let mut occupation_points = HashMap::new();
        let mut personal_points = HashMap::new();
        let mut remaining_occupation = math.occupation_budget.max(0);
        let mut remaining_personal = math.personal_budget.max(0);

        for row in math.skill_rows {
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
                occupation_points.insert(row.id, occ_add);
            }
            if personal_add > 0 {
                personal_points.insert(row.id, personal_add);
            }
        }

        self.allocations.occupation_points = occupation_points;
        self.allocations.personal_points = personal_points;
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
        let skill_id = Skill::from_name(skill)?;
        math.skill_rows.iter().find(|row| row.id == skill_id)
    }

    pub(crate) fn occupation_allocation_max_for(&self, skill: &str) -> i32 {
        let math = self.sheet_math();
        let Some(row) = Self::allocation_row_for(&math, skill) else {
            return 0;
        };
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

    pub(crate) fn personal_allocation_max_for(&self, skill: &str) -> i32 {
        let math = self.sheet_math();
        let Some(row) = Self::allocation_row_for(&math, skill) else {
            return 0;
        };
        if !skill_accepts_personal_points(row.name.as_str()) {
            return 0;
        }

        let per_skill_cap = (MAX_CREATION_VALUE - row.base - row.occ_add).max(0);
        let used_without_this =
            Self::used_personal_points_from(&math.skill_rows) - row.personal_add;
        let budget_cap = (math.personal_budget - used_without_this).max(0);

        per_skill_cap.min(budget_cap)
    }

    pub(crate) fn set_occupation_allocation(&mut self, skill: &str, value: i32) {
        let max_value = self.occupation_allocation_max_for(skill);
        let Some(skill_id) = Skill::from_name(skill) else {
            return;
        };

        if max_value > 0 {
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

    pub(crate) fn set_personal_allocation(&mut self, skill: &str, value: i32) {
        let max_value = self.personal_allocation_max_for(skill);
        let Some(skill_id) = Skill::from_name(skill) else {
            return;
        };

        if max_value > 0 {
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
}
