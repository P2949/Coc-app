use super::data::*;
use super::models::*;
use super::occupations::*;
use super::ruleset::*;
use std::collections::HashSet;

fn test_app() -> CoC7eApp {
    let occupations = build_occupations();
    validate_occupations(&occupations);
    CoC7eApp::fresh(occupations, 0xC0C7_E7E5_0000_0001)
}

fn resolve_nurse_choice(app: &mut CoC7eApp) {
    app.occupation_choices.insert(
        ChoiceKey::new("nurse-interpersonal", 0),
        "Persuade".to_owned(),
    );
}

fn characteristic_values(pairs: &[(&str, i32)]) -> CharacteristicValues {
    let mut values = CharacteristicValues::default();
    for (key, value) in pairs {
        let characteristic = Characteristic::from_key(key)
            .unwrap_or_else(|| panic!("unknown characteristic key in test preset: {key}"));
        values.set_char(characteristic, *value);
    }
    values
}

#[test]
fn half_and_fifth_round_down() {
    assert_eq!(floor_half(51), 25);
    assert_eq!(floor_fifth(54), 10);
}

#[test]
fn manual_characteristic_values_snap_to_multiples_of_five() {
    let mut app = test_app();

    app.set_char_value("STR", 52);
    assert_eq!(app.char_value("STR"), 50);

    app.set_char_value("STR", 53);
    assert_eq!(app.char_value("STR"), 55);

    app.set_char_value("STR", 999);
    assert_eq!(app.char_value("STR"), 90);
}

#[test]
fn step_five_snapping_handles_age_deductions() {
    assert_eq!(clamp_step_5(3, 0, 20), 5);
    assert_eq!(clamp_step_5(2, 0, 20), 0);
    assert_eq!(clamp_step_5(999, 0, 20), 20);
}

#[test]
fn snap_to_step_rounds_midpoints_consistently() {
    assert_eq!(snap_to_step(2, 5), 0);
    assert_eq!(snap_to_step(3, 5), 5);
    assert_eq!(snap_to_step(7, 5), 5);
    assert_eq!(snap_to_step(8, 5), 10);
}

#[test]
fn damage_bonus_boundary_values() {
    assert_eq!(get_damage_bonus(0, 0).db, "−2");
    assert_eq!(get_damage_bonus(32, 32).db, "−2");
    assert_eq!(get_damage_bonus(33, 32).db, "−1");
    assert_eq!(get_damage_bonus(42, 42).db, "−1");
    assert_eq!(get_damage_bonus(43, 42).db, "None");
    assert_eq!(get_damage_bonus(62, 62).db, "None");
    assert_eq!(get_damage_bonus(63, 62).db, "+1D4");
}

#[test]
fn movement_rate_core_cases() {
    let bracket = get_age_bracket(30);
    assert_eq!(get_movement_rate(40, 40, 60, bracket), 7);
    assert_eq!(get_movement_rate(50, 60, 50, bracket), 8);
    assert_eq!(get_movement_rate(70, 70, 50, bracket), 9);
    assert_eq!(get_movement_rate(70, 70, 50, get_age_bracket(45)), 8);
}

#[test]
fn derived_stats_formulas() {
    let chars = characteristic_values(&[
        ("STR", 50),
        ("CON", 60),
        ("SIZ", 70),
        ("DEX", 50),
        ("APP", 50),
        ("INT", 60),
        ("POW", 65),
        ("EDU", 70),
    ]);
    let d = calculate_derived(&chars, get_age_bracket(30), 0);
    assert_eq!(d.hp, 13);
    assert_eq!(d.major_wound, 7);
    assert_eq!(d.san, 65);
    assert_eq!(d.max_san, 99);
    assert_eq!(d.mp, 13);
    assert_eq!(d.dodge, 25);
}

#[test]
fn final_chars_apply_age_penalties_and_physical_deductions() {
    let mut app = test_app();
    app.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 50),
            ("CON", 50),
            ("SIZ", 60),
            ("DEX", 50),
            ("APP", 50),
            ("INT", 70),
            ("POW", 60),
            ("EDU", 70),
        ],
    );
    app.concept.age = 65;
    app.sync_age_bracket();
    app.age_deductions.set_char(Characteristic::Str, 10);
    app.age_deductions.set_char(Characteristic::Con, 5);
    app.age_deductions.set_char(Characteristic::Dex, 5);

    let final_chars = app.final_chars();

    assert_eq!(final_chars["STR"], 40);
    assert_eq!(final_chars["CON"], 45);
    assert_eq!(final_chars["DEX"], 45);
    assert_eq!(final_chars["APP"], 35);
    assert_eq!(final_chars["EDU"], 70);
}

