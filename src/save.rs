use super::data::*;
use super::models::*;
use super::ruleset::BACKSTORY_CATEGORIES;
use std::collections::HashSet;
use std::path::Path;

fn migrate_v0_to_v1(value: &mut serde_json::Value) {
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "version".to_owned(),
            serde_json::Value::from(INVESTIGATOR_SAVE_VERSION),
        );
    }
}

fn unknown_allocation_skills(value: &serde_json::Value) -> Vec<String> {
    let mut unknown = Vec::new();
    for field in ["occupation_points", "personal_points"] {
        let Some(points) = value
            .get("allocations")
            .and_then(|allocations| allocations.get(field))
            .and_then(serde_json::Value::as_object)
        else {
            continue;
        };
        for skill in points.keys() {
            if Skill::from_name(skill).is_none() {
                unknown.push(format!("{field}: {skill}"));
            }
        }
    }
    unknown
}

fn migrate_save_value(
    mut value: serde_json::Value,
) -> Result<(InvestigatorSaveFile, Vec<String>), String> {
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);

    match version {
        0 => migrate_v0_to_v1(&mut value),
        current if current == INVESTIGATOR_SAVE_VERSION as u64 => {}
        unsupported => {
            return Err(format!(
                "unsupported save version {unsupported}; this app supports up to version {INVESTIGATOR_SAVE_VERSION}"
            ));
        }
    }

    let unknown_skills = unknown_allocation_skills(&value);
    let save = serde_json::from_value(value)
        .map_err(|error| format!("could not parse JSON save: {error}"))?;
    Ok((save, unknown_skills))
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
            rng_seed: self.rng_seed,
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

    pub(crate) fn save_json_to_path(&self, path: &Path) -> Result<(), String> {
        if path.as_os_str().is_empty() {
            return Err("enter a save path before saving".to_owned());
        }
        let json = self
            .export_json_save()
            .map_err(|error| format!("could not build JSON save: {error}"))?;
        std::fs::write(path, json)
            .map_err(|error| format!("could not write JSON save to {}: {error}", path.display()))
    }

    pub(crate) fn load_json_from_path(&mut self, path: &Path) -> Result<SanitizeReport, String> {
        if path.as_os_str().is_empty() {
            return Err("enter a save path before loading".to_owned());
        }
        let input = std::fs::read_to_string(path).map_err(|error| {
            format!("could not read JSON save from {}: {error}", path.display())
        })?;
        self.import_json_save(&input)
    }

    pub(crate) fn import_json_save(&mut self, input: &str) -> Result<SanitizeReport, String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err("paste a JSON save before loading".to_owned());
        }

        let value: serde_json::Value = serde_json::from_str(trimmed)
            .map_err(|error| format!("could not parse JSON save: {error}"))?;
        let (save, unknown_skills) = migrate_save_value(value)?;
        let mut report = self.load_save_file(save);
        report.removed_unknown_skills.extend(unknown_skills);
        Ok(report)
    }

    pub(crate) fn load_save_file(&mut self, save: InvestigatorSaveFile) -> SanitizeReport {
        self.concept = save.concept;
        self.concept.age = self.concept.age.clamp(15, 89);
        self.last_age_bracket_index = get_age_bracket_index(self.concept.age);
        self.char_method = save.char_method;
        self.chars = save.chars;
        self.char_rolls = save.char_rolls.into_iter().collect();
        self.rng_seed = if save.rng_seed == 0 {
            DEFAULT_RNG_SEED
        } else {
            save.rng_seed | 1
        };
        self.rng = AppRng::seeded(self.rng_seed);
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

        let report = self.sanitize_state_with_report();
        self.refresh_reachability();
        report
    }
}
