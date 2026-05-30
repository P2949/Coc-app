use super::data::*;
use super::models::*;
use super::ruleset::BACKSTORY_CATEGORIES;
use std::collections::HashSet;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

fn migrate_legacy_to_current(value: &mut serde_json::Value) {
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "version".to_owned(),
            serde_json::Value::from(INVESTIGATOR_SAVE_VERSION),
        );
    }
}

fn normalize_imported_rng_roll_history(roll_sides: Vec<u32>) -> (Vec<u32>, bool) {
    let was_truncated = roll_sides.len() > MAX_RNG_ROLL_HISTORY;
    let mut normalized = was_truncated;
    let replay_sides = roll_sides
        .into_iter()
        .take(MAX_RNG_ROLL_HISTORY)
        .filter(|sides| {
            if *sides > 0 {
                true
            } else {
                normalized = true;
                false
            }
        })
        .collect();

    (replay_sides, normalized)
}

fn path_uses_unexpanded_home(path: &Path) -> bool {
    let path = path.as_os_str().to_string_lossy();
    path == "~" || path.starts_with("~/") || path.starts_with("~\\")
}

fn validate_json_path_for_save(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("enter a save path before saving".to_owned());
    }
    if path_uses_unexpanded_home(path) {
        return Err(
            "~ is not expanded in save paths; use an absolute path or a relative path without ~"
                .to_owned(),
        );
    }
    if path.is_dir() {
        return Err(format!(
            "save path points to a directory; choose a JSON file path: {}",
            path.display()
        ));
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        return Err(format!(
            "save directory does not exist: {}",
            parent.display()
        ));
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && parent.exists()
        && !parent.is_dir()
    {
        return Err(format!(
            "save parent path is not a directory: {}",
            parent.display()
        ));
    }
    Ok(())
}

fn validate_json_path_for_load(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("enter a save path before loading".to_owned());
    }
    if path_uses_unexpanded_home(path) {
        return Err(
            "~ is not expanded in load paths; use an absolute path or a relative path without ~"
                .to_owned(),
        );
    }
    if !path.exists() {
        return Err(format!("save file does not exist: {}", path.display()));
    }
    if path.is_dir() {
        return Err(format!(
            "load path points to a directory; choose a JSON file path: {}",
            path.display()
        ));
    }
    Ok(())
}

fn save_temp_dir_for(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

#[cfg(unix)]
fn sync_parent_dir_after_save(path: &Path) {
    let temp_dir = save_temp_dir_for(path);
    let _ = std::fs::File::open(temp_dir).and_then(|dir| dir.sync_all());
}

#[cfg(not(unix))]
fn sync_parent_dir_after_save(_path: &Path) {}

fn write_json_atomically(path: &Path, json: &str) -> Result<(), String> {
    let temp_dir = save_temp_dir_for(path);
    let mut temp = NamedTempFile::new_in(temp_dir).map_err(|error| {
        format!(
            "could not create temporary save file in {}: {error}",
            temp_dir.display()
        )
    })?;

    temp.write_all(json.as_bytes()).map_err(|error| {
        format!(
            "could not write temporary JSON save for {}: {error}",
            path.display()
        )
    })?;
    temp.as_file().sync_all().map_err(|error| {
        format!(
            "could not flush temporary JSON save for {}: {error}",
            path.display()
        )
    })?;
    temp.persist(path).map_err(|error| {
        format!(
            "could not replace JSON save at {}: {}",
            path.display(),
            error.error
        )
    })?;
    sync_parent_dir_after_save(path);
    Ok(())
}

fn parse_i32_import_report_value(value: &serde_json::Value) -> Option<i32> {
    value
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())
        .or_else(|| value.as_str()?.trim().parse::<i32>().ok())
}

fn import_value_is_i32(value: &serde_json::Value) -> bool {
    parse_i32_import_report_value(value).is_some()
}

fn import_value_is_non_negative_usize(value: &serde_json::Value) -> bool {
    if let Some(value) = value.as_i64() {
        return value >= 0 && usize::try_from(value).is_ok();
    }

    value.as_str().is_some_and(|value| {
        value
            .trim()
            .parse::<i64>()
            .is_ok_and(|value| value >= 0 && usize::try_from(value).is_ok())
    })
}

fn import_value_is_u64(value: &serde_json::Value) -> bool {
    value.as_u64().is_some()
        || value
            .as_str()
            .is_some_and(|value| value.trim().parse::<u64>().is_ok())
}

fn import_value_is_bool(value: &serde_json::Value) -> bool {
    value.as_bool().is_some()
        || value
            .as_str()
            .is_some_and(|value| value.trim().parse::<bool>().is_ok())
}