#[test]
fn physical_deduction_overassignment_past_minimum_is_not_effective() {
    let mut app = test_app();
    app.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 15),
            ("CON", 15),
            ("SIZ", 60),
            ("DEX", 15),
            ("APP", 40),
            ("INT", 70),
            ("POW", 60),
            ("EDU", 80),
        ],
    );
    app.concept.age = 85;
    app.sync_age_bracket();
    app.age_deductions.set_char(Characteristic::Str, 80);

    let final_chars = app.final_chars();

    assert_eq!(final_chars["STR"], 5);
    assert_eq!(app.assigned_physical_deduction_total(), 80);
    assert_eq!(app.physical_deduction_total(), 10);
    assert_ne!(
        app.physical_deduction_total(),
        app.age_bracket().physical_deduct
    );
}

#[test]
fn final_chars_apply_young_age_edu_penalty() {
    let mut app = test_app();
    app.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 50),
            ("CON", 50),
            ("SIZ", 60),
            ("DEX", 50),
            ("APP", 50),
            ("INT", 70),
            ("POW", 60),
            ("EDU", 40),
        ],
    );
    app.concept.age = 17;
    app.sync_age_bracket();

    let final_chars = app.final_chars();

    assert_eq!(final_chars["EDU"], 35);
}

#[test]
fn final_chars_preserve_unset_characteristics() {
    let mut app = test_app();
    app.concept.age = 89;
    app.sync_age_bracket();
    app.age_deductions.set_char(Characteristic::Str, 90);
    app.edu_bonus = 10;

    let final_chars = app.final_chars();

    for def in CHARACTERISTICS {
        assert_eq!(
            final_chars.get_char(def.key),
            0,
            "{} should remain unset",
            def.key.key()
        );
    }
}

#[test]
fn edu_age_checks_do_nothing_when_age_bracket_has_no_checks() {
    let mut app = test_app();
    app.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 50),
            ("CON", 50),
            ("SIZ", 60),
            ("DEX", 50),
            ("APP", 50),
            ("INT", 70),
            ("POW", 60),
            ("EDU", 40),
        ],
    );
    app.concept.age = 18;
    app.sync_age_bracket();

    app.roll_edu_age_checks();

    assert_eq!(app.edu_bonus, 0);
    assert!(app.edu_check_rolls.is_empty());
}

#[test]
fn edu_age_check_improves_when_d100_exceeds_current_edu() {
    let mut app = test_app();
    app.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 50),
            ("CON", 50),
            ("SIZ", 60),
            ("DEX", 50),
            ("APP", 50),
            ("INT", 70),
            ("POW", 60),
            ("EDU", 40),
        ],
    );
    app.concept.age = 30;
    app.sync_age_bracket();

    app.apply_edu_age_check_rolls(&[(90, 7)]);

    assert_eq!(app.edu_check_rolls.len(), 1);
    let roll = &app.edu_check_rolls[0];
    assert_eq!(roll.d100, 90);
    assert!(roll.improved);
    assert_eq!(roll.gain, 7);
    assert_eq!(app.edu_bonus, 7);
    assert_eq!(roll.resulting_edu, 47);
}

#[test]
fn summary_is_blocked_until_required_edu_age_checks_are_done() {
    let mut app = test_app();
    app.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 50),
            ("CON", 50),
            ("SIZ", 60),
            ("DEX", 50),
            ("APP", 40),
            ("INT", 70),
            ("POW", 60),
            ("EDU", 80),
        ],
    );
    app.concept.age = 30;
    app.sync_age_bracket();
    app.set_occupation("Nurse".to_owned());
    resolve_nurse_choice(&mut app);
    assert_eq!(app.max_reachable_step(), 5);

    app.roll_edu_age_checks();

    assert_eq!(app.max_reachable_step(), 6);
}

#[test]
fn occupation_data_has_unique_names_and_validates() {
    let occupations = build_occupations();
    let mut names = HashSet::new();
    for occupation in &occupations {
        assert!(
            names.insert(occupation.name.as_str()),
            "duplicate occupation name `{}`",
            occupation.name
        );
    }
    validate_occupations(&occupations);
}

