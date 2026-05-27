use super::data::*;
use super::models::*;
use super::ruleset::BACKSTORY_CATEGORIES;
use std::collections::HashSet;

fn migrate_save_value(value: serde_json::Value) -> Result<InvestigatorSaveFile, String> {
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "save is missing numeric version".to_owned())?;

    if version != INVESTIGATOR_SAVE_VERSION as u64 {
        return Err(format!(
            "unsupported save version {version}; this app supports version {INVESTIGATOR_SAVE_VERSION}"
        ));
    }

    serde_json::from_value(value).map_err(|error| format!("could not parse JSON save: {error}"))
}

impl CoC7eApp {
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
            char_rolls: self
                .char_rolls
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            luck_state: self.luck_state.clone(),
            age_deductions: self.age_deductions.clone(),
            edu_bonus: self.edu_bonus,
            edu_check_rolls: self.edu_check_rolls.clone(),
            occupation_id: self.occupation_id.clone(),
            formula_key: self.formula_key,
            occupation_choices,
            custom_occupation: self.custom_occupation.clone(),
            allocations: self.allocations.clone(),
            backstory: self
                .backstory
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
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

        let value: serde_json::Value = serde_json::from_str(trimmed)
            .map_err(|error| format!("could not parse JSON save: {error}"))?;
        let save = migrate_save_value(value)?;
        self.load_save_file(save);
        Ok(())
    }

    pub(crate) fn load_save_file(&mut self, save: InvestigatorSaveFile) {
        self.concept = save.concept;
        self.concept.age = self.concept.age.clamp(15, 89);
        self.last_age_bracket_index = get_age_bracket_index(self.concept.age);
        self.char_method = save.char_method;
        self.chars = save.chars;
        self.char_rolls = save.char_rolls.into_iter().collect();
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

        self.sanitize_state();
        self.refresh_reachability();
    }
}