fn import_value_is_positive_u32(value: &serde_json::Value) -> bool {
    value
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .is_some_and(|value| value > 0)
}

fn dice_result_is_valid(value: &serde_json::Value) -> bool {
    serde_json::from_value::<DiceResult>(value.clone()).is_ok()
}

fn report_invalid_string_field(
    object: &serde_json::Map<String, serde_json::Value>,
    prefix: &str,
    field: &str,
    unknown: &mut Vec<String>,
) {
    if let Some(raw) = object.get(field)
        && !raw.is_string()
    {
        unknown.push(format!("{prefix}.{field}: expected string"));
    }
}

fn report_invalid_characteristic_values(
    field: &str,
    raw: &serde_json::Value,
    unknown: &mut Vec<String>,
) {
    if let Some(values) = raw.as_array() {
        if values.len() != Characteristic::COUNT {
            unknown.push(format!(
                "{field}: expected {} ordered values, got {}",
                Characteristic::COUNT,
                values.len()
            ));
        }
        for (index, value) in values.iter().take(Characteristic::COUNT).enumerate() {
            if !import_value_is_i32(value) {
                unknown.push(format!("{field}[{index}]: expected integer"));
            }
        }
        return;
    }

    let Some(object) = raw.as_object() else {
        if !raw.is_null() {
            unknown.push(format!("{field}: expected characteristic object or array"));
        }
        return;
    };

    if let Some(values) = object.get("values") {
        if object.keys().any(|key| key != "values") {
            unknown.push(format!(
                "{field}: legacy values object cannot be mixed with named fields"
            ));
        }
        if let Some(values) = values.as_array() {
            if values.len() != Characteristic::COUNT {
                unknown.push(format!(
                    "{field}.values: expected {} ordered values, got {}",
                    Characteristic::COUNT,
                    values.len()
                ));
            }
            for (index, value) in values.iter().take(Characteristic::COUNT).enumerate() {
                if !import_value_is_i32(value) {
                    unknown.push(format!("{field}.values[{index}]: expected integer"));
                }
            }
        } else if !values.is_null() {
            unknown.push(format!("{field}.values: expected array"));
        }
        return;
    }

    for (key, value) in object {
        if Characteristic::from_key(key).is_none() {
            unknown.push(format!("{field}: unknown characteristic {key}"));
        } else if !import_value_is_i32(value) {
            unknown.push(format!("{field}.{key}: expected integer"));
        }
    }
}

fn report_invalid_dice_result_map(field: &str, raw: &serde_json::Value, unknown: &mut Vec<String>) {
    let Some(object) = raw.as_object() else {
        if !raw.is_null() {
            unknown.push(format!("{field}: expected object"));
        }
        return;
    };

    for (key, value) in object {
        if !dice_result_is_valid(value) {
            unknown.push(format!("{field}[{key}]: malformed roll evidence"));
        }
    }
}

fn report_invalid_luck_state(raw: &serde_json::Value, unknown: &mut Vec<String>) {
    let Some(object) = raw.as_object() else {
        if !raw.is_null() {
            unknown.push("luck_state: expected object".to_owned());
        }
        return;
    };

    if let Some(value) = object.get("value")
        && !value.is_null()
        && !import_value_is_i32(value)
    {
        unknown.push("luck_state.value: expected integer or null".to_owned());
    }

    if let Some(rolls) = object.get("rolls") {
        if let Some(rolls) = rolls.as_array() {
            for (index, roll) in rolls.iter().enumerate() {
                if !dice_result_is_valid(roll) {
                    unknown.push(format!(
                        "luck_state.rolls[{index}]: malformed roll evidence"
                    ));
                }
            }
        } else if !rolls.is_null() {
            unknown.push("luck_state.rolls: expected array".to_owned());
        }
    }
}