#[test]
fn occupation_resolution_respects_filled_choice_slots() {
    let mut app = test_app();
    app.set_occupation("Soldier".to_owned());
    app.occupation_choices
        .insert(ChoiceKey::new("soldier-climb-swim", 0), "Climb".to_owned());
    app.occupation_choices.insert(
        ChoiceKey::new("soldier-firearms", 0),
        "Firearms (Handgun)".to_owned(),
    );
    app.occupation_choices
        .insert(ChoiceKey::new("soldier-two", 0), "First Aid".to_owned());
    app.occupation_choices.insert(
        ChoiceKey::new("soldier-two", 1),
        "Mechanical Repair".to_owned(),
    );

    let occupation = app.selected_occupation().expect("soldier should exist");
    let resolved = app.resolved_occupation_skills_for(&occupation);

    assert_eq!(app.unresolved_choice_count_for(&occupation), 0);
    assert_eq!(app.unique_occupation_shortfall_for(&occupation), 0);
    assert!(resolved.contains(&"Dodge".to_owned()));
    assert!(resolved.contains(&"Fighting (Brawl)".to_owned()));
    assert!(resolved.contains(&"Firearms (Handgun)".to_owned()));
    assert!(resolved.contains(&"Mechanical Repair".to_owned()));
    assert_eq!(resolved.len(), app.occupation_slot_count_for(&occupation));
}

#[test]
fn prune_occupation_allocations_removes_skills_no_longer_allowed() {
    let mut app = test_app();
    app.set_occupation("Soldier".to_owned());
    app.occupation_choices
        .insert(ChoiceKey::new("soldier-climb-swim", 0), "Climb".to_owned());
    app.occupation_choices.insert(
        ChoiceKey::new("soldier-firearms", 0),
        "Firearms (Handgun)".to_owned(),
    );
    app.occupation_choices
        .insert(ChoiceKey::new("soldier-two", 0), "First Aid".to_owned());
    app.occupation_choices.insert(
        ChoiceKey::new("soldier-two", 1),
        "Mechanical Repair".to_owned(),
    );

    app.allocations
        .occupation_points
        .insert("Climb".to_owned(), 20);
    app.allocations
        .occupation_points
        .insert("Accounting".to_owned(), 20);
    app.allocations
        .occupation_points
        .insert("Credit Rating".to_owned(), 10);

    app.occupation_choices
        .insert(ChoiceKey::new("soldier-climb-swim", 0), "Swim".to_owned());
    app.prune_occupation_allocations();

    assert!(!app.allocations.occupation_points.contains_key("Climb"));
    assert!(!app.allocations.occupation_points.contains_key("Accounting"));
    assert!(
        app.allocations
            .occupation_points
            .contains_key("Credit Rating")
    );
}

#[test]
fn prune_occupation_allocations_removes_credit_rating_without_occupation() {
    let mut app = test_app();
    app.allocations
        .occupation_points
        .insert("Credit Rating".to_owned(), 10);
    app.allocations
        .occupation_points
        .insert("Library Use".to_owned(), 20);

    assert!(app.occupation_skill_set().is_empty());

    app.prune_occupation_allocations();

    assert!(app.allocations.occupation_points.is_empty());
}

#[test]
fn prune_personal_allocations_removes_credit_rating_and_mythos() {
    let mut app = test_app();
    app.allocations
        .personal_points
        .insert("Credit Rating".to_owned(), 10);
    app.allocations
        .personal_points
        .insert("Cthulhu Mythos".to_owned(), 10);
    app.allocations
        .personal_points
        .insert("Library Use".to_owned(), 10);

    app.prune_personal_allocations();

    assert!(
        !app.allocations
            .personal_points
            .contains_key("Credit Rating")
    );
    assert!(
        !app.allocations
            .personal_points
            .contains_key("Cthulhu Mythos")
    );
    assert_eq!(
        app.allocations.personal_points.get("Library Use"),
        Some(&10)
    );
}

#[test]
fn personal_allocation_math_ignores_reserved_skills_before_pruning() {
    let mut app = test_app();
    app.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 50),
            ("CON", 50),
            ("SIZ", 60),
            ("DEX", 50),
            ("APP", 40),
            ("INT", 70),
            ("POW", 60),
            ("EDU", 80),
        ],
    );
    app.allocations
        .personal_points
        .insert("Credit Rating".to_owned(), 10);
    app.allocations
        .personal_points
        .insert("Cthulhu Mythos".to_owned(), 10);
    app.allocations
        .personal_points
        .insert("Library Use".to_owned(), 10);

    let math = app.sheet_math();
    let credit_rating = math
        .skill_rows
        .iter()
        .find(|row| row.name == "Credit Rating")
        .expect("Credit Rating row should exist");
    let mythos = math
        .skill_rows
        .iter()
        .find(|row| row.name == "Cthulhu Mythos")
        .expect("Cthulhu Mythos row should exist");

    assert_eq!(app.used_personal_points(), 10);
    assert_eq!(credit_rating.personal_add, 0);
    assert_eq!(mythos.personal_add, 0);
}

