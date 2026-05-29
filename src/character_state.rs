use super::data::*;
use super::models::*;
use super::ruleset::*;
use std::collections::{HashMap, HashSet};

fn adjusted_final_characteristic(raw: i32, delta: i32) -> i32 {
    if raw <= 0 {
        0
    } else {
        (raw + delta).clamp(1, MAX_CREATION_VALUE)
    }
}

fn dice_result_matches_kind(result: &DiceResult, kind: DiceKind) -> bool {
    if !result.rolls.iter().all(|roll| (1..=6).contains(roll)) {
        return false;
    }

    match kind {
        DiceKind::ThreeD6 => {
            result.rolls.len() == 3
                && !result.plus_six
                && result.value == result.rolls.iter().sum::<u32>() as i32 * 5
        }
        DiceKind::TwoD6Plus6 => {
            result.rolls.len() == 2
                && result.plus_six
                && result.value == (result.rolls.iter().sum::<u32>() as i32 + 6) * 5
        }
    }
}

impl CoC7eApp {
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
        let sides = sides.max(1);
        if self.rng_roll_sides.len() >= MAX_RNG_ROLL_HISTORY {
            self.rng_seed = self.rng.reseed_from_stream();
            self.rng_roll_sides.clear();
        }
        self.rng_roll_sides.push(sides);
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
            let Some(def) = CHARACTERISTICS
                .iter()
                .find(|def| def.key.key() == key.as_str())
            else {
                return false;
            };
            let value = chars.get_char(def.key);
            value > 0
                && value == roll.value
                && roll.kept.is_none()
                && dice_result_matches_kind(roll, def.dice)
        });
    }

    pub(crate) fn sanitize_luck_state(&mut self) {
        let expected_attempts = if self.age_bracket().luck_advantage {
            2
        } else {
            1
        };

        let mut attempts: Vec<DiceResult> = self
            .luck_state
            .rolls
            .iter()
            .take(expected_attempts)
            .filter(|roll| dice_result_matches_kind(roll, DiceKind::ThreeD6))
            .cloned()
            .collect();

        if attempts.len() != expected_attempts {
            self.luck_state = LuckState::default();
            return;
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

    pub(crate) fn sanitize_edu_age_checks(&mut self) {
        let bracket = self.age_bracket();
        self.edu_check_rolls.truncate(bracket.edu_checks);

        if bracket.edu_checks == 0 || self.chars.get_char(Characteristic::Edu) <= 0 {
            self.edu_bonus = 0;
            self.edu_check_rolls.clear();
            return;
        }

        let starting_edu = (self.chars.get_char(Characteristic::Edu) - bracket.edu_penalty)
            .clamp(1, MAX_CREATION_VALUE);
        let mut current_edu = starting_edu;
        let mut sanitized = Vec::new();

        for roll in &self.edu_check_rolls {
            let d100 = roll.d100.clamp(1, 100);
            let improved = d100 > current_edu;
            let gain = if improved { roll.gain.clamp(1, 10) } else { 0 };
            current_edu = (current_edu + gain).clamp(1, MAX_CREATION_VALUE);

            sanitized.push(EduCheckRoll {
                d100,
                improved,
                gain,
                resulting_edu: current_edu,
            });
        }

        self.edu_bonus = (current_edu - starting_edu).max(0);
        self.edu_check_rolls = sanitized;
    }

    pub(crate) fn physical_deduction_source_label(&self) -> String {
        self.age_bracket()
            .physical_from
            .iter()
            .map(|key| key.key())
            .collect::<Vec<_>>()
            .join("/")
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
}