fn report_invalid_edu_check_rolls(raw: &serde_json::Value, unknown: &mut Vec<String>) {
    let Some(rolls) = raw.as_array() else {
        if !raw.is_null() {
            unknown.push("edu_check_rolls: expected array".to_owned());
        }
        return;
    };

    for (index, roll) in rolls.iter().enumerate() {
        let Some(roll) = roll.as_object() else {
            unknown.push(format!("edu_check_rolls[{index}]: expected object"));
            continue;
        };
        match roll.get("d100").and_then(parse_i32_import_report_value) {
            Some(d100) if !(1..=100).contains(&d100) => {
                unknown.push(format!("edu_check_rolls[{index}].d100: expected 1..=100"));
            }
            Some(_) => {}
            None if roll.get("d100").is_some() => {
                unknown.push(format!("edu_check_rolls[{index}].d100: expected integer"));
            }
            None => {
                unknown.push(format!("edu_check_rolls[{index}]: missing required d100"));
            }
        }
        match roll.get("gain").and_then(parse_i32_import_report_value) {
            Some(gain) if !(0..=10).contains(&gain) => {
                unknown.push(format!("edu_check_rolls[{index}].gain: expected 0..=10"));
            }
            Some(_) => {}
            None if roll.get("gain").is_some() => {
                unknown.push(format!("edu_check_rolls[{index}].gain: expected integer"));
            }
            None => {
                unknown.push(format!("edu_check_rolls[{index}]: missing required gain"));
            }
        }
        if let Some(raw) = roll.get("resulting_edu")
            && !import_value_is_i32(raw)
        {
            unknown.push(format!(
                "edu_check_rolls[{index}].resulting_edu: expected integer"
            ));
        }
        if let Some(raw) = roll.get("improved")
            && !import_value_is_bool(raw)
        {
            unknown.push(format!("edu_check_rolls[{index}].improved: expected bool"));
        }
    }
}

fn report_invalid_string_map(field: &str, raw: &serde_json::Value, unknown: &mut Vec<String>) {
    let Some(object) = raw.as_object() else {
        if !raw.is_null() {
            unknown.push(format!("{field}: expected object"));
        }
        return;
    };

    for (key, value) in object {
        if !value.is_string() {
            unknown.push(format!("{field}[{key}]: expected string"));
        }
    }
}

fn allocation_value_is_i32(value: &serde_json::Value) -> bool {
    value
        .as_i64()
        .is_some_and(|value| i32::try_from(value).is_ok())
}