#[test]
fn occupation_allocation_math_ignores_disallowed_skills_before_pruning() {
    let mut app = test_app();
    app.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 50),
            ("CON", 50),
            ("SIZ", 60),
            ("DEX", 50),
            ("APP", 40),
            ("INT", 70),
            ("POW", 60),
            ("EDU", 80),
        ],
    );
    app.set_occupation("Nurse".to_owned());
    resolve_nurse_choice(&mut app);
    app.allocations
        .occupation_points
        .insert("First Aid".to_owned(), 10);
    app.allocations
        .occupation_points
        .insert("Credit Rating".to_owned(), 5);
    app.allocations
        .occupation_points
        .insert("Library Use".to_owned(), 50);

    let math = app.sheet_math();
    let first_aid = math
        .skill_rows
        .iter()
        .find(|row| row.name == "First Aid")
        .expect("First Aid row should exist");
    let library_use = math
        .skill_rows
        .iter()
        .find(|row| row.name == "Library Use")
        .expect("Library Use row should exist");

    assert_eq!(app.used_occupation_points(), 15);
    assert_eq!(first_aid.occ_add, 10);
    assert_eq!(library_use.occ_add, 0);
}

#[test]
fn credit_rating_ignores_stale_occupation_points_without_occupation() {
    let mut app = test_app();
    app.allocations
        .occupation_points
        .insert("Credit Rating".to_owned(), 50);

    assert!(app.occupation_skill_set().is_empty());
    assert_eq!(app.used_occupation_points(), 0);
    assert_eq!(app.credit_rating(), 0);
}

#[test]
fn occupation_budget_uses_selected_occupation_formula_when_state_drifts() {
    let mut app = test_app();
    app.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 50),
            ("CON", 50),
            ("SIZ", 60),
            ("DEX", 50),
            ("APP", 40),
            ("INT", 70),
            ("POW", 60),
            ("EDU", 80),
        ],
    );
    app.set_occupation("Soldier".to_owned());
    app.formula_key = FormulaKey::Edu4;

    let final_chars = app.final_chars();

    let selected = app.selected_occupation();
    let active_formula_key = app.active_formula_key_for(selected.as_ref());

    assert_eq!(active_formula_key, FormulaKey::Edu2Dex2);
    assert_eq!(active_formula_key.calculate(&final_chars), 260);
    assert_eq!(app.sheet_math().occupation_budget, 260);
}

#[test]
fn allocation_math_sanitizes_stale_allowed_values() {
    let mut app = test_app();
    app.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 50),
            ("CON", 50),
            ("SIZ", 60),
            ("DEX", 50),
            ("APP", 40),
            ("INT", 70),
            ("POW", 60),
            ("EDU", 80),
        ],
    );
    app.set_occupation("Nurse".to_owned());
    resolve_nurse_choice(&mut app);
    app.allocations
        .occupation_points
        .insert("First Aid".to_owned(), 500);
    app.allocations
        .personal_points
        .insert("Library Use".to_owned(), 500);
    app.allocations
        .personal_points
        .insert("Spot Hidden".to_owned(), -20);

    let math = app.sheet_math();
    let first_aid = math
        .skill_rows
        .iter()
        .find(|row| row.name == "First Aid")
        .expect("First Aid row should exist");
    let library_use = math
        .skill_rows
        .iter()
        .find(|row| row.name == "Library Use")
        .expect("Library Use row should exist");
    let spot_hidden = math
        .skill_rows
        .iter()
        .find(|row| row.name == "Spot Hidden")
        .expect("Spot Hidden row should exist");

    assert_eq!(first_aid.total, MAX_CREATION_VALUE);
    assert_eq!(first_aid.occ_add, MAX_CREATION_VALUE - first_aid.base);
    assert_eq!(library_use.total, MAX_CREATION_VALUE);
    assert_eq!(
        library_use.personal_add,
        MAX_CREATION_VALUE - library_use.base
    );
    assert_eq!(spot_hidden.personal_add, 0);
    assert_eq!(app.used_occupation_points(), first_aid.occ_add);
    assert_eq!(app.used_personal_points(), library_use.personal_add);
}

#[test]
fn normalize_formula_key_replaces_stale_formula_for_selected_occupation() {
    let mut app = test_app();
    app.set_occupation("Soldier".to_owned());
    app.formula_key = FormulaKey::Edu4;

    let selected = app.selected_occupation();
    app.normalize_formula_key_for(selected.as_ref());

    assert_eq!(app.formula_key, FormulaKey::Edu2Dex2);
}

