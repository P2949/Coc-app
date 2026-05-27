use super::allocations::sanitized_allocation_value;
use super::data::*;
use super::models::*;
use std::collections::HashMap;

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
        let occupation_budget = self
            .active_formula_key_for(Some(&occupation))
            .calculate(&final_chars);
        let mut remaining_budget = occupation_budget.max(0);
        let mut skill_order = self.resolved_occupation_skills_for(&occupation);
        skill_order.sort_by(|left, right| {
            get_base_skill_for(*left, &final_chars)
                .cmp(&get_base_skill_for(*right, &final_chars))
                .then_with(|| left.name().cmp(right.name()))
        });

        let mut next = HashMap::new();
        if occupation.credit.0 > 0 && remaining_budget > 0 {
            let credit_base = get_base_skill_for(Skill::CreditRating, &final_chars);
            let credit_cap = (MAX_CREATION_VALUE - credit_base).max(0);
            let credit_add = occupation.credit.0.min(remaining_budget).min(credit_cap);

            if credit_add > 0 {
                next.insert(Skill::CreditRating, credit_add);
                remaining_budget -= credit_add;
            }
        }

        for (skill, target) in skill_order.iter().zip(package_values) {
            if remaining_budget <= 0 {
                break;
            }

            let skill_id = *skill;
            let base = get_base_skill_for(skill_id, &final_chars);
            let personal_add = sanitized_allocation_value(
                &self.allocations.personal_points,
                skill_id,
                MAX_CREATION_VALUE - base,
            );
            let current_add = next.get(&skill_id).copied().unwrap_or(0);
            let skill_cap = (MAX_CREATION_VALUE - base - personal_add - current_add).max(0);
            let target_add = (target - base - current_add).max(0);
            let add = target_add.min(skill_cap).min(remaining_budget);

            if add > 0 {
                *next.entry(skill_id).or_insert(0) += add;
                remaining_budget -= add;
            }
        }

        for skill in &skill_order {
            if remaining_budget <= 0 {
                break;
            }

            let skill_id = *skill;
            let base = get_base_skill_for(skill_id, &final_chars);
            let personal_add = sanitized_allocation_value(
                &self.allocations.personal_points,
                skill_id,
                MAX_CREATION_VALUE - base,
            );
            let current_add = next.get(&skill_id).copied().unwrap_or(0);
            let skill_cap = (MAX_CREATION_VALUE - base - personal_add - current_add).max(0);
            let add = skill_cap.min(remaining_budget);

            if add > 0 {
                *next.entry(skill_id).or_insert(0) += add;
                remaining_budget -= add;
            }
        }

        self.allocations.occupation_points = next;
        self.sanitize_allocations();
    }
}
