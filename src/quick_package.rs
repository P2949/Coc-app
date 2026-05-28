use super::data::*;
use super::models::*;
use std::collections::HashMap;

fn add_occupation_package_points(
    canonical: &mut HashMap<Skill, i32>,
    custom: &mut HashMap<usize, i32>,
    row: &SkillRow,
    add: i32,
) {
    if add <= 0 {
        return;
    }

    if let Some(index) = row.custom_index {
        *custom.entry(index).or_insert(0) += add;
    } else {
        *canonical.entry(row.id).or_insert(0) += add;
    }
}

fn current_package_points(
    canonical: &HashMap<Skill, i32>,
    custom: &HashMap<usize, i32>,
    row: &SkillRow,
) -> i32 {
    match row.custom_index {
        Some(index) => custom.get(&index).copied().unwrap_or(0),
        None => canonical.get(&row.id).copied().unwrap_or(0),
    }
}

impl CoC7eApp {
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
        let math = self.sheet_math_from(final_chars.clone());
        let occupation_budget = self
            .active_formula_key_for(Some(&occupation))
            .calculate(&final_chars);
        let mut remaining_budget = occupation_budget.max(0);
        let mut skill_rows: Vec<SkillRow> = math
            .skill_rows
            .iter()
            .filter(|row| {
                row.id != Skill::CreditRating && math.occupation_skill_set.contains(&row.id)
            })
            .cloned()
            .collect();
        skill_rows.sort_by(|left, right| {
            left.base
                .cmp(&right.base)
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.custom_index.cmp(&right.custom_index))
        });

        let mut next = HashMap::new();
        let mut next_custom = HashMap::new();
        if occupation.credit.0 > 0 && remaining_budget > 0 {
            let credit_base = get_base_skill_for(Skill::CreditRating, &final_chars);
            let credit_cap = (MAX_CREATION_VALUE - credit_base).max(0);
            let credit_add = occupation.credit.0.min(remaining_budget).min(credit_cap);

            if credit_add > 0 {
                next.insert(Skill::CreditRating, credit_add);
                remaining_budget -= credit_add;
            }
        }

        for (row, target) in skill_rows.iter().zip(package_values) {
            if remaining_budget <= 0 {
                break;
            }

            let current_add = current_package_points(&next, &next_custom, row);
            let skill_cap = (MAX_CREATION_VALUE - row.base - row.personal_add - current_add).max(0);
            let target_add = (target - row.base - current_add).max(0);
            let add = target_add.min(skill_cap).min(remaining_budget);

            if add > 0 {
                add_occupation_package_points(&mut next, &mut next_custom, row, add);
                remaining_budget -= add;
            }
        }

        for row in &skill_rows {
            if remaining_budget <= 0 {
                break;
            }

            let current_add = current_package_points(&next, &next_custom, row);
            let skill_cap = (MAX_CREATION_VALUE - row.base - row.personal_add - current_add).max(0);
            let add = skill_cap.min(remaining_budget);

            if add > 0 {
                add_occupation_package_points(&mut next, &mut next_custom, row, add);
                remaining_budget -= add;
            }
        }

        self.allocations.occupation_points = next;
        self.allocations.custom_occupation_points = next_custom;
        self.sanitize_allocations();
    }
}