#[test]
fn prune_occupation_choices_removes_stale_and_invalid_choice_state() {
    let mut app = test_app();
    app.set_occupation("Nurse".to_owned());
    app.occupation_choices.insert(
        ChoiceKey::new("nurse-interpersonal", 0),
        "Persuade".to_owned(),
    );
    app.occupation_choices.insert(
        ChoiceKey::new("soldier-firearms", 0),
        "Firearms (Handgun)".to_owned(),
    );

    let occupation = app
        .selected_occupation()
        .expect("Nurse occupation should exist");
    app.prune_occupation_choices_for(&occupation);

    assert_eq!(app.occupation_choices.len(), 1);
    assert_eq!(
        app.occupation_choices
            .get(&ChoiceKey::new("nurse-interpersonal", 0)),
        Some(&"Persuade".to_owned())
    );

    app.occupation_choices.insert(
        ChoiceKey::new("nurse-interpersonal", 0),
        "Bogus Skill".to_owned(),
    );
    app.prune_occupation_choices_for(&occupation);

    assert!(app.occupation_choices.is_empty());
}

#[test]
fn max_reachable_step_does_not_jump_to_skills_without_characteristics() {
    let mut app = test_app();
    app.set_occupation("Soldier".to_owned());
    assert_eq!(app.max_reachable_step(), 2);
}

#[test]
fn manual_edit_after_roll_marks_characteristics_mixed() {
    let mut app = test_app();
    app.roll_all_characteristics();
    assert_eq!(app.char_method, CharMethod::Roll);

    app.set_char_value("STR", 50);

    assert_eq!(app.char_method, CharMethod::Mixed);
    assert!(!app.char_rolls.contains_key("STR"));
}

#[test]
fn explicit_same_value_set_after_roll_clears_stale_roll_display() {
    let mut app = test_app();
    app.roll_all_characteristics();
    let str_value = app.char_value("STR");

    app.set_char_value("STR", str_value);

    assert_eq!(app.char_method, CharMethod::Mixed);
    assert!(!app.char_rolls.contains_key("STR"));
}

#[test]
fn invalidated_step_is_clamped_to_current_reachability() {
    let mut app = test_app();
    app.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 50),
            ("CON", 50),
            ("SIZ", 60),
            ("DEX", 50),
            ("APP", 40),
            ("INT", 70),
            ("POW", 60),
            ("EDU", 80),
        ],
    );
    app.set_occupation("Nurse".to_owned());
    resolve_nurse_choice(&mut app);
    assert_eq!(app.max_reachable_step(), 5);
    app.roll_edu_age_checks();
    assert_eq!(app.max_reachable_step(), 6);

    app.step = 6;
    app.chars = CharacteristicValues::default();
    app.frame_max_reachable_step = app.max_reachable_step();
    if app.step > app.frame_max_reachable_step {
        app.step = app.frame_max_reachable_step;
    }

    assert_eq!(app.frame_max_reachable_step, 2);
    assert_eq!(app.step, 2);
}

#[test]
fn custom_occupation_requires_all_eight_unique_skills() {
    let mut app = test_app();
    app.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 50),
            ("CON", 50),
            ("SIZ", 60),
            ("DEX", 50),
            ("APP", 40),
            ("INT", 70),
            ("POW", 60),
            ("EDU", 80),
        ],
    );
    app.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());
    app.custom_occupation.skills[0] = "Library Use".to_owned();

    let occupation = app
        .selected_occupation()
        .expect("custom occupation should exist");

    assert_eq!(app.resolved_occupation_skills_for(&occupation).len(), 1);
    assert_eq!(app.unique_occupation_shortfall_for(&occupation), 7);
    assert_eq!(app.max_reachable_step(), 4);
}

#[test]
fn unknown_occupation_id_is_not_displayed_as_selected_occupation() {
    let mut app = test_app();

    app.set_occupation("Bogus Occupation".to_owned());

    assert!(app.occupation_id.is_empty());
    assert!(app.selected_occupation().is_none());
    assert_eq!(app.selected_occupation_name(), "No occupation");

    app.occupation_id = "Bogus Occupation".to_owned();

    assert!(app.selected_occupation().is_none());
    assert_eq!(app.selected_occupation_name(), "No occupation");
}