fn invalid_import_entries(value: &serde_json::Value) -> Vec<String> {
    let mut unknown = Vec::new();

    if let Some(raw_concept) = value.get("concept") {
        if let Some(concept) = raw_concept.as_object() {
            for field in ["name", "pronouns", "residence", "birthplace"] {
                report_invalid_string_field(concept, "concept", field, &mut unknown);
            }
            if let Some(raw) = concept.get("age")
                && !import_value_is_i32(raw)
            {
                unknown.push("concept.age: expected integer".to_owned());
            }
        } else if !raw_concept.is_null() {
            unknown.push("concept: expected object".to_owned());
        }
    }

    if let Some(raw) = value.get("char_method")
        && serde_json::from_value::<CharMethod>(raw.clone()).is_err()
    {
        unknown.push("char_method: unknown method".to_owned());
    }
    if let Some(raw) = value.get("chars") {
        report_invalid_characteristic_values("chars", raw, &mut unknown);
    }
    if let Some(raw) = value.get("char_rolls") {
        report_invalid_dice_result_map("char_rolls", raw, &mut unknown);
    }
    if let Some(raw) = value.get("rng_seed")
        && !import_value_is_u64(raw)
    {
        unknown.push("rng_seed: expected non-negative integer".to_owned());
    }
    if let Some(raw) = value.get("rng_roll_sides") {
        if let Some(sides) = raw.as_array() {
            for (index, side) in sides.iter().enumerate() {
                if !import_value_is_positive_u32(side) {
                    unknown.push(format!(
                        "rng_roll_sides[{index}]: expected positive integer"
                    ));
                }
            }
        } else if !raw.is_null() {
            unknown.push("rng_roll_sides: expected array".to_owned());
        }
    }
    if let Some(raw) = value.get("luck_state") {
        report_invalid_luck_state(raw, &mut unknown);
    }
    if let Some(raw) = value.get("age_deductions") {
        report_invalid_characteristic_values("age_deductions", raw, &mut unknown);
    }
    if let Some(raw) = value.get("edu_bonus")
        && !import_value_is_i32(raw)
    {
        unknown.push("edu_bonus: expected integer".to_owned());
    }
    if let Some(raw) = value.get("edu_check_rolls") {
        report_invalid_edu_check_rolls(raw, &mut unknown);
    }
    if let Some(raw) = value.get("occupation_id")
        && !raw.is_string()
    {
        unknown.push("occupation_id: expected string".to_owned());
    }
    if let Some(raw) = value.get("formula_key")
        && serde_json::from_value::<FormulaKey>(raw.clone()).is_err()
    {
        unknown.push("formula_key: unknown formula".to_owned());
    }
    if let Some(raw) = value.get("backstory") {
        report_invalid_string_map("backstory", raw, &mut unknown);
    }

    if let Some(allocations) = value.get("allocations") {
        if !allocations.is_object() && !allocations.is_null() {
            unknown.push("allocations: expected object".to_owned());
        }
        for field in ["occupation_points", "personal_points"] {
            let Some(raw_points) = allocations.get(field) else {
                continue;
            };
            let Some(points) = raw_points.as_object() else {
                if !raw_points.is_null() {
                    unknown.push(format!("{field}: expected object"));
                }
                continue;
            };
            for (skill, value) in points {
                if Skill::from_name(skill).is_none() {
                    unknown.push(format!("{field}: {skill}"));
                }
                if !allocation_value_is_i32(value) {
                    unknown.push(format!("{field}[{skill}]: non-integer allocation value"));
                }
            }
        }

        for field in ["custom_occupation_points", "custom_personal_points"] {
            let Some(raw_points) = allocations.get(field) else {
                continue;
            };
            let Some(points) = raw_points.as_object() else {
                if !raw_points.is_null() {
                    unknown.push(format!("{field}: expected object"));
                }
                continue;
            };
            for (slot, value) in points {
                if slot.parse::<usize>().is_err() {
                    unknown.push(format!("{field}: {slot}"));
                }
                if !allocation_value_is_i32(value) {
                    unknown.push(format!("{field}[{slot}]: non-integer allocation value"));
                }
            }
        }
    }

    if let Some(raw_custom_occupation) = value.get("custom_occupation") {
        if let Some(custom_occupation) = raw_custom_occupation.as_object() {
            for field in ["credit_min", "credit_max"] {
                if let Some(raw) = custom_occupation.get(field)
                    && !import_value_is_i32(raw)
                {
                    unknown.push(format!("custom_occupation.{field}: expected integer"));
                }
            }
            if let Some(raw) = custom_occupation.get("name")
                && !raw.is_string()
            {
                unknown.push("custom_occupation.name: expected string".to_owned());
            }
            if let Some(raw) = custom_occupation.get("formula_key")
                && serde_json::from_value::<FormulaKey>(raw.clone()).is_err()
            {
                unknown.push("custom_occupation.formula_key: unknown formula".to_owned());
            }
            if let Some(raw) = custom_occupation.get("required_skill_count")
                && !import_value_is_non_negative_usize(raw)
            {
                unknown.push(
                    "custom_occupation.required_skill_count: expected non-negative integer"
                        .to_owned(),
                );
            }
            if let Some(raw_skills) = custom_occupation.get("skills") {
                if let Some(skills) = raw_skills.as_array() {
                    for (index, skill) in skills.iter().enumerate() {
                        if !skill.is_string() {
                            unknown.push(format!(
                                "custom_occupation.skills[{index}]: non-string skill"
                            ));
                        }
                    }
                } else if !raw_skills.is_null() {
                    unknown.push("custom_occupation.skills: expected array".to_owned());
                }
            }

            if let Some(raw_labels) = custom_occupation.get("skill_labels") {
                if let Some(labels) = raw_labels.as_object() {
                    for (skill, label) in labels {
                        if !label.is_string() {
                            unknown.push(format!("skill_labels[{skill}]: non-string label"));
                        }
                    }
                } else if !raw_labels.is_null() {
                    unknown.push("skill_labels: expected object".to_owned());
                }
            }

            if let Some(raw_labels) = custom_occupation.get("skill_slot_labels") {
                if let Some(labels) = raw_labels.as_object() {
                    for (slot, label) in labels {
                        if slot.parse::<usize>().is_err() {
                            unknown.push(format!("skill_slot_labels: {slot}"));
                        } else if !label.is_string() {
                            unknown.push(format!("skill_slot_labels[{slot}]: non-string label"));
                        }
                    }
                } else if !raw_labels.is_null() {
                    unknown.push("skill_slot_labels: expected object".to_owned());
                }
            }
        } else if !raw_custom_occupation.is_null() {
            unknown.push("custom_occupation: expected object".to_owned());
        }
    }

    if let Some(raw_choices) = value.get("occupation_choices") {
        if let Some(choices) = raw_choices.as_array() {
            for (index, choice) in choices.iter().enumerate() {
                let Some(choice) = choice.as_object() else {
                    unknown.push(format!("occupation_choices[{index}]: expected object"));
                    continue;
                };

                if let Some(raw) = choice.get("id")
                    && !raw.is_string()
                {
                    unknown.push(format!("occupation_choices[{index}].id: expected string"));
                }
                if let Some(raw) = choice.get("index")
                    && !import_value_is_non_negative_usize(raw)
                {
                    unknown.push(format!(
                        "occupation_choices[{index}].index: expected non-negative integer"
                    ));
                }
                if let Some(raw) = choice.get("value")
                    && !raw.is_string()
                {
                    unknown.push(format!(
                        "occupation_choices[{index}].value: expected string"
                    ));
                }
            }
        } else if !raw_choices.is_null() {
            unknown.push("occupation_choices: expected array".to_owned());
        }
    }

    unknown
}

