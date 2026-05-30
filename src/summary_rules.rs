use super::allocations::{sanitized_allocation_value, skill_accepts_occupation_points};
use super::data::*;
use super::models::*;
use std::collections::HashSet;

impl CoC7eApp {
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
        occupation_skill_set: &HashSet<Skill>,
    ) -> i32 {
        let occupation_add =
            if skill_accepts_occupation_points(Skill::CreditRating, occupation_skill_set) {
                sanitized_allocation_value(
                    &self.allocations.occupation_points,
                    Skill::CreditRating,
                    MAX_CREATION_VALUE - get_base_skill_for(Skill::CreditRating, final_chars),
                )
            } else {
                0
            };

        get_base_skill_for(Skill::CreditRating, final_chars) + occupation_add
    }

    pub(crate) fn summary_generation_method_check(&self) -> (CheckState, String) {
        match self.char_method {
            CharMethod::PointBuy => {
                let point_spent = self.point_buy_spent();
                (
                    if point_spent == POINT_BUY_BUDGET {
                        CheckState::Pass
                    } else {
                        CheckState::Fail
                    },
                    format!("Point budget {point_spent}/{POINT_BUY_BUDGET}"),
                )
            }
            CharMethod::Roll => (CheckState::Pass, "Generation method Roll".to_owned()),
            CharMethod::QuickArray => {
                (CheckState::Pass, "Generation method Quick Array".to_owned())
            }
            CharMethod::Mixed => (CheckState::Warn, "Generation method Mixed".to_owned()),
        }
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
                    "age deductions impossible: requires {}, current {} can absorb only {}",
                    bracket.physical_deduct,
                    self.physical_deduction_source_label(),
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
        if has_occupation && used_occ != math.occupation_budget {
            let occupation_capacity = Self::occupation_budget_capacity_from(math);
            if math.occupation_budget > occupation_capacity {
                blockers.push(format!(
                    "occupation points impossible: budget {} exceeds current skill cap \
                    {occupation_capacity}; add custom skills or lower the occupation formula",
                    math.occupation_budget
                ));
            } else {
                blockers.push(format!(
                    "occupation points {used_occ}/{}",
                    math.occupation_budget
                ));
            }
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
        // current physical source values, Summary still unlocks so the blocker can
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
}