#[test]
fn quick_skill_package_sets_credit_rating_to_occupation_minimum() {
    let mut app = test_app();
    app.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 50),
            ("CON", 50),
            ("SIZ", 60),
            ("DEX", 50),
            ("APP", 40),
            ("INT", 70),
            ("POW", 60),
            ("EDU", 80),
        ],
    );
    app.set_occupation("Soldier".to_owned());
    app.occupation_choices
        .insert(ChoiceKey::new("soldier-climb-swim", 0), "Climb".to_owned());
    app.occupation_choices.insert(
        ChoiceKey::new("soldier-firearms", 0),
        "Firearms (Handgun)".to_owned(),
    );
    app.occupation_choices
        .insert(ChoiceKey::new("soldier-two", 0), "First Aid".to_owned());
    app.occupation_choices.insert(
        ChoiceKey::new("soldier-two", 1),
        "Mechanical Repair".to_owned(),
    );

    app.apply_quick_skill_package();

    assert_eq!(
        app.allocations.occupation_points.get("Credit Rating"),
        Some(&9)
    );
    assert!(app.credit_rating() <= 30);
}

#[test]
fn custom_occupation_discards_unknown_and_reserved_skills() {
    let mut app = test_app();
    app.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 50),
            ("CON", 50),
            ("SIZ", 60),
            ("DEX", 50),
            ("APP", 40),
            ("INT", 70),
            ("POW", 60),
            ("EDU", 80),
        ],
    );
    app.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());
    app.custom_occupation.skills = vec![
        "Library Use".to_owned(),
        "Spot Hidden".to_owned(),
        "Bogus Skill".to_owned(),
        "Credit Rating".to_owned(),
        "Cthulhu Mythos".to_owned(),
        "Listen".to_owned(),
        "Stealth".to_owned(),
        "Persuade".to_owned(),
    ];

    let occupation = app
        .selected_occupation()
        .expect("custom occupation should exist");
    let resolved = app.resolved_occupation_skills_for(&occupation);

    assert_eq!(
        resolved,
        vec![
            "Library Use".to_owned(),
            "Spot Hidden".to_owned(),
            "Listen".to_owned(),
            "Stealth".to_owned(),
            "Persuade".to_owned(),
        ]
    );
    assert_eq!(app.unique_occupation_shortfall_for(&occupation), 3);
    assert_eq!(app.max_reachable_step(), 4);
}

#[test]
fn custom_occupation_skill_slots_normalize_to_required_count() {
    let mut app = test_app();
    app.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());

    app.custom_occupation.skills = vec!["Library Use".to_owned()];
    app.normalize_custom_occupation_skills();
    assert_eq!(
        app.custom_occupation.skills.len(),
        CUSTOM_OCCUPATION_SKILL_COUNT
    );
    assert_eq!(app.custom_occupation.skills[0], "Library Use");
    assert!(
        app.custom_occupation.skills[1..]
            .iter()
            .all(String::is_empty)
    );

    app.custom_occupation.skills = vec![
        "Library Use".to_owned(),
        "Spot Hidden".to_owned(),
        "Listen".to_owned(),
        "Stealth".to_owned(),
        "Persuade".to_owned(),
        "Charm".to_owned(),
        "Fast Talk".to_owned(),
        "Intimidate".to_owned(),
        "Law".to_owned(),
    ];
    app.normalize_custom_occupation_skills();

    assert_eq!(
        app.custom_occupation.skills.len(),
        CUSTOM_OCCUPATION_SKILL_COUNT
    );
    assert!(!app.custom_occupation.skills.contains(&"Law".to_owned()));
}

#[test]
fn custom_occupation_required_skill_count_does_not_follow_vector_length() {
    let mut app = test_app();
    app.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());
    app.custom_occupation.skills = vec!["Library Use".to_owned()];

    let occupation = app
        .selected_occupation()
        .expect("custom occupation should exist");

    assert_eq!(
        app.required_occupation_skill_count_for(&occupation),
        CUSTOM_OCCUPATION_SKILL_COUNT
    );
    assert_eq!(app.unique_occupation_shortfall_for(&occupation), 7);

    app.custom_occupation.skills = vec![
        "Library Use".to_owned(),
        "Spot Hidden".to_owned(),
        "Listen".to_owned(),
        "Stealth".to_owned(),
        "Persuade".to_owned(),
        "Charm".to_owned(),
        "Fast Talk".to_owned(),
        "Intimidate".to_owned(),
        "Law".to_owned(),
    ];
    let occupation = app
        .selected_occupation()
        .expect("custom occupation should exist");

    assert_eq!(
        app.required_occupation_skill_count_for(&occupation),
        CUSTOM_OCCUPATION_SKILL_COUNT
    );
    assert_eq!(app.unique_occupation_shortfall_for(&occupation), 0);
    assert_eq!(app.resolved_occupation_skills_for(&occupation).len(), 8);
    assert!(
        !app.resolved_occupation_skills_for(&occupation)
            .contains(&"Law".to_owned())
    );
}