fn migrate_save_value(
    mut value: serde_json::Value,
) -> Result<(InvestigatorSaveFile, Vec<String>), String> {
    let version = match value.get("version") {
        None => 0,
        Some(raw) => raw
            .as_u64()
            .ok_or_else(|| "save version must be a non-negative integer".to_owned())?,
    };

    match version {
        0 | 1 => migrate_legacy_to_current(&mut value),
        current if current == INVESTIGATOR_SAVE_VERSION as u64 => {}
        unsupported => {
            return Err(format!(
                "unsupported save version {unsupported}; this app supports up to version {INVESTIGATOR_SAVE_VERSION}"
            ));
        }
    }

    let invalid_entries = invalid_import_entries(&value);
    let save = serde_json::from_value(value)
        .map_err(|error| format!("could not parse JSON save: {error}"))?;
    Ok((save, invalid_entries))
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
            rng_roll_sides: self.rng_roll_sides.clone(),
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
        validate_json_path_for_save(path)?;
        let json = self
            .export_json_save()
            .map_err(|error| format!("could not build JSON save: {error}"))?;
        write_json_atomically(path, &json)
    }

    pub(crate) fn load_json_from_path(&mut self, path: &Path) -> Result<SanitizeReport, String> {
        validate_json_path_for_load(path)?;
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
        let (save, invalid_entries) = migrate_save_value(value)?;
        let mut report = self.load_save_file(save);
        report
            .removed_unknown_import_entries
            .extend(invalid_entries);
        Ok(report)
    }

    pub(crate) fn load_save_file(&mut self, save: InvestigatorSaveFile) -> SanitizeReport {
        let mut normalized_import_fields = Vec::new();
        let imported_age = save.concept.age;
        self.concept = save.concept;
        self.concept.age = self.concept.age.clamp(15, 89);
        if imported_age != self.concept.age {
            normalized_import_fields.push(format!("age: {imported_age} → {}", self.concept.age));
        }
        self.last_age_bracket_index = get_age_bracket_index(self.concept.age);
        self.char_method = save.char_method;
        self.chars = save.chars;
        self.char_rolls = save.char_rolls.into_iter().collect();
        self.rng_seed = if save.rng_seed == 0 {
            DEFAULT_RNG_SEED
        } else {
            save.rng_seed
        };
        let mut rng = AppRng::seeded(self.rng_seed);
        let (replay_sides, normalized_rng_history) =
            normalize_imported_rng_roll_history(save.rng_roll_sides);
        let normalized_rng_state = save.rng_seed == 0 || normalized_rng_history;
        for sides in &replay_sides {
            let _ = rng.roll_inclusive(*sides);
        }
        self.rng_roll_sides = replay_sides;
        self.rng = rng;
        self.luck_state = save.luck_state;
        self.age_deductions = save.age_deductions;
        let imported_edu_bonus = save.edu_bonus;
        self.edu_bonus = save.edu_bonus.clamp(0, MAX_CREATION_VALUE);
        if imported_edu_bonus != self.edu_bonus {
            normalized_import_fields.push(format!(
                "EDU bonus: {imported_edu_bonus} → {}",
                self.edu_bonus
            ));
        }
        self.edu_check_rolls = save.edu_check_rolls;
        let imported_occupation_id = save.occupation_id.clone();
        let occupation_is_known = save.occupation_id == CUSTOM_OCCUPATION_ID
            || self
                .occupations
                .iter()
                .any(|occupation| occupation.name == save.occupation_id);
        self.occupation_id = if occupation_is_known {
            save.occupation_id
        } else {
            if !imported_occupation_id.trim().is_empty() {
                normalized_import_fields.push(format!("occupation: {imported_occupation_id}"));
            }
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
        let mut removed_backstory_categories = Vec::new();
        self.backstory = save
            .backstory
            .into_iter()
            .filter_map(|(category, value)| {
                let trimmed_category = category.trim();
                if !allowed_backstory.contains(trimmed_category) || value.trim().is_empty() {
                    removed_backstory_categories.push(category);
                    None
                } else {
                    Some((trimmed_category.to_owned(), value))
                }
            })
            .collect();

        let mut report = self.sanitize_state_with_report();
        report.normalized_rng_state |= normalized_rng_state;
        report
            .removed_backstory_categories
            .extend(removed_backstory_categories);
        report
            .normalized_import_fields
            .extend(normalized_import_fields);
        self.refresh_reachability();
        report
    }
}