#[test]
fn sheet_math_uses_shared_occupation_skill_set() {
    let mut app = test_app();
    app.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 50),
            ("CON", 50),
            ("SIZ", 60),
            ("DEX", 50),
            ("APP", 40),
            ("INT", 70),
            ("POW", 60),
            ("EDU", 80),
        ],
    );
    app.set_occupation("Nurse".to_owned());
    resolve_nurse_choice(&mut app);

    assert_eq!(
        app.sheet_math().occupation_skill_set,
        app.occupation_skill_set()
    );
}

#[test]
fn custom_occupation_name_and_skills_are_trimmed() {
    let mut app = test_app();
    app.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());
    app.custom_occupation.name = "  Field Researcher  ".to_owned();
    app.custom_occupation.skills[0] = "  Library Use  ".to_owned();

    let occupation = app
        .selected_occupation()
        .expect("custom occupation should exist");

    assert_eq!(occupation.name, "Field Researcher");
    assert_eq!(
        app.resolved_occupation_skills_for(&occupation),
        vec!["Library Use".to_owned()]
    );
}

#[test]
fn unique_strings_trims_and_deduplicates() {
    let values = vec![
        " Library Use ".to_owned(),
        "Library Use".to_owned(),
        String::new(),
        "  ".to_owned(),
        "Spot Hidden".to_owned(),
    ];

    assert_eq!(
        unique_strings(values),
        vec!["Library Use".to_owned(), "Spot Hidden".to_owned()]
    );
}

#[test]
fn skill_name_constants_match_skill_specs() {
    let spec_names: HashSet<&str> = SKILL_SPECS.iter().map(|skill| skill.name).collect();
    let spec_ids: HashSet<Skill> = SKILL_SPECS.iter().map(|skill| skill.id).collect();
    let all_names: HashSet<&str> = ALL_SKILL_NAMES.iter().copied().collect();

    assert_eq!(
        ALL_SKILL_NAMES.len(),
        all_names.len(),
        "ALL_SKILL_NAMES contains duplicates"
    );
    assert_eq!(
        SKILL_SPECS.len(),
        spec_names.len(),
        "SKILL_SPECS contains duplicate skill names"
    );
    assert_eq!(
        SKILL_SPECS.len(),
        spec_ids.len(),
        "SKILL_SPECS contains duplicate skill ids"
    );
    assert_eq!(all_names, spec_names);

    let selectable_expected: HashSet<&str> = spec_names
        .iter()
        .copied()
        .filter(|skill| *skill != "Credit Rating" && *skill != "Cthulhu Mythos")
        .collect();
    let selectable_actual: HashSet<&str> = OCCUPATION_SELECTABLE_SKILLS.iter().copied().collect();

    assert_eq!(
        OCCUPATION_SELECTABLE_SKILLS.len(),
        selectable_actual.len(),
        "OCCUPATION_SELECTABLE_SKILLS contains duplicates"
    );
    assert_eq!(selectable_actual, selectable_expected);

    for skill in ART_SKILLS {
        assert!(spec_names.contains(skill), "unknown art skill: {skill}");
        assert!(skill.starts_with("Art/Craft"));
    }

    for skill in SCIENCE_SKILLS {
        assert!(spec_names.contains(skill), "unknown science skill: {skill}");
        assert!(skill.starts_with("Science"));
    }

    for skill in INTERPERSONAL_SKILLS {
        assert!(
            spec_names.contains(skill),
            "unknown interpersonal skill: {skill}"
        );
    }

    for skill in FIREARMS_SKILLS {
        assert!(
            spec_names.contains(skill),
            "unknown firearms skill: {skill}"
        );
        assert!(skill.starts_with("Firearms"));
    }
}

#[test]
fn invalid_choice_value_does_not_resolve_or_unlock_occupation() {
    let mut app = test_app();
    app.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 50),
            ("CON", 50),
            ("SIZ", 60),
            ("DEX", 50),
            ("APP", 40),
            ("INT", 70),
            ("POW", 60),
            ("EDU", 80),
        ],
    );
    app.set_occupation("Soldier".to_owned());
    app.occupation_choices.insert(
        ChoiceKey::new("soldier-climb-swim", 0),
        "Bogus Skill".to_owned(),
    );
    app.occupation_choices.insert(
        ChoiceKey::new("soldier-firearms", 0),
        "Firearms (Handgun)".to_owned(),
    );
    app.occupation_choices
        .insert(ChoiceKey::new("soldier-two", 0), "First Aid".to_owned());
    app.occupation_choices.insert(
        ChoiceKey::new("soldier-two", 1),
        "Mechanical Repair".to_owned(),
    );

    let occupation = app.selected_occupation().expect("soldier should exist");

    assert_eq!(app.unresolved_choice_count_for(&occupation), 1);
    assert!(
        !app.resolved_occupation_skills_for(&occupation)
            .contains(&"Bogus Skill".to_owned())
    );
    assert_eq!(app.max_reachable_step(), 4);
}

#[test]
fn refresh_reachability_clamps_invalidated_current_step() {
    let mut app = test_app();
    app.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 50),
            ("CON", 50),
            ("SIZ", 60),
            ("DEX", 50),
            ("APP", 40),
            ("INT", 70),
            ("POW", 60),
            ("EDU", 80),
        ],
    );
    app.set_occupation("Nurse".to_owned());
    resolve_nurse_choice(&mut app);
    assert_eq!(app.max_reachable_step(), 5);
    app.roll_edu_age_checks();
    assert_eq!(app.max_reachable_step(), 6);
    app.step = 6;
    app.chars = CharacteristicValues::default();

    app.refresh_reachability();

    assert_eq!(app.frame_max_reachable_step, 2);
    assert_eq!(app.step, 2);
}

#[test]
fn choice_pool_matching_accepts_valid_unique_assignment() {
    let pools = vec![vec!["Climb", "Swim"], vec!["Climb"], vec!["First Aid"]];
    assert!(choice_pools_have_full_matching(&pools));
}

#[test]
#[should_panic(expected = "selectable non-fixed option")]
fn occupation_validation_rejects_choice_slots_hidden_by_fixed_skills() {
    let occupations = vec![occupation(
        "Bad Hidden Choice Occupation",
        (0, 10),
        vec![FormulaKey::Edu4],
        vec![
            fixed("Accounting"),
            fixed("Anthropology"),
            fixed("Appraise"),
            fixed("Archaeology"),
            fixed("Art/Craft"),
            fixed("Charm"),
            fixed("Climb"),
            choice(
                "bad-hidden",
                "Impossible choice",
                vec!["Accounting".to_owned()],
                1,
            ),
        ],
    )];

    validate_occupations(&occupations);
}

#[test]
#[should_panic(expected = "cannot be resolved to unique non-fixed skills")]
fn occupation_validation_rejects_cross_choice_impossible_unique_picks() {
    let occupations = vec![occupation(
        "Bad Overlapping Choice Occupation",
        (0, 10),
        vec![FormulaKey::Edu4],
        vec![
            fixed("Accounting"),
            fixed("Anthropology"),
            fixed("Appraise"),
            fixed("Archaeology"),
            fixed("Art/Craft"),
            fixed("Charm"),
            choice("bad-a", "Bad A", vec!["Climb".to_owned()], 1),
            choice("bad-b", "Bad B", vec!["Climb".to_owned()], 1),
        ],
    )];

    validate_occupations(&occupations);
}

#[test]
#[should_panic(expected = "duplicate occupation name")]
fn occupation_validation_rejects_duplicate_names() {
    let occupations = vec![
        occupation(
            "Duplicate Occupation",
            (0, 10),
            vec![FormulaKey::Edu4],
            vec![
                fixed("Accounting"),
                fixed("Anthropology"),
                fixed("Appraise"),
                fixed("Archaeology"),
                fixed("Art/Craft"),
                fixed("Charm"),
                fixed("Climb"),
                fixed("Disguise"),
            ],
        ),
        occupation(
            "Duplicate Occupation",
            (0, 10),
            vec![FormulaKey::Edu4],
            vec![
                fixed("Accounting"),
                fixed("Anthropology"),
                fixed("Appraise"),
                fixed("Archaeology"),
                fixed("Art/Craft"),
                fixed("Charm"),
                fixed("Climb"),
                fixed("Disguise"),
            ],
        ),
    ];

    validate_occupations(&occupations);
}

#[test]
#[should_panic(expected = "duplicate formula")]
fn occupation_validation_rejects_duplicate_formulas() {
    let occupations = vec![occupation(
        "Bad Formula Occupation",
        (0, 10),
        vec![FormulaKey::Edu4, FormulaKey::Edu4],
        vec![
            fixed("Accounting"),
            fixed("Anthropology"),
            fixed("Appraise"),
            fixed("Archaeology"),
            fixed("Art/Craft"),
            fixed("Charm"),
            fixed("Climb"),
            fixed("Disguise"),
        ],
    )];

    validate_occupations(&occupations);
}
