use super::data::*;
use super::models::*;
use super::occupations::*;
use super::ruleset::*;
use std::collections::HashSet;

fn test_app() -> CoC7eApp {
    validate_skill_constants();
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
fn impossible_physical_deduction_reports_capacity_and_unlocks_summary_explanation() {
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
    app.set_age(70);
    app.set_occupation("Nurse".to_owned());
    resolve_nurse_choice(&mut app);
    app.apply_edu_age_check_rolls(&[(100, 1), (100, 1), (100, 1), (100, 1)]);

    assert_eq!(app.age_bracket().physical_deduct, 40);
    assert_eq!(app.max_possible_physical_deduction(), 30);
    assert!(!app.physical_deduction_is_possible());
    assert_eq!(app.max_reachable_step(), 6);

    let blockers = app.summary_blockers_for(&app.sheet_math());
    assert!(blockers.iter().any(|blocker| {
        blocker == "age deductions impossible: requires 40, current STR/CON/DEX can absorb only 30"
    }));
}

#[test]
fn young_age_physical_deduction_message_names_str_and_siz() {
    let mut app = test_app();
    app.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 15),
            ("CON", 50),
            ("SIZ", 40),
            ("DEX", 50),
            ("APP", 40),
            ("INT", 70),
            ("POW", 60),
            ("EDU", 80),
        ],
    );
    app.set_age(17);

    assert_eq!(app.physical_deduction_source_label(), "STR/SIZ");
}

#[test]
fn set_age_deduction_clamps_against_live_total() {
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
    app.concept.age = 65;
    app.sync_age_bracket();

    app.set_age_deduction(Characteristic::Str, 10);
    app.set_age_deduction(Characteristic::Con, 10);
    app.set_age_deduction(Characteristic::Dex, 10);

    assert_eq!(app.age_deductions.get_char(Characteristic::Str), 10);
    assert_eq!(app.age_deductions.get_char(Characteristic::Con), 10);
    assert_eq!(app.age_deductions.get_char(Characteristic::Dex), 0);
    assert_eq!(app.physical_deduction_total(), 20);
}

#[test]
fn set_age_clamps_and_resets_age_bracket_state() {
    let mut app = test_app();
    app.luck_state.value = Some(50);
    app.luck_state.rolls.push(DiceResult {
        rolls: vec![3, 3, 4],
        plus_six: false,
        value: 50,
        kept: None,
    });

    app.set_age(999);

    assert_eq!(app.concept.age, 89);
    assert_eq!(app.age_bracket().label, "80–89");
    assert_eq!(app.luck_state.value, None);
    assert!(app.luck_state.rolls.is_empty());
}

#[test]
fn custom_occupation_name_setter_preserves_in_progress_text() {
    let mut app = test_app();

    app.set_custom_occupation_name("Occult ".to_owned());

    assert_eq!(app.custom_occupation.name, "Occult ");
}

#[test]
fn selected_custom_occupation_name_is_trimmed_for_display_and_rules() {
    let mut app = test_app();
    app.set_custom_occupation_name("  Occult Tinkerer  ".to_owned());
    app.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());

    assert_eq!(app.custom_occupation.name, "  Occult Tinkerer  ");
    assert_eq!(app.selected_occupation_name(), "Occult Tinkerer");
    assert_eq!(
        app.selected_occupation()
            .expect("custom occupation should build")
            .name,
        "Occult Tinkerer"
    );
}

#[test]
fn custom_occupation_credit_setters_clamp_boundary_values() {
    let mut app = test_app();
    app.set_custom_occupation_credit_min(-10);
    app.set_custom_occupation_credit_max(150);

    assert_eq!(app.custom_occupation.credit_min, 0);
    assert_eq!(app.custom_occupation.credit_max, 99);
}

#[test]
fn sanitize_state_clamps_imported_age_deductions() {
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
    app.concept.age = 65;
    app.sync_age_bracket();
    app.age_deductions.set_char(Characteristic::Str, 25);
    app.age_deductions.set_char(Characteristic::Con, 20);
    app.age_deductions.set_char(Characteristic::Dex, 20);
    app.age_deductions.set_char(Characteristic::Siz, 20);

    app.sanitize_state();

    assert_eq!(app.age_deductions.get_char(Characteristic::Siz), 0);
    assert_eq!(app.age_deductions.get_char(Characteristic::Str), 20);
    assert_eq!(app.age_deductions.get_char(Characteristic::Con), 0);
    assert_eq!(app.age_deductions.get_char(Characteristic::Dex), 0);
    assert_eq!(
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
    assert!(resolved.contains(&Skill::Dodge));
    assert!(resolved.contains(&Skill::FightingBrawl));
    assert!(resolved.contains(&Skill::FirearmsHandgun));
    assert!(resolved.contains(&Skill::MechanicalRepair));
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

    app.allocations.occupation_points.insert(Skill::Climb, 20);
    app.allocations
        .occupation_points
        .insert(Skill::Accounting, 20);
    app.allocations
        .occupation_points
        .insert(Skill::CreditRating, 10);

    app.occupation_choices
        .insert(ChoiceKey::new("soldier-climb-swim", 0), "Swim".to_owned());
    app.prune_occupation_allocations();

    assert!(
        !app.allocations
            .occupation_points
            .contains_key(&Skill::Climb)
    );
    assert!(
        !app.allocations
            .occupation_points
            .contains_key(&Skill::Accounting)
    );
    assert!(
        app.allocations
            .occupation_points
            .contains_key(&Skill::CreditRating)
    );
}

#[test]
fn prune_occupation_allocations_removes_credit_rating_without_occupation() {
    let mut app = test_app();
    app.allocations
        .occupation_points
        .insert(Skill::CreditRating, 10);
    app.allocations
        .occupation_points
        .insert(Skill::LibraryUse, 20);

    assert!(app.sheet_math().occupation_skill_set.is_empty());

    app.prune_occupation_allocations();

    assert!(app.allocations.occupation_points.is_empty());
}

#[test]
fn prune_personal_allocations_removes_credit_rating_and_mythos() {
    let mut app = test_app();
    app.allocations
        .personal_points
        .insert(Skill::CreditRating, 10);
    app.allocations
        .personal_points
        .insert(Skill::CthulhuMythos, 10);
    app.allocations
        .personal_points
        .insert(Skill::LibraryUse, 10);

    app.prune_personal_allocations();

    assert!(
        !app.allocations
            .personal_points
            .contains_key(&Skill::CreditRating)
    );
    assert!(
        !app.allocations
            .personal_points
            .contains_key(&Skill::CthulhuMythos)
    );
    assert_eq!(
        app.allocations.personal_points.get(&Skill::LibraryUse),
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
        .insert(Skill::CreditRating, 10);
    app.allocations
        .personal_points
        .insert(Skill::CthulhuMythos, 10);
    app.allocations
        .personal_points
        .insert(Skill::LibraryUse, 10);

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
        .insert(Skill::FirstAid, 10);
    app.allocations
        .occupation_points
        .insert(Skill::CreditRating, 5);
    app.allocations
        .occupation_points
        .insert(Skill::LibraryUse, 50);

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
        .insert(Skill::CreditRating, 50);

    assert!(app.sheet_math().occupation_skill_set.is_empty());
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
        .insert(Skill::FirstAid, 500);
    app.allocations
        .personal_points
        .insert(Skill::LibraryUse, 500);
    app.allocations
        .personal_points
        .insert(Skill::SpotHidden, -20);

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
fn prune_allocation_sanitizers_rewrite_stale_allowed_values() {
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
        .insert(Skill::FirstAid, 500);
    app.allocations
        .occupation_points
        .insert(Skill::LibraryUse, 500);
    app.allocations
        .personal_points
        .insert(Skill::LibraryUse, 500);
    app.allocations
        .personal_points
        .insert(Skill::SpotHidden, -20);

    app.prune_occupation_allocations();

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

    assert_eq!(
        app.allocations.occupation_points.get(&Skill::FirstAid),
        Some(&first_aid.occ_add)
    );
    assert!(
        !app.allocations
            .occupation_points
            .contains_key(&Skill::LibraryUse)
    );
    assert_eq!(
        app.allocations.personal_points.get(&Skill::LibraryUse),
        Some(&library_use.personal_add)
    );
    assert!(
        !app.allocations
            .personal_points
            .contains_key(&Skill::SpotHidden)
    );
}

#[test]
fn allocation_setters_derive_caps_instead_of_trusting_callers() {
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

    app.set_personal_allocation("First Aid", 50);
    app.set_occupation_allocation("First Aid", 500);

    let math = app.sheet_math();
    let first_aid = math
        .skill_rows
        .iter()
        .find(|row| row.name == "First Aid")
        .expect("First Aid row should exist");
    assert_eq!(
        first_aid.base + first_aid.occ_add + first_aid.personal_add,
        99
    );
    assert_eq!(first_aid.occ_add, 19);
}

#[test]
fn manual_occupation_allocations_cannot_exceed_total_budget() {
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

    let mut skills: Vec<Skill> = app
        .sheet_math()
        .occupation_skill_set
        .iter()
        .copied()
        .collect();
    skills.sort_by_key(|skill| skill.name());

    for skill in skills {
        app.set_occupation_allocation(skill.name(), 500);
        let math = app.sheet_math();
        assert!(
            CoC7eApp::used_occupation_points_from(&math.skill_rows) <= math.occupation_budget,
            "manual occupation allocation overspent after assigning {}",
            skill.name()
        );
    }
}

#[test]
fn manual_personal_allocations_cannot_exceed_total_budget() {
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

    for skill in ALL_SKILL_NAMES {
        app.set_personal_allocation(skill, 500);
        let math = app.sheet_math();
        assert!(
            CoC7eApp::used_personal_points_from(&math.skill_rows) <= math.personal_budget,
            "manual personal allocation overspent after assigning {skill}"
        );
    }
}

#[test]
fn sanitize_allocations_trims_imported_values_to_total_budgets() {
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

    for skill in app.sheet_math().occupation_skill_set {
        app.allocations.occupation_points.insert(skill, 99);
    }
    for skill in ALL_SKILL_NAMES {
        let skill = Skill::from_name(skill).expect("skill constant should be known");
        app.allocations.personal_points.insert(skill, 99);
    }

    app.sanitize_allocations();

    let math = app.sheet_math();
    assert!(CoC7eApp::used_occupation_points_from(&math.skill_rows) <= math.occupation_budget);
    assert!(CoC7eApp::used_personal_points_from(&math.skill_rows) <= math.personal_budget);
    assert!(
        math.skill_rows
            .iter()
            .all(|row| row.total <= MAX_CREATION_VALUE)
    );
}

#[test]
fn allocation_setters_remove_ineligible_skills() {
    let mut app = test_app();
    app.set_occupation_allocation("First Aid", 99);
    app.set_personal_allocation("Credit Rating", 99);

    assert!(
        !app.allocations
            .occupation_points
            .contains_key(&Skill::FirstAid)
    );
    assert!(
        !app.allocations
            .personal_points
            .contains_key(&Skill::CreditRating)
    );
}

#[test]
fn sanitize_state_cleans_imported_boundary_state() {
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
    app.custom_occupation.skills = vec!["Library Use".to_owned()];
    app.occupation_choices.insert(
        ChoiceKey::new("soldier-climb-swim", 0),
        "Bogus Skill".to_owned(),
    );
    app.occupation_choices.insert(
        ChoiceKey::new("nurse-interpersonal", 0),
        "Persuade".to_owned(),
    );
    app.allocations.occupation_points.insert(Skill::Dodge, 500);
    app.allocations
        .occupation_points
        .insert(Skill::LibraryUse, 500);
    app.allocations
        .personal_points
        .insert(Skill::LibraryUse, 500);

    app.sanitize_state();

    let math = app.sheet_math();
    let dodge_occ_add = math
        .skill_rows
        .iter()
        .find(|row| row.name == "Dodge")
        .expect("Dodge row should exist")
        .occ_add;

    assert_eq!(app.formula_key, FormulaKey::Edu2Dex2);
    assert_eq!(
        app.custom_occupation.skills.len(),
        CUSTOM_OCCUPATION_SKILL_COUNT
    );
    assert!(app.occupation_choices.is_empty());
    assert_eq!(
        app.allocations.occupation_points.get(&Skill::Dodge),
        Some(&dodge_occ_add)
    );
    assert!(
        !app.allocations
            .occupation_points
            .contains_key(&Skill::LibraryUse)
    );
    assert!(
        app.allocations
            .personal_points
            .contains_key(&Skill::LibraryUse)
    );
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
fn prune_occupation_choices_removes_duplicate_and_fixed_conflicts() {
    let mut app = test_app();
    app.set_occupation("Student".to_owned());
    app.occupation_choices
        .insert(ChoiceKey::new("student-any", 0), "Library Use".to_owned());
    app.occupation_choices
        .insert(ChoiceKey::new("student-any", 1), "Charm".to_owned());
    app.occupation_choices
        .insert(ChoiceKey::new("student-any", 2), "Charm".to_owned());
    app.occupation_choices
        .insert(ChoiceKey::new("student-any", 3), "Law".to_owned());

    let occupation = app.selected_occupation().expect("student should exist");
    app.prune_occupation_choices_for(&occupation);

    assert!(
        !app.occupation_choices
            .contains_key(&ChoiceKey::new("student-any", 0))
    );
    assert_eq!(
        app.occupation_choices
            .get(&ChoiceKey::new("student-any", 1)),
        Some(&"Charm".to_owned())
    );
    assert!(
        !app.occupation_choices
            .contains_key(&ChoiceKey::new("student-any", 2))
    );
    assert_eq!(
        app.occupation_choices
            .get(&ChoiceKey::new("student-any", 3)),
        Some(&"Law".to_owned())
    );
    assert_eq!(app.unresolved_choice_count_for(&occupation), 2);
}

#[test]
fn set_occupation_choice_rejects_duplicate_and_fixed_conflicts() {
    let mut app = test_app();
    app.set_occupation("Student".to_owned());
    assert!(
        !app.set_occupation_choice(ChoiceKey::new("student-any", 0), "Library Use".to_owned(),)
    );
    assert!(
        !app.occupation_choices
            .contains_key(&ChoiceKey::new("student-any", 0))
    );

    assert!(app.set_occupation_choice(ChoiceKey::new("student-any", 0), "Charm".to_owned(),));
    assert!(!app.set_occupation_choice(ChoiceKey::new("student-any", 1), "Charm".to_owned(),));
    assert!(
        !app.occupation_choices
            .contains_key(&ChoiceKey::new("student-any", 1))
    );
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
        app.allocations.occupation_points.get(&Skill::CreditRating),
        Some(&9)
    );
    assert!(app.credit_rating() <= 30);
}

#[test]
fn quick_skill_package_never_overspends_occupation_budget() {
    let mut app = test_app();
    app.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 15),
            ("CON", 15),
            ("SIZ", 15),
            ("DEX", 15),
            ("APP", 15),
            ("INT", 15),
            ("POW", 15),
            ("EDU", 15),
        ],
    );
    app.set_occupation("Nurse".to_owned());
    resolve_nurse_choice(&mut app);

    app.apply_quick_skill_package();

    let math = app.sheet_math();
    let used_occ = CoC7eApp::used_occupation_points_from(&math.skill_rows);
    assert!(
        used_occ <= math.occupation_budget,
        "quick package used {used_occ} occupation points but only {} are available",
        math.occupation_budget
    );
    assert!(
        math.skill_rows
            .iter()
            .all(|row| row.total <= MAX_CREATION_VALUE),
        "quick package should not push any skill over the creation cap"
    );
}

#[test]
fn quick_skill_package_spends_remaining_budget_when_skill_caps_allow_it() {
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
            ("EDU", 90),
        ],
    );
    app.set_occupation("Nurse".to_owned());
    resolve_nurse_choice(&mut app);

    app.apply_quick_skill_package();

    let math = app.sheet_math();
    let used_occ = CoC7eApp::used_occupation_points_from(&math.skill_rows);
    assert_eq!(used_occ, math.occupation_budget);
    assert!(
        math.skill_rows
            .iter()
            .all(|row| row.total <= MAX_CREATION_VALUE),
        "quick package should not push any skill over the creation cap"
    );
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
            Skill::LibraryUse,
            Skill::SpotHidden,
            Skill::Listen,
            Skill::Stealth,
            Skill::Persuade,
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
fn sanitize_custom_occupation_cleans_raw_imported_state() {
    let mut app = test_app();
    app.custom_occupation.credit_min = -999;
    app.custom_occupation.credit_max = 999;
    app.custom_occupation.skills = vec![
        " Library Use ".to_owned(),
        "Bogus Skill".to_owned(),
        "Credit Rating".to_owned(),
        "Library Use".to_owned(),
        String::new(),
        "Spot Hidden".to_owned(),
        " Cthulhu Mythos ".to_owned(),
        "Listen".to_owned(),
        "Law".to_owned(),
    ];

    app.sanitize_custom_occupation();

    assert_eq!(app.custom_occupation.credit_min, 0);
    assert_eq!(app.custom_occupation.credit_max, 99);
    assert_eq!(
        app.custom_occupation.skills,
        vec![
            "Library Use".to_owned(),
            String::new(),
            String::new(),
            "Library Use".to_owned(),
            String::new(),
            "Spot Hidden".to_owned(),
            String::new(),
            "Listen".to_owned(),
        ]
    );
    assert!(app.custom_occupation.skill_slot_labels.contains_key(&0));
    assert!(app.custom_occupation.skill_slot_labels.contains_key(&3));
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
            .contains(&Skill::Law)
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

    let selected_occupation = app.selected_occupation();
    assert_eq!(
        app.sheet_math().occupation_skill_set,
        app.occupation_skill_set_for(selected_occupation.as_ref())
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
        vec![Skill::LibraryUse]
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

    let selectable_option_names: HashSet<&str> = OCCUPATION_SELECTABLE_SKILL_OPTIONS
        .iter()
        .map(|skill| skill.name())
        .collect();
    assert_eq!(selectable_option_names, selectable_actual);

    for skill in ART_SKILL_OPTIONS {
        assert!(
            spec_names.contains(skill.name()),
            "unknown art skill: {}",
            skill.name()
        );
        assert!(skill.name().starts_with("Art/Craft"));
    }

    for skill in SCIENCE_SKILL_OPTIONS {
        assert!(
            spec_names.contains(skill.name()),
            "unknown science skill: {}",
            skill.name()
        );
        assert!(skill.name().starts_with("Science"));
    }

    for skill in INTERPERSONAL_SKILL_OPTIONS {
        assert!(
            spec_names.contains(skill.name()),
            "unknown interpersonal skill: {}",
            skill.name()
        );
    }

    for skill in FIREARMS_SKILL_OPTIONS {
        assert!(
            spec_names.contains(skill.name()),
            "unknown firearms skill: {}",
            skill.name()
        );
        assert!(skill.name().starts_with("Firearms"));
    }
}

#[test]
fn runtime_skill_constant_validation_checks_typed_option_mirrors() {
    assert!(skill_constant_validation_errors().is_empty());

    let mismatch_errors = typed_skill_list_validation_errors(
        "TEST_SKILL_OPTIONS",
        &["Accounting", "Anthropology"],
        &[Skill::Accounting, Skill::Climb],
    );
    assert!(mismatch_errors.iter().any(|error| {
        error.contains("TEST_SKILL_OPTIONS typed skill list must match its string skill list")
            && error.contains("Anthropology")
            && error.contains("Climb")
    }));

    let duplicate_errors = typed_skill_list_validation_errors(
        "TEST_DUPLICATE_OPTIONS",
        &["Accounting", "Anthropology"],
        &[Skill::Accounting, Skill::Accounting],
    );
    assert!(duplicate_errors.iter().any(|error| {
        error.contains("TEST_DUPLICATE_OPTIONS typed skill list contains duplicate entries")
    }));

    let order_errors = typed_skill_list_validation_errors(
        "TEST_ORDER_OPTIONS",
        &["Accounting", "Anthropology", "Appraise"],
        &[Skill::Accounting, Skill::Appraise, Skill::Anthropology],
    );
    assert!(order_errors.iter().any(|error| {
        error.contains("TEST_ORDER_OPTIONS typed skill list order must match its string skill list")
    }));
}

#[test]
fn skill_rows_carry_typed_skill_ids_matching_display_names() {
    let app = test_app();
    let math = app.sheet_math();

    for row in math.skill_rows {
        assert_eq!(row.id.name(), row.name);
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
        app.resolved_occupation_skills_for(&occupation)
            .iter()
            .all(|skill| skill.name() != "Bogus Skill")
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
    let pools = vec![
        vec![Skill::Climb, Skill::Swim],
        vec![Skill::Climb],
        vec![Skill::FirstAid],
    ];
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
            fixed(Skill::Accounting),
            fixed(Skill::Anthropology),
            fixed(Skill::Appraise),
            fixed(Skill::Archaeology),
            fixed(Skill::ArtCraft),
            fixed(Skill::Charm),
            fixed(Skill::Climb),
            choice(
                "bad-hidden",
                "Impossible choice",
                vec![Skill::Accounting],
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
            fixed(Skill::Accounting),
            fixed(Skill::Anthropology),
            fixed(Skill::Appraise),
            fixed(Skill::Archaeology),
            fixed(Skill::ArtCraft),
            fixed(Skill::Charm),
            choice("bad-a", "Bad A", vec![Skill::Climb], 1),
            choice("bad-b", "Bad B", vec![Skill::Climb], 1),
        ],
    )];

    validate_occupations(&occupations);
}

#[test]
fn summary_generation_method_check_is_method_aware() {
    let mut app = test_app();

    assert_eq!(
        app.summary_generation_method_check(),
        (CheckState::Pass, "Generation method Roll".to_owned())
    );

    app.apply_characteristic_preset(
        CharMethod::PointBuy,
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
    assert_eq!(
        app.summary_generation_method_check(),
        (CheckState::Pass, "Point budget 460/460".to_owned())
    );

    app.chars.set_char(Characteristic::Str, 55);
    assert_eq!(
        app.summary_generation_method_check(),
        (CheckState::Fail, "Point budget 465/460".to_owned())
    );

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
    assert_eq!(
        app.summary_generation_method_check(),
        (CheckState::Pass, "Generation method Quick Array".to_owned())
    );

    app.char_method = CharMethod::Mixed;
    assert_eq!(
        app.summary_generation_method_check(),
        (CheckState::Warn, "Generation method Mixed".to_owned())
    );
}

#[test]
fn summary_blockers_prevent_copying_incomplete_sheet() {
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

    let blockers = app.summary_blockers_for(&app.sheet_math());

    assert!(blockers.iter().any(|blocker| blocker == "Luck not rolled"));
    assert!(
        blockers
            .iter()
            .any(|blocker| blocker.starts_with("occupation points "))
    );
}

#[test]
fn plaintext_summary_trims_concept_and_backstory_output() {
    let mut app = test_app();
    app.concept.name = "  Ida Know  ".to_owned();
    app.concept.pronouns = "  she/her  ".to_owned();
    app.concept.residence = "  Arkham  ".to_owned();
    app.concept.birthplace = "  Boston  ".to_owned();
    app.backstory.insert(
        "Traits".to_owned(),
        "  Writes everything down before speaking.  ".to_owned(),
    );

    let summary = app.plaintext_summary();

    assert!(summary.contains("Ida Know"));
    assert!(summary.contains("Pronouns/Gender: she/her"));
    assert!(summary.contains("Residence: Arkham"));
    assert!(summary.contains("Birthplace: Boston"));
    assert!(summary.contains("Traits: Writes everything down before speaking."));
    assert!(!summary.contains("  Ida Know  "));
    assert!(!summary.contains("Pronouns/Gender:   she/her  "));
    assert!(!summary.contains("Traits:   Writes everything down before speaking.  "));
}

#[test]
fn occupation_validation_errors_are_collectible_without_startup_panic() {
    let occupations = vec![
        occupation(
            "Duplicate Occupation",
            (0, 10),
            vec![FormulaKey::Edu4],
            vec![
                fixed(Skill::Accounting),
                fixed(Skill::Anthropology),
                fixed(Skill::Appraise),
                fixed(Skill::Archaeology),
                fixed(Skill::ArtCraft),
                fixed(Skill::Charm),
                fixed(Skill::Climb),
                fixed(Skill::Disguise),
            ],
        ),
        occupation(
            "Duplicate Occupation",
            (0, 10),
            vec![FormulaKey::Edu4],
            vec![
                fixed(Skill::Accounting),
                fixed(Skill::Anthropology),
                fixed(Skill::Appraise),
                fixed(Skill::Archaeology),
                fixed(Skill::ArtCraft),
                fixed(Skill::Charm),
                fixed(Skill::Climb),
                fixed(Skill::Disguise),
            ],
        ),
    ];

    let errors = occupation_validation_errors(&occupations);

    assert!(
        errors
            .iter()
            .any(|error| error.contains("duplicate occupation name"))
    );
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
                fixed(Skill::Accounting),
                fixed(Skill::Anthropology),
                fixed(Skill::Appraise),
                fixed(Skill::Archaeology),
                fixed(Skill::ArtCraft),
                fixed(Skill::Charm),
                fixed(Skill::Climb),
                fixed(Skill::Disguise),
            ],
        ),
        occupation(
            "Duplicate Occupation",
            (0, 10),
            vec![FormulaKey::Edu4],
            vec![
                fixed(Skill::Accounting),
                fixed(Skill::Anthropology),
                fixed(Skill::Appraise),
                fixed(Skill::Archaeology),
                fixed(Skill::ArtCraft),
                fixed(Skill::Charm),
                fixed(Skill::Climb),
                fixed(Skill::Disguise),
            ],
        ),
    ];

    validate_occupations(&occupations);
}

#[test]
fn occupation_validation_rejects_names_that_only_differ_by_outer_whitespace() {
    let occupations = vec![
        occupation(
            "Duplicate Occupation",
            (0, 10),
            vec![FormulaKey::Edu4],
            vec![
                fixed(Skill::Accounting),
                fixed(Skill::Anthropology),
                fixed(Skill::Appraise),
                fixed(Skill::Archaeology),
                fixed(Skill::ArtCraft),
                fixed(Skill::Charm),
                fixed(Skill::Climb),
                fixed(Skill::Disguise),
            ],
        ),
        occupation(
            "  Duplicate Occupation  ",
            (0, 10),
            vec![FormulaKey::Edu4],
            vec![
                fixed(Skill::Accounting),
                fixed(Skill::Anthropology),
                fixed(Skill::Appraise),
                fixed(Skill::Archaeology),
                fixed(Skill::ArtCraft),
                fixed(Skill::Charm),
                fixed(Skill::Climb),
                fixed(Skill::Disguise),
            ],
        ),
    ];

    let errors = occupation_validation_errors(&occupations);

    assert!(
        errors
            .iter()
            .any(|error| error.contains("duplicate occupation name"))
    );
}

#[test]
fn occupation_validation_rejects_choice_ids_that_only_differ_by_outer_whitespace() {
    let occupations = vec![occupation(
        "Whitespace Choice Id",
        (0, 10),
        vec![FormulaKey::Edu4],
        vec![
            fixed(Skill::Accounting),
            fixed(Skill::Anthropology),
            fixed(Skill::Appraise),
            fixed(Skill::Archaeology),
            fixed(Skill::ArtCraft),
            fixed(Skill::Charm),
            choice(
                "duplicate-choice",
                "First choice",
                vec![Skill::Climb, Skill::Disguise],
                1,
            ),
            choice(
                " duplicate-choice ",
                "Second choice",
                vec![Skill::Dodge, Skill::DriveAuto],
                1,
            ),
        ],
    )];

    let errors = occupation_validation_errors(&occupations);

    assert!(
        errors
            .iter()
            .any(|error| error.contains("choice id") && error.contains("outer whitespace"))
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("duplicate choice id"))
    );
}

#[test]
fn occupation_validation_rejects_duplicate_choice_options() {
    let occupations = vec![occupation(
        "Duplicate Choice Option",
        (0, 10),
        vec![FormulaKey::Edu4],
        vec![
            fixed(Skill::Accounting),
            fixed(Skill::Anthropology),
            fixed(Skill::Appraise),
            fixed(Skill::Archaeology),
            fixed(Skill::ArtCraft),
            fixed(Skill::Climb),
            fixed(Skill::Disguise),
            choice(
                "duplicate-option",
                "Duplicate option",
                vec![Skill::Charm, Skill::Charm],
                1,
            ),
        ],
    )];

    let errors = occupation_validation_errors(&occupations);

    assert!(
        errors
            .iter()
            .any(|error| error.contains("duplicate option"))
    );
}

#[test]
fn json_save_round_trips_editable_investigator_state() {
    let mut app = test_app();
    app.concept.name = "Ida Know".to_owned();
    app.concept.pronouns = "she/her".to_owned();
    app.concept.residence = "Arkham".to_owned();
    app.concept.birthplace = "Boston".to_owned();
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
    app.set_age(55);
    app.set_age_deduction(Characteristic::Str, 5);
    app.set_age_deduction(Characteristic::Con, 5);
    app.set_occupation("Nurse".to_owned());
    resolve_nurse_choice(&mut app);
    app.set_occupation_allocation("First Aid", 20);
    app.set_personal_allocation("Accounting", 15);
    app.luck_state.value = Some(55);
    app.luck_state.rolls = vec![DiceResult {
        rolls: vec![3, 4, 4],
        plus_six: false,
        value: 55,
        kept: Some(true),
    }];
    app.backstory
        .insert("Traits".to_owned(), "Writes everything down.".to_owned());

    let json = app.export_json_save().expect("save should serialize");
    assert!(json.contains("Ida Know"));
    assert!(json.contains("nurse-interpersonal"));

    let mut loaded = test_app();
    loaded
        .import_json_save(&json)
        .expect("fresh app should load its own save format");

    assert_eq!(loaded.concept.name, "Ida Know");
    assert_eq!(loaded.concept.age, 55);
    assert_eq!(loaded.char_method, CharMethod::QuickArray);
    assert_eq!(loaded.char_value("EDU"), 80);
    assert_eq!(loaded.age_deductions.get_char(Characteristic::Str), 5);
    assert_eq!(loaded.age_deductions.get_char(Characteristic::Con), 5);
    assert_eq!(loaded.occupation_id, "Nurse");
    assert_eq!(
        loaded
            .occupation_choices
            .get(&ChoiceKey::new("nurse-interpersonal", 0))
            .map(String::as_str),
        Some("Persuade")
    );
    assert_eq!(
        loaded.allocations.occupation_points.get(&Skill::FirstAid),
        Some(&20)
    );
    assert_eq!(
        loaded.allocations.personal_points.get(&Skill::Accounting),
        Some(&15)
    );
    assert_eq!(loaded.luck_state.value, Some(55));
    assert_eq!(
        loaded.backstory.get("Traits").map(String::as_str),
        Some("Writes everything down.")
    );
}

#[test]
fn json_export_uses_named_characteristics_and_stable_save_map_keys() {
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
        .occupation_points
        .insert(Skill::LibraryUse, 10);
    app.allocations
        .occupation_points
        .insert(Skill::Accounting, 5);
    app.char_rolls.insert(
        "STR".to_owned(),
        DiceResult {
            rolls: vec![3, 3, 4],
            plus_six: false,
            value: 50,
            kept: None,
        },
    );
    app.char_rolls.insert(
        "DEX".to_owned(),
        DiceResult {
            rolls: vec![4, 4, 2],
            plus_six: false,
            value: 50,
            kept: None,
        },
    );
    app.backstory
        .insert("Traits".to_owned(), "Careful note-taker.".to_owned());
    app.backstory.insert(
        "Ideology & Beliefs".to_owned(),
        "Trusts observable evidence.".to_owned(),
    );

    let json = app.export_json_save().expect("save should serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("exported save should parse");

    assert_eq!(value["chars"]["STR"], 50);
    assert_eq!(value["chars"]["EDU"], 80);
    assert!(value["chars"].as_array().is_none());

    let occupation_points = value["allocations"]["occupation_points"]
        .as_object()
        .expect("allocation points should serialize as an object");
    let keys: Vec<&str> = occupation_points.keys().map(String::as_str).collect();
    assert_eq!(keys, vec!["Accounting", "Library Use"]);

    let key_position_after = |anchor: &str, key: &str| {
        let start = json
            .find(anchor)
            .unwrap_or_else(|| panic!("missing JSON anchor {anchor}"));
        let needle = format!("\"{key}\"");
        json[start..]
            .find(&needle)
            .map(|offset| start + offset)
            .unwrap_or_else(|| panic!("missing JSON key {key} after {anchor}"))
    };

    assert!(
        key_position_after("\"char_rolls\"", "DEX") < key_position_after("\"char_rolls\"", "STR")
    );
    assert!(
        key_position_after("\"backstory\"", "Ideology & Beliefs")
            < key_position_after("\"backstory\"", "Traits")
    );
}

#[test]
fn json_import_accepts_legacy_ordered_characteristic_arrays() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");
    let mut value: serde_json::Value =
        serde_json::from_str(&json).expect("exported save should parse");

    value["chars"] = serde_json::json!([50, 55, 60, 65, 70, 75, 80, 85]);
    value["age_deductions"] = serde_json::json!([0, 5, 0, 0, 0, 0, 0, 0]);

    let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
    let mut loaded = test_app();
    loaded
        .import_json_save(&edited_json)
        .expect("legacy characteristic arrays should still import");

    assert_eq!(loaded.char_value("STR"), 50);
    assert_eq!(loaded.char_value("EDU"), 85);
}

#[test]
fn json_import_accepts_legacy_characteristic_values_objects() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");
    let mut value: serde_json::Value =
        serde_json::from_str(&json).expect("exported save should parse");

    value["chars"] = serde_json::json!({
        "values": [50, 55, 60, 65, 70, 75, 80, 85]
    });
    value["age_deductions"] = serde_json::json!({
        "values": [0, 5, 0, 0, 0, 0, 0, 0]
    });

    let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
    let mut loaded = test_app();
    loaded
        .import_json_save(&edited_json)
        .expect("legacy values-object characteristic saves should still import");

    assert_eq!(loaded.char_value("STR"), 50);
    assert_eq!(loaded.char_value("EDU"), 85);
}

#[test]
fn json_import_rejects_incomplete_named_characteristic_maps() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");
    let mut value: serde_json::Value =
        serde_json::from_str(&json).expect("exported save should parse");

    value["chars"] = serde_json::json!({
        "STR": 50,
        "CON": 55,
        "SIZ": 60,
        "DEX": 65,
        "APP": 70,
        "INT": 75,
        "POW": 80
    });

    let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
    let mut loaded = test_app();
    let error = loaded
        .import_json_save(&edited_json)
        .expect_err("missing EDU should reject named characteristic maps");

    assert!(error.contains("EDU") || error.contains("missing"));
}

#[test]
fn json_import_reports_unsupported_future_save_versions() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");
    let mut value: serde_json::Value =
        serde_json::from_str(&json).expect("exported save should be valid JSON");
    value["version"] = serde_json::Value::from((INVESTIGATOR_SAVE_VERSION + 1) as u64);

    let mut loaded = test_app();
    let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
    let error = loaded
        .import_json_save(&edited_json)
        .expect_err("future save versions should be rejected");

    assert!(error.contains("unsupported save version"));
}

#[test]
fn json_import_sanitizes_characteristics_and_stale_rolls() {
    let source = test_app();
    let mut save = source.save_file();
    save.chars.set_char(Characteristic::Str, i32::MAX);
    save.chars.set_char(Characteristic::Siz, -50);
    save.chars.set_char(Characteristic::Dex, 52);
    save.char_rolls.insert(
        "STR".to_owned(),
        DiceResult {
            rolls: vec![1, 1, 1],
            plus_six: false,
            value: i32::MAX,
            kept: None,
        },
    );
    save.char_rolls.insert(
        "BOGUS".to_owned(),
        DiceResult {
            rolls: vec![1, 1, 1],
            plus_six: false,
            value: 15,
            kept: None,
        },
    );

    let json = serde_json::to_string(&save).expect("test save should serialize");
    let mut loaded = test_app();
    loaded
        .import_json_save(&json)
        .expect("sanitized save should import");

    assert_eq!(loaded.chars.get_char(Characteristic::Str), 90);
    assert_eq!(loaded.chars.get_char(Characteristic::Siz), 0);
    assert_eq!(loaded.chars.get_char(Characteristic::Dex), 50);
    assert!(loaded.char_rolls.is_empty());
}

#[test]
fn json_import_normalizes_roll_method_when_roll_evidence_is_removed() {
    let mut source = test_app();
    source.apply_characteristic_preset(
        CharMethod::QuickArray,
        &[
            ("STR", 50),
            ("CON", 50),
            ("SIZ", 60),
            ("DEX", 50),
            ("APP", 50),
            ("INT", 70),
            ("POW", 60),
            ("EDU", 80),
        ],
    );
    let mut save = source.save_file();
    save.char_method = CharMethod::Roll;
    save.char_rolls.clear();
    save.char_rolls.insert(
        "STR".to_owned(),
        DiceResult {
            rolls: vec![1, 1, 1],
            plus_six: false,
            value: 15,
            kept: None,
        },
    );

    let json = serde_json::to_string(&save).expect("test save should serialize");
    let mut loaded = test_app();
    let report = loaded
        .import_json_save(&json)
        .expect("save with stale roll method should import");

    assert_eq!(loaded.char_method, CharMethod::Mixed);
    assert!(
        report
            .normalized_import_fields
            .iter()
            .any(|entry| entry.contains("char_method: Roll → Mixed")),
        "expected char_method normalization report, got {report:?}"
    );
}

#[test]
fn json_import_normalizes_roll_method_when_roll_evidence_is_partial() {
    let mut source = test_app();
    source.roll_all_characteristics();
    let mut save = source.save_file();
    let str_roll = save
        .char_rolls
        .get("STR")
        .cloned()
        .expect("rolled save should include STR evidence");
    save.char_rolls.clear();
    save.char_rolls.insert("STR".to_owned(), str_roll);

    let json = serde_json::to_string(&save).expect("test save should serialize");
    let mut loaded = test_app();
    let report = loaded
        .import_json_save(&json)
        .expect("save with partial roll evidence should import");

    assert_eq!(loaded.char_method, CharMethod::Mixed);
    assert_eq!(loaded.char_rolls.len(), 1);
    assert!(loaded.char_rolls.contains_key("STR"));
    assert!(
        report
            .normalized_import_fields
            .iter()
            .any(|entry| entry.contains("char_method: Roll → Mixed")),
        "expected char_method normalization report, got {report:?}"
    );
}

#[test]
fn json_import_recomputes_edu_bonus_from_imported_rolls() {
    let source = test_app();
    let mut save = source.save_file();
    save.concept.age = 40;
    save.chars.set_char(Characteristic::Edu, 40);
    save.edu_bonus = 99;
    save.edu_check_rolls = vec![
        EduCheckRoll {
            d100: 100,
            improved: false,
            gain: 7,
            resulting_edu: 99,
        },
        EduCheckRoll {
            d100: -50,
            improved: true,
            gain: 9,
            resulting_edu: 99,
        },
    ];

    let json = serde_json::to_string(&save).expect("test save should serialize");
    let mut loaded = test_app();
    let report = loaded
        .import_json_save(&json)
        .expect("sanitized save should import");

    assert_eq!(loaded.edu_bonus, 7);
    assert_eq!(loaded.edu_check_rolls.len(), 1);
    assert_eq!(loaded.edu_check_rolls[0].d100, 100);
    assert!(loaded.edu_check_rolls[0].improved);
    assert_eq!(loaded.edu_check_rolls[0].gain, 7);
    assert_eq!(loaded.edu_check_rolls[0].resulting_edu, 47);
    assert!(
        report
            .removed_unknown_import_entries
            .iter()
            .any(|entry| entry.contains("edu_check_rolls[1].d100: expected 1..=100")),
        "expected invalid d100 report, got {report:?}"
    );
}

#[test]
fn json_import_clears_luck_without_valid_roll_evidence() {
    let source = test_app();
    let mut save = source.save_file();
    save.luck_state.value = Some(99);
    save.luck_state.rolls.clear();

    let json = serde_json::to_string(&save).expect("test save should serialize");
    let mut loaded = test_app();
    loaded
        .import_json_save(&json)
        .expect("sanitized save should import");

    assert_eq!(loaded.luck_state.value, None);
    assert!(loaded.luck_state.rolls.is_empty());
}

#[test]
fn json_import_recomputes_luck_from_valid_roll_evidence() {
    let source = test_app();
    let mut save = source.save_file();
    save.concept.age = 18;
    save.luck_state.value = Some(99);
    save.luck_state.rolls = vec![
        DiceResult {
            rolls: vec![1, 1, 1],
            plus_six: false,
            value: 15,
            kept: Some(true),
        },
        DiceResult {
            rolls: vec![6, 6, 6],
            plus_six: false,
            value: 90,
            kept: Some(false),
        },
    ];

    let json = serde_json::to_string(&save).expect("test save should serialize");
    let mut loaded = test_app();
    loaded
        .import_json_save(&json)
        .expect("sanitized save should import");

    assert_eq!(loaded.luck_state.value, Some(90));
    assert_eq!(loaded.luck_state.rolls.len(), 2);
    assert_eq!(loaded.luck_state.rolls[0].kept, Some(false));
    assert_eq!(loaded.luck_state.rolls[1].kept, Some(true));
}

#[test]
fn json_import_drops_matching_value_rolls_with_invalid_dice_evidence() {
    let source = test_app();
    let mut save = source.save_file();
    save.chars.set_char(Characteristic::Str, 50);
    save.char_rolls.insert(
        "STR".to_owned(),
        DiceResult {
            rolls: Vec::new(),
            plus_six: false,
            value: 50,
            kept: None,
        },
    );

    let json = serde_json::to_string(&save).expect("test save should serialize");
    let mut loaded = test_app();
    loaded
        .import_json_save(&json)
        .expect("sanitized save should import");

    assert_eq!(loaded.chars.get_char(Characteristic::Str), 50);
    assert!(!loaded.char_rolls.contains_key("STR"));
}

#[test]
fn json_import_sanitizes_custom_occupation_raw_state_before_resaving() {
    let source = test_app();
    let mut save = source.save_file();
    save.occupation_id = CUSTOM_OCCUPATION_ID.to_owned();
    save.custom_occupation.credit_min = -20;
    save.custom_occupation.credit_max = 140;
    save.custom_occupation.skills = vec![
        " Library Use ".to_owned(),
        "Credit Rating".to_owned(),
        "Bogus Skill".to_owned(),
        "Spot Hidden".to_owned(),
        "Library Use".to_owned(),
        "Listen".to_owned(),
        "Cthulhu Mythos".to_owned(),
        "Stealth".to_owned(),
        "Persuade".to_owned(),
    ];

    let json = serde_json::to_string(&save).expect("test save should serialize");
    let mut loaded = test_app();
    loaded
        .import_json_save(&json)
        .expect("sanitized save should import");

    assert_eq!(loaded.custom_occupation.credit_min, 0);
    assert_eq!(loaded.custom_occupation.credit_max, 99);
    assert_eq!(
        loaded.custom_occupation.skills,
        vec![
            "Library Use".to_owned(),
            String::new(),
            String::new(),
            "Spot Hidden".to_owned(),
            "Library Use".to_owned(),
            "Listen".to_owned(),
            String::new(),
            "Stealth".to_owned(),
        ]
    );
    assert!(loaded.custom_occupation.skill_slot_labels.contains_key(&0));
    assert!(loaded.custom_occupation.skill_slot_labels.contains_key(&4));

    let exported = loaded.save_file();
    assert_eq!(exported.custom_occupation.credit_min, 0);
    assert_eq!(exported.custom_occupation.credit_max, 99);
    assert_eq!(
        exported.custom_occupation.skills,
        loaded.custom_occupation.skills
    );

    let occupation = loaded
        .selected_occupation()
        .expect("custom occupation should remain selected");
    assert_eq!(
        loaded.resolved_occupation_skills_for(&occupation),
        vec![
            Skill::LibraryUse,
            Skill::SpotHidden,
            Skill::LibraryUse,
            Skill::Listen,
            Skill::Stealth,
        ]
    );
}

#[test]
fn json_import_drops_unknown_backstory_categories() {
    let source = test_app();
    let mut save = source.save_file();
    save.backstory
        .insert("Traits".to_owned(), "Cautious note-taker.".to_owned());
    save.backstory.insert(
        " Unknown Category ".to_owned(),
        "This should not be preserved.".to_owned(),
    );
    save.backstory.insert(
        "Phobias & Manias ".to_owned(),
        "Hates deep water.".to_owned(),
    );

    let json = serde_json::to_string(&save).expect("test save should serialize");
    let mut loaded = test_app();
    loaded
        .import_json_save(&json)
        .expect("sanitized save should import");

    assert_eq!(
        loaded.backstory.get("Traits").map(String::as_str),
        Some("Cautious note-taker.")
    );
    assert_eq!(
        loaded.backstory.get("Phobias & Manias").map(String::as_str),
        Some("Hates deep water.")
    );
    assert!(!loaded.backstory.contains_key("Unknown Category"));
    assert_eq!(loaded.backstory.len(), 2);
}

#[test]
#[should_panic(expected = "duplicate formula")]
fn occupation_validation_rejects_duplicate_formulas() {
    let occupations = vec![occupation(
        "Bad Formula Occupation",
        (0, 10),
        vec![FormulaKey::Edu4, FormulaKey::Edu4],
        vec![
            fixed(Skill::Accounting),
            fixed(Skill::Anthropology),
            fixed(Skill::Appraise),
            fixed(Skill::Archaeology),
            fixed(Skill::ArtCraft),
            fixed(Skill::Charm),
            fixed(Skill::Climb),
            fixed(Skill::Disguise),
        ],
    )];

    validate_occupations(&occupations);
}

#[test]
fn json_import_accepts_missing_version_as_legacy_v0_save() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");
    let mut value: serde_json::Value =
        serde_json::from_str(&json).expect("exported save should parse");
    value
        .as_object_mut()
        .expect("save root should be an object")
        .remove("version");

    let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
    let mut loaded = test_app();
    let report = loaded
        .import_json_save(&edited_json)
        .expect("missing-version saves should migrate as legacy v0");

    assert!(report.is_clean());
    assert_eq!(loaded.save_file().version, INVESTIGATOR_SAVE_VERSION);
}

#[test]
fn json_import_reports_corrected_allocations_and_unknown_allocation_keys() {
    let mut source = test_app();
    source.apply_characteristic_preset(
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
    source.set_occupation("Nurse".to_owned());
    resolve_nurse_choice(&mut source);

    let json = source.export_json_save().expect("save should serialize");
    let mut value: serde_json::Value =
        serde_json::from_str(&json).expect("exported save should parse");
    value["allocations"]["occupation_points"]["First Aid"] = serde_json::json!(999);
    value["allocations"]["personal_points"]["Credit Rating"] = serde_json::json!(30);
    value["allocations"]["personal_points"]["Bogus Skill"] = serde_json::json!(10);

    let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
    let mut loaded = test_app();
    let report = loaded
        .import_json_save(&edited_json)
        .expect("sanitized save should import");

    assert!(
        report
            .clamped_allocations
            .iter()
            .any(|entry| entry.contains("First Aid")),
        "expected clamped First Aid allocation, got {report:?}"
    );
    assert!(
        report
            .removed_allocations
            .iter()
            .any(|entry| entry.contains("Credit Rating")),
        "expected removed Credit Rating personal allocation, got {report:?}"
    );
    assert!(
        report
            .removed_unknown_import_entries
            .iter()
            .any(|entry| entry.contains("Bogus Skill")),
        "expected unknown Bogus Skill allocation report, got {report:?}"
    );
}

#[test]
fn custom_occupation_can_use_lower_required_skill_count() {
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
    app.set_custom_occupation_required_skill_count(3);
    app.set_custom_occupation_skill(0, "Library Use".to_owned());
    app.set_custom_occupation_skill(1, "Spot Hidden".to_owned());

    let occupation = app
        .selected_occupation()
        .expect("custom occupation should exist");
    assert_eq!(app.required_occupation_skill_count_for(&occupation), 3);
    assert_eq!(app.unique_occupation_shortfall_for(&occupation), 1);

    app.set_custom_occupation_skill(2, "Listen".to_owned());
    let occupation = app
        .selected_occupation()
        .expect("custom occupation should exist");
    assert_eq!(app.unique_occupation_shortfall_for(&occupation), 0);
    assert_eq!(
        app.resolved_occupation_skills_for(&occupation),
        vec![Skill::LibraryUse, Skill::SpotHidden, Skill::Listen]
    );
}

#[test]
fn custom_occupation_skill_labels_drive_display_without_changing_canonical_skill_ids() {
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
    app.set_custom_occupation_required_skill_count(1);
    assert!(app.set_custom_occupation_skill(0, "Language (Other)".to_owned()));
    assert!(
        app.set_custom_occupation_skill_label(Skill::LanguageOther, "Language (Latin)".to_owned())
    );

    let math = app.sheet_math();
    let row = math
        .skill_rows
        .iter()
        .find(|row| row.id == Skill::LanguageOther)
        .expect("Language (Other) row should exist");
    assert_eq!(row.name.as_str(), "Language (Latin)");

    app.set_occupation_allocation_for_instance(Skill::LanguageOther, Some(0), 20);
    assert_eq!(app.allocations.custom_occupation_points.get(&0), Some(&20));
}

#[test]
fn custom_occupation_duplicate_specialties_get_independent_rows_and_allocations() {
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
    app.set_custom_occupation_required_skill_count(2);
    assert!(app.set_custom_occupation_skill(0, "Language (Other)".to_owned()));
    assert!(app.set_custom_occupation_skill(1, "Language (Other)".to_owned()));
    assert!(app.set_custom_occupation_skill_label_for_slot(0, "Language (Latin)".to_owned()));
    assert!(app.set_custom_occupation_skill_label_for_slot(1, "Language (Greek)".to_owned()));

    let math = app.sheet_math();
    let language_rows: Vec<_> = math
        .skill_rows
        .iter()
        .filter(|row| row.id == Skill::LanguageOther)
        .collect();
    assert_eq!(language_rows.len(), 2);
    assert!(
        language_rows
            .iter()
            .any(|row| row.name == "Language (Latin)")
    );
    assert!(
        language_rows
            .iter()
            .any(|row| row.name == "Language (Greek)")
    );

    app.set_occupation_allocation_for_instance(Skill::LanguageOther, Some(0), 15);
    app.set_occupation_allocation_for_instance(Skill::LanguageOther, Some(1), 25);
    assert_eq!(app.allocations.custom_occupation_points.get(&0), Some(&15));
    assert_eq!(app.allocations.custom_occupation_points.get(&1), Some(&25));
}

#[test]
fn changing_custom_slot_skill_clears_slot_allocations() {
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
    app.set_custom_occupation_required_skill_count(1);
    assert!(app.set_custom_occupation_skill(0, "Language (Other)".to_owned()));
    assert!(app.set_custom_occupation_skill_label_for_slot(0, "Language (Latin)".to_owned()));
    app.set_occupation_allocation_for_instance(Skill::LanguageOther, Some(0), 20);
    app.set_personal_allocation_for_instance(Skill::LanguageOther, Some(0), 10);
    assert_eq!(app.allocations.custom_occupation_points.get(&0), Some(&20));
    assert_eq!(app.allocations.custom_personal_points.get(&0), Some(&10));

    assert!(app.set_custom_occupation_skill(0, "Pilot".to_owned()));

    assert_eq!(app.allocations.custom_occupation_points.get(&0), None);
    assert_eq!(app.allocations.custom_personal_points.get(&0), None);
    assert_eq!(app.custom_occupation.skill_slot_labels.get(&0), None);
}

#[test]
fn inactive_custom_slot_allocations_survive_required_count_changes() {
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
    assert!(app.set_custom_occupation_skill(0, "Library Use".to_owned()));
    assert!(app.set_custom_occupation_skill(1, "Spot Hidden".to_owned()));
    assert!(app.set_custom_occupation_skill(2, "Listen".to_owned()));
    assert!(app.set_custom_occupation_skill(3, "Stealth".to_owned()));
    app.set_occupation_allocation_for_instance(Skill::Stealth, Some(3), 25);
    app.set_personal_allocation_for_instance(Skill::Stealth, Some(3), 5);

    app.set_custom_occupation_required_skill_count(3);
    app.sanitize_state();
    assert_eq!(app.custom_occupation.skills[3], "Stealth");
    assert_eq!(app.allocations.custom_occupation_points.get(&3), Some(&25));
    assert_eq!(app.allocations.custom_personal_points.get(&3), Some(&5));

    app.set_custom_occupation_required_skill_count(4);
    let math = app.sheet_math();
    let stealth = math
        .skill_rows
        .iter()
        .find(|row| row.custom_index == Some(3) && row.id == Skill::Stealth)
        .expect("inactive custom Stealth slot should become active again");
    assert_eq!(stealth.occ_add, 25);
    assert_eq!(stealth.personal_add, 5);
}

#[test]
fn normalized_luck_is_reported_without_resetting_luck() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");
    let mut value: serde_json::Value =
        serde_json::from_str(&json).expect("exported save should parse");
    value["luck_state"] = serde_json::json!({
        "value": 15,
        "rolls": [
            {
                "rolls": [1, 1, 1],
                "plus_six": false,
                "value": 15,
                "kept": false
            },
            {
                "rolls": [6, 6, 6],
                "plus_six": false,
                "value": 90,
                "kept": true
            }
        ]
    });

    let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
    let mut loaded = test_app();
    let report = loaded
        .import_json_save(&edited_json)
        .expect("normalized Luck save should import");

    assert!(report.normalized_luck, "{report:?}");
    assert!(!report.reset_luck, "{report:?}");
    assert_eq!(loaded.luck_state.value, Some(15));
    assert_eq!(loaded.luck_state.rolls.len(), 1);
    assert_eq!(loaded.luck_state.rolls[0].kept, Some(true));
}

#[test]
fn json_export_uses_current_v2_save_version() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("save should parse");

    assert_eq!(value["version"], serde_json::json!(2));
    assert_eq!(app.save_file().version, INVESTIGATOR_SAVE_VERSION);
}

#[test]
fn json_import_accepts_v1_save_as_legacy_schema() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");
    let mut value: serde_json::Value = serde_json::from_str(&json).expect("save should parse");
    value["version"] = serde_json::json!(1);
    value
        .as_object_mut()
        .expect("save root should be an object")
        .remove("rng_roll_sides");
    value["custom_occupation"]
        .as_object_mut()
        .expect("custom occupation should be an object")
        .remove("skill_slot_labels");
    value["allocations"]
        .as_object_mut()
        .expect("allocations should be an object")
        .remove("custom_occupation_points");
    value["allocations"]
        .as_object_mut()
        .expect("allocations should be an object")
        .remove("custom_personal_points");

    let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
    let mut loaded = test_app();
    let report = loaded
        .import_json_save(&edited_json)
        .expect("v1 saves should migrate into the current schema");

    assert!(report.is_clean(), "{report:?}");
    assert_eq!(loaded.save_file().version, INVESTIGATOR_SAVE_VERSION);
}

#[test]
fn duplicate_custom_slot_labels_are_rejected_without_deleting_skills() {
    let mut app = test_app();
    app.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());
    app.set_custom_occupation_required_skill_count(2);
    assert!(app.set_custom_occupation_skill(0, "Language (Other)".to_owned()));
    assert!(app.set_custom_occupation_skill(1, "Language (Other)".to_owned()));

    let first_label = app
        .custom_occupation
        .skill_slot_labels
        .get(&0)
        .cloned()
        .expect("first duplicate should get an auto label");
    assert!(
        !app.set_custom_occupation_skill_label_for_slot(1, first_label),
        "duplicate slot labels should be rejected"
    );
    assert!(
        !app.set_custom_occupation_skill_label_for_slot(1, String::new()),
        "active duplicate labels must not be cleared"
    );

    app.sanitize_state();
    assert_eq!(app.custom_occupation.skills[0], "Language (Other)");
    assert_eq!(app.custom_occupation.skills[1], "Language (Other)");
}

#[test]
fn custom_slot_label_sanitizer_compares_truncated_labels() {
    let mut app = test_app();
    app.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());
    app.custom_occupation.required_skill_count = 2;
    app.custom_occupation.skills =
        vec!["Language (Other)".to_owned(), "Language (Other)".to_owned()];
    let shared_prefix = "L".repeat(64);
    app.custom_occupation
        .skill_slot_labels
        .insert(0, format!("{shared_prefix}A"));
    app.custom_occupation
        .skill_slot_labels
        .insert(1, format!("{shared_prefix}B"));

    app.sanitize_custom_occupation();

    assert_eq!(app.custom_occupation.skills[0], "Language (Other)");
    assert_eq!(app.custom_occupation.skills[1], "Language (Other)");
    assert_eq!(
        app.custom_occupation.skill_slot_labels.get(&0),
        Some(&shared_prefix)
    );
    let second = app
        .custom_occupation
        .skill_slot_labels
        .get(&1)
        .expect("second duplicate should be relabeled instead of removed");
    assert_ne!(second, &shared_prefix);
}

#[test]
fn generated_looking_custom_label_is_preserved_when_group_shrinks() {
    let mut app = test_app();
    app.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());
    app.set_custom_occupation_required_skill_count(2);
    assert!(app.set_custom_occupation_skill(0, "Language (Other)".to_owned()));
    assert!(app.set_custom_occupation_skill(1, "Language (Other)".to_owned()));
    assert!(app.set_custom_occupation_skill_label_for_slot(0, "Language (Other) 1".to_owned()));
    assert!(app.custom_occupation.skill_slot_labels.contains_key(&1));

    assert!(app.set_custom_occupation_skill(1, "Pilot".to_owned()));
    app.sanitize_state();

    assert_eq!(app.custom_occupation.skills[0], "Language (Other)");
    assert_eq!(app.custom_occupation.skills[1], "Pilot");
    assert_eq!(
        app.custom_occupation.skill_slot_labels.get(&0),
        Some(&"Language (Other) 1".to_owned())
    );
}

#[test]
fn inactive_custom_slot_allocations_are_clamped_while_preserved() {
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
    assert!(app.set_custom_occupation_skill(0, "Library Use".to_owned()));
    assert!(app.set_custom_occupation_skill(1, "Spot Hidden".to_owned()));
    assert!(app.set_custom_occupation_skill(2, "Listen".to_owned()));
    assert!(app.set_custom_occupation_skill(3, "Stealth".to_owned()));
    app.custom_occupation.required_skill_count = 3;
    app.allocations.custom_occupation_points.insert(3, 999);
    app.allocations.custom_personal_points.insert(3, 999);

    let report = app.sanitize_state_with_report();

    assert_eq!(app.custom_occupation.skills[3], "Stealth");
    assert_eq!(app.allocations.custom_occupation_points.get(&3), Some(&79));
    assert_eq!(app.allocations.custom_personal_points.get(&3), None);
    assert!(
        report
            .clamped_allocations
            .iter()
            .any(|entry| entry.contains("custom skill slot 4")),
        "expected inactive custom allocation clamp to be reported, got {report:?}"
    );
    assert!(
        report
            .removed_allocations
            .iter()
            .any(|entry| entry.contains("custom skill slot 4")),
        "expected inactive custom personal allocation removal to be reported, got {report:?}"
    );
}

#[test]
fn changing_occupation_clears_custom_occupation_points() {
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
    app.set_custom_occupation_required_skill_count(1);
    assert!(app.set_custom_occupation_skill(0, "Library Use".to_owned()));
    app.set_occupation_allocation_for_instance(Skill::LibraryUse, Some(0), 20);
    app.set_personal_allocation_for_instance(Skill::LibraryUse, Some(0), 10);
    assert_eq!(app.allocations.custom_occupation_points.get(&0), Some(&20));
    assert_eq!(app.allocations.custom_personal_points.get(&0), Some(&10));

    app.set_occupation("Nurse".to_owned());

    assert!(app.allocations.custom_occupation_points.is_empty());
    assert!(app.allocations.custom_personal_points.is_empty());
}

#[test]
fn imported_non_custom_occupation_drops_hidden_custom_allocations() {
    let mut source = test_app();
    source.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());
    source.set_custom_occupation_required_skill_count(1);
    assert!(source.set_custom_occupation_skill(0, "Library Use".to_owned()));
    source.allocations.custom_occupation_points.insert(0, 20);
    source.allocations.custom_personal_points.insert(0, 10);

    let json = source.export_json_save().expect("save should serialize");
    let mut value: serde_json::Value = serde_json::from_str(&json).expect("save should parse");
    value["occupation_id"] = serde_json::json!("Nurse");

    let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
    let mut loaded = test_app();
    let report = loaded
        .import_json_save(&edited_json)
        .expect("non-custom save should import after dropping hidden custom points");

    assert_eq!(loaded.occupation_id, "Nurse");
    assert!(loaded.allocations.custom_occupation_points.is_empty());
    assert!(loaded.allocations.custom_personal_points.is_empty());
    assert!(
        report
            .removed_allocations
            .iter()
            .any(|entry| entry.contains("custom skill slot 1")),
        "expected hidden custom allocations to be reported, got {report:?}"
    );
}

#[test]
fn inactive_duplicate_slot_labels_are_protected_when_editing_active_slot() {
    let mut app = test_app();
    app.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());
    app.set_custom_occupation_required_skill_count(2);
    assert!(app.set_custom_occupation_skill(0, "Language (Other)".to_owned()));
    assert!(app.set_custom_occupation_skill(1, "Language (Other)".to_owned()));
    let inactive_label = app
        .custom_occupation
        .skill_slot_labels
        .get(&1)
        .cloned()
        .expect("inactive duplicate should have a label before lowering count");

    app.set_custom_occupation_required_skill_count(1);
    app.sanitize_state();

    assert!(
        !app.set_custom_occupation_skill_label_for_slot(0, String::new()),
        "active duplicate label should not be clearable while an inactive same-skill slot exists"
    );
    assert!(
        !app.set_custom_occupation_skill_label_for_slot(0, inactive_label),
        "active duplicate label should not be reusable from an inactive same-skill slot"
    );

    app.set_custom_occupation_required_skill_count(2);
    app.sanitize_state();

    assert_eq!(app.custom_occupation.skills[0], "Language (Other)");
    assert_eq!(app.custom_occupation.skills[1], "Language (Other)");
    let first = app
        .custom_occupation
        .skill_slot_labels
        .get(&0)
        .expect("first duplicate should keep a label");
    let second = app
        .custom_occupation
        .skill_slot_labels
        .get(&1)
        .expect("second duplicate should keep a label");
    assert_ne!(first, second);
}

#[test]
fn lowering_then_raising_required_count_preserves_duplicate_custom_specialties() {
    let mut app = test_app();
    app.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());
    app.set_custom_occupation_required_skill_count(2);
    assert!(app.set_custom_occupation_skill(0, "Language (Other)".to_owned()));
    assert!(app.set_custom_occupation_skill(1, "Language (Other)".to_owned()));
    let first_label = app.custom_occupation.skill_slot_labels.get(&0).cloned();
    let second_label = app.custom_occupation.skill_slot_labels.get(&1).cloned();
    assert!(
        first_label
            .as_deref()
            .is_some_and(|label| !label.trim().is_empty())
    );
    assert!(
        second_label
            .as_deref()
            .is_some_and(|label| !label.trim().is_empty())
    );

    app.set_custom_occupation_required_skill_count(1);
    app.sanitize_state();
    assert_eq!(app.custom_occupation.skills[0], "Language (Other)");
    assert_eq!(app.custom_occupation.skills[1], "Language (Other)");

    app.set_custom_occupation_required_skill_count(2);
    app.sanitize_state();

    assert_eq!(app.custom_occupation.skills[0], "Language (Other)");
    assert_eq!(app.custom_occupation.skills[1], "Language (Other)");
    assert!(app.custom_occupation.skill_slot_labels.contains_key(&0));
    assert!(app.custom_occupation.skill_slot_labels.contains_key(&1));
    let rows: Vec<_> = app
        .sheet_math()
        .skill_rows
        .into_iter()
        .filter(|row| row.id == Skill::LanguageOther)
        .collect();
    assert_eq!(rows.len(), 2);
}

#[test]
fn json_import_reports_and_ignores_malformed_custom_allocation_keys() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");
    let mut value: serde_json::Value = serde_json::from_str(&json).expect("save should parse");
    value["allocations"]["custom_occupation_points"] = serde_json::json!({
        "-1": 10,
        "slot4": 20,
        "0": 30
    });
    value["allocations"]["custom_personal_points"] = serde_json::json!({
        "bad": 5
    });

    let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
    let mut loaded = test_app();
    let report = loaded
        .import_json_save(&edited_json)
        .expect("malformed custom allocation keys should be ignored, not fatal");

    assert!(loaded.allocations.custom_occupation_points.is_empty());
    assert!(loaded.allocations.custom_personal_points.is_empty());
    assert!(
        report
            .removed_unknown_import_entries
            .iter()
            .any(|entry| entry.contains("custom_occupation_points: -1")),
        "expected malformed -1 key report, got {report:?}"
    );
    assert!(
        report
            .removed_unknown_import_entries
            .iter()
            .any(|entry| entry.contains("custom_occupation_points: slot4")),
        "expected malformed slot4 key report, got {report:?}"
    );
    assert!(
        report
            .removed_unknown_import_entries
            .iter()
            .any(|entry| entry.contains("custom_personal_points: bad")),
        "expected malformed bad key report, got {report:?}"
    );
}

#[test]
fn json_import_reports_and_ignores_malformed_allocation_values() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");
    let mut value: serde_json::Value = serde_json::from_str(&json).expect("save should parse");
    value["allocations"]["occupation_points"] = serde_json::json!({
        "Library Use": "20",
        "Spot Hidden": 20
    });
    value["allocations"]["personal_points"] = serde_json::json!({
        "Listen": null
    });
    value["allocations"]["custom_occupation_points"] = serde_json::json!({
        "0": "30"
    });
    value["allocations"]["custom_personal_points"] = serde_json::json!({
        "1": []
    });

    let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
    let mut loaded = test_app();
    let report = loaded
        .import_json_save(&edited_json)
        .expect("malformed allocation values should be ignored, not fatal");

    assert!(
        !loaded
            .allocations
            .occupation_points
            .contains_key(&Skill::LibraryUse)
    );
    assert!(loaded.allocations.personal_points.is_empty());
    assert!(loaded.allocations.custom_occupation_points.is_empty());
    assert!(loaded.allocations.custom_personal_points.is_empty());
    for expected in [
        "occupation_points[Library Use]: non-integer allocation value",
        "personal_points[Listen]: non-integer allocation value",
        "custom_occupation_points[0]: non-integer allocation value",
        "custom_personal_points[1]: non-integer allocation value",
    ] {
        assert!(
            report
                .removed_unknown_import_entries
                .iter()
                .any(|entry| entry.contains(expected)),
            "expected {expected:?} report, got {report:?}"
        );
    }
}

#[test]
fn json_import_reports_and_ignores_malformed_custom_slot_label_keys() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");
    let mut value: serde_json::Value = serde_json::from_str(&json).expect("save should parse");
    value["custom_occupation"]["skill_slot_labels"] = serde_json::json!({
        "bad": "Language (Latin)",
        "-1": "Language (Greek)",
        "0": "Library Use"
    });

    let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
    let mut loaded = test_app();
    let report = loaded
        .import_json_save(&edited_json)
        .expect("malformed custom slot label keys should be ignored, not fatal");

    assert!(
        report
            .removed_unknown_import_entries
            .iter()
            .any(|entry| entry.contains("skill_slot_labels: bad")),
        "expected malformed bad label key report, got {report:?}"
    );
    assert!(
        report
            .removed_unknown_import_entries
            .iter()
            .any(|entry| entry.contains("skill_slot_labels: -1")),
        "expected malformed -1 label key report, got {report:?}"
    );
}

#[test]
fn json_import_reports_and_ignores_malformed_custom_slot_label_values() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");

    for (labels, expected) in [
        (
            serde_json::json!({
                "0": 123,
                "1": null,
                "2": "Library Use"
            }),
            "skill_slot_labels[0]: non-string label",
        ),
        (
            serde_json::json!(["not", "an", "object"]),
            "skill_slot_labels: expected object",
        ),
    ] {
        let mut value: serde_json::Value = serde_json::from_str(&json).expect("save should parse");
        value["custom_occupation"]["skill_slot_labels"] = labels;

        let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
        let mut loaded = test_app();
        let report = loaded
            .import_json_save(&edited_json)
            .expect("malformed custom slot label values should be ignored, not fatal");

        assert!(
            report
                .removed_unknown_import_entries
                .iter()
                .any(|entry| entry.contains(expected)),
            "expected {expected:?} report, got {report:?}"
        );
    }
}

#[test]
fn json_import_reports_and_ignores_malformed_legacy_custom_skill_labels() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");

    for (labels, expected) in [
        (
            serde_json::json!({
                "Pilot": 123,
                "Library Use": "Library (Archive)"
            }),
            "skill_labels[Pilot]: non-string label",
        ),
        (
            serde_json::json!(["not", "an", "object"]),
            "skill_labels: expected object",
        ),
    ] {
        let mut value: serde_json::Value = serde_json::from_str(&json).expect("save should parse");
        value["custom_occupation"]["skill_labels"] = labels;

        let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
        let mut loaded = test_app();
        let report = loaded
            .import_json_save(&edited_json)
            .expect("malformed legacy custom skill labels should be ignored, not fatal");

        assert!(
            report
                .removed_unknown_import_entries
                .iter()
                .any(|entry| entry.contains(expected)),
            "expected {expected:?} report, got {report:?}"
        );
    }
}

#[test]
fn custom_slot_labels_cannot_duplicate_visible_skill_rows() {
    let mut app = test_app();
    app.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());
    app.set_custom_occupation_required_skill_count(2);
    assert!(app.set_custom_occupation_skill(0, "Pilot".to_owned()));
    assert!(app.set_custom_occupation_skill(1, "Language (Other)".to_owned()));

    assert!(
        !app.set_custom_occupation_skill_label_for_slot(0, "Library Use".to_owned()),
        "custom labels should not duplicate standard visible rows"
    );
    assert!(
        !app.set_custom_occupation_skill_label_for_slot(0, "Language (Other)".to_owned()),
        "custom labels should not duplicate other custom rows"
    );
    assert!(app.set_custom_occupation_skill_label_for_slot(0, "Pilot (Boat)".to_owned()));
}

#[test]
fn sanitize_custom_occupation_removes_visible_label_collisions() {
    let mut app = test_app();
    app.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());
    app.set_custom_occupation_required_skill_count(2);
    assert!(app.set_custom_occupation_skill(0, "Pilot".to_owned()));
    assert!(app.set_custom_occupation_skill(1, "Language (Other)".to_owned()));
    app.custom_occupation
        .skill_slot_labels
        .insert(0, "Library Use".to_owned());
    app.custom_occupation
        .skill_slot_labels
        .insert(1, "Pilot".to_owned());

    app.sanitize_state();

    assert!(!app.custom_occupation.skill_slot_labels.contains_key(&0));
    assert!(!app.custom_occupation.skill_slot_labels.contains_key(&1));
    let row_names = app
        .sheet_math()
        .skill_rows
        .into_iter()
        .map(|row| row.name)
        .collect::<Vec<_>>();
    assert_eq!(
        row_names
            .iter()
            .filter(|name| name.as_str() == "Library Use")
            .count(),
        1
    );
    assert_eq!(
        row_names
            .iter()
            .filter(|name| name.as_str() == "Pilot")
            .count(),
        1
    );
}

#[test]
fn sanitize_custom_occupation_normalizes_legacy_label_keys_before_collision_cleanup() {
    let mut app = test_app();
    app.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());
    app.set_custom_occupation_required_skill_count(1);
    assert!(app.set_custom_occupation_skill(0, "Pilot".to_owned()));
    app.custom_occupation
        .skill_labels
        .insert(" Pilot ".to_owned(), "Library Use".to_owned());

    app.sanitize_state();

    assert!(app.custom_occupation.skill_labels.is_empty());
    let row_names = app
        .sheet_math()
        .skill_rows
        .into_iter()
        .map(|row| row.name)
        .collect::<Vec<_>>();
    assert_eq!(
        row_names
            .iter()
            .filter(|name| name.as_str() == "Library Use")
            .count(),
        1
    );
    assert_eq!(
        row_names
            .iter()
            .filter(|name| name.as_str() == "Pilot")
            .count(),
        1
    );
}

#[test]
fn lowering_custom_required_skill_count_preserves_inactive_slots() {
    let mut app = test_app();
    app.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());
    let skills = [
        "Accounting",
        "Anthropology",
        "Appraise",
        "Archaeology",
        "Charm",
        "Climb",
        "Disguise",
        "Dodge",
    ];
    for (index, skill) in skills.iter().enumerate() {
        assert!(app.set_custom_occupation_skill(index, (*skill).to_owned()));
    }

    app.set_custom_occupation_required_skill_count(3);
    app.sanitize_state();
    assert_eq!(app.custom_occupation.skills[4], "Charm");

    app.set_custom_occupation_required_skill_count(8);
    let occupation = app
        .selected_occupation()
        .expect("custom occupation should exist");
    assert_eq!(app.unique_occupation_shortfall_for(&occupation), 0);
    assert_eq!(app.resolved_occupation_skills_for(&occupation).len(), 8);
}

#[test]
fn json_import_rejects_present_but_malformed_save_versions() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");

    for version in [
        serde_json::json!("1"),
        serde_json::json!(null),
        serde_json::json!(-1),
    ] {
        let mut value: serde_json::Value =
            serde_json::from_str(&json).expect("exported save should parse");
        value["version"] = version;
        let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
        let mut loaded = test_app();
        let error = loaded
            .import_json_save(&edited_json)
            .expect_err("malformed version should be rejected");
        assert!(
            error.contains("save version must be a non-negative integer"),
            "unexpected error: {error}"
        );
    }
}

#[test]
fn rng_roll_history_restores_next_roll_position() {
    let mut source = test_app();
    let _ = source.roll_die(6);
    let _ = source.roll_die(100);
    let _ = source.roll_die(10);

    let json = source.export_json_save().expect("save should serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("save should parse");
    assert_eq!(value["rng_roll_sides"], serde_json::json!([6, 100, 10]));

    let mut loaded = test_app();
    loaded
        .import_json_save(&json)
        .expect("save should import cleanly");
    assert_eq!(loaded.rng_roll_sides, vec![6, 100, 10]);
    assert_eq!(loaded.roll_die(6), source.roll_die(6));
}

#[test]
fn live_rng_roll_history_is_bounded_and_rollover_preserves_next_roll() {
    let mut source = test_app();
    for _ in 0..(MAX_RNG_ROLL_HISTORY + 8) {
        let _ = source.roll_die(6);
    }

    assert_eq!(source.rng_roll_sides.len(), 8);
    let json = source.export_json_save().expect("save should serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("save should parse");
    assert_eq!(
        value["rng_roll_sides"]
            .as_array()
            .expect("rng_roll_sides should be an array")
            .len(),
        source.rng_roll_sides.len()
    );

    let mut loaded = test_app();
    loaded
        .import_json_save(&json)
        .expect("rolled-over RNG history should import cleanly");
    assert_eq!(loaded.rng_seed, source.rng_seed);
    assert_eq!(loaded.rng_roll_sides, source.rng_roll_sides);
    assert_eq!(loaded.roll_die(6), source.roll_die(6));
}

#[test]
fn nonstandard_die_sides_are_recorded_and_replayed() {
    let mut source = test_app();
    let _ = source.roll_die(12);
    let _ = source.roll_die(42);

    let json = source.export_json_save().expect("save should serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("save should parse");
    assert_eq!(value["rng_roll_sides"], serde_json::json!([12, 42]));

    let mut loaded = test_app();
    loaded
        .import_json_save(&json)
        .expect("save with nonstandard die sides should import cleanly");
    assert_eq!(loaded.rng_roll_sides, vec![12, 42]);
    assert_eq!(loaded.roll_die(6), source.roll_die(6));
}

#[test]
fn legacy_custom_skill_labels_are_truncated_during_sanitization() {
    let mut app = test_app();
    app.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());
    app.set_custom_occupation_required_skill_count(1);
    assert!(app.set_custom_occupation_skill(0, "Pilot".to_owned()));
    let label = "P".repeat(96);
    app.custom_occupation
        .skill_labels
        .insert("Pilot".to_owned(), label);

    app.sanitize_state();

    let saved_label = app
        .custom_occupation
        .skill_labels
        .get("Pilot")
        .expect("legacy custom label should be retained after normalization");
    assert_eq!(saved_label.chars().count(), 64);
    assert_eq!(
        app.custom_skill_display_name_for_slot(0, Skill::Pilot)
            .chars()
            .count(),
        64
    );
}

#[test]
fn import_report_includes_direct_load_time_corrections() {
    let app = test_app();
    let mut save = app.save_file();
    save.concept.age = 999;
    save.edu_bonus = 999;
    save.occupation_id = "Made Up Occupation".to_owned();

    let json = serde_json::to_string(&save).expect("test save should serialize");
    let mut loaded = test_app();
    let report = loaded
        .import_json_save(&json)
        .expect("save with direct load-time corrections should import");

    assert_eq!(loaded.concept.age, 89);
    assert_eq!(loaded.occupation_id, "");
    assert!(
        report
            .normalized_import_fields
            .iter()
            .any(|entry| entry.contains("age: 999 → 89")),
        "expected age normalization to be reported, got {report:?}"
    );
    assert!(
        report
            .normalized_import_fields
            .iter()
            .any(|entry| entry.contains("EDU bonus: 999 → 99")),
        "expected EDU bonus normalization to be reported, got {report:?}"
    );
    assert!(
        report
            .normalized_import_fields
            .iter()
            .any(|entry| entry.contains("occupation: Made Up Occupation")),
        "expected unknown occupation normalization to be reported, got {report:?}"
    );
    let summary = report.summary();
    assert!(
        summary.contains("age: 999 → 89"),
        "unexpected summary: {summary}"
    );
    assert!(
        summary.contains("EDU bonus: 999 → 99"),
        "unexpected summary: {summary}"
    );
    assert!(
        summary.contains("occupation: Made Up Occupation"),
        "unexpected summary: {summary}"
    );
}

#[test]
fn import_report_includes_non_allocation_corrections() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");
    let mut value: serde_json::Value =
        serde_json::from_str(&json).expect("exported save should parse");
    value["chars"]["STR"] = serde_json::json!(999);
    value["char_rolls"]["STR"] = serde_json::json!({
        "rolls": [1, 1, 1],
        "plus_six": false,
        "value": 15,
        "kept": null
    });
    value["luck_state"] = serde_json::json!({
        "value": 15,
        "rolls": []
    });
    value["backstory"]["Bogus"] = serde_json::json!("discard me");
    value["rng_seed"] = serde_json::json!(0);
    value["rng_roll_sides"] = serde_json::json!([6, 0, 10]);

    let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
    let mut loaded = test_app();
    let report = loaded
        .import_json_save(&edited_json)
        .expect("sanitized save should import");

    assert!(!report.clamped_characteristics.is_empty(), "{report:?}");
    assert!(
        !report.removed_characteristic_rolls.is_empty(),
        "{report:?}"
    );
    assert!(report.normalized_luck, "{report:?}");
    assert!(report.reset_luck, "{report:?}");
    assert!(
        !report.removed_backstory_categories.is_empty(),
        "{report:?}"
    );
    assert!(report.normalized_rng_state, "{report:?}");
}

#[test]
fn json_import_truncates_and_filters_malformed_rng_roll_history() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");
    let mut value: serde_json::Value = serde_json::from_str(&json).expect("save should parse");
    let mut roll_sides = vec![
        serde_json::json!(6),
        serde_json::json!(-1),
        serde_json::json!("6"),
        serde_json::json!(12),
        serde_json::json!(0),
        serde_json::json!(10),
    ];
    roll_sides.extend((0..MAX_RNG_ROLL_HISTORY).map(|_| serde_json::json!(100)));
    value["rng_roll_sides"] = serde_json::Value::Array(roll_sides);

    let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
    let mut loaded = test_app();
    let report = loaded
        .import_json_save(&edited_json)
        .expect("oversized RNG history should be normalized, not fatal");

    assert!(report.normalized_rng_state, "{report:?}");
    assert!(loaded.rng_roll_sides.len() <= MAX_RNG_ROLL_HISTORY);
    assert!(
        loaded.rng_roll_sides.iter().all(|sides| *sides > 0),
        "unexpected zero RNG side after normalization: {:?}",
        loaded.rng_roll_sides
    );
    assert_eq!(loaded.rng_roll_sides.first(), Some(&6));
    assert_eq!(loaded.rng_roll_sides.get(1), Some(&12));
    assert_eq!(loaded.rng_roll_sides.get(2), Some(&10));
}

#[test]
fn json_file_helpers_round_trip_save_data() {
    let mut source = test_app();
    source.concept.name = "File Round Trip".to_owned();
    let mut path = std::env::temp_dir();
    path.push(format!(
        "coc7e_investigator_creator_test_{}_round_trip.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    source
        .save_json_to_path(&path)
        .expect("save file should be written");
    let mut loaded = test_app();
    let report = loaded
        .load_json_from_path(&path)
        .expect("save file should be readable");
    let _ = std::fs::remove_file(&path);

    assert!(report.is_clean());
    assert_eq!(loaded.concept.name, "File Round Trip");
}

#[test]
fn json_file_helpers_overwrite_existing_save_with_complete_json() {
    let mut source = test_app();
    source.concept.name = "First Save".to_owned();
    let mut path = std::env::temp_dir();
    path.push(format!(
        "coc7e_investigator_creator_test_{}_overwrite.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    std::fs::write(&path, "not valid json").expect("test should create placeholder file");
    source
        .save_json_to_path(&path)
        .expect("save file should replace an existing file");

    let saved = std::fs::read_to_string(&path).expect("save file should be readable");
    let _ = std::fs::remove_file(&path);
    let value: serde_json::Value = serde_json::from_str(&saved)
        .expect("overwritten save should be complete JSON, not a partial write");
    assert_eq!(value["concept"]["name"], serde_json::json!("First Save"));
}

#[test]
fn rng_seed_is_saved_and_restored() {
    let source = test_app();
    let json = source.export_json_save().expect("save should serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("save should parse");

    assert_eq!(
        value["rng_seed"],
        serde_json::json!(0xC0C7_E7E5_0000_0001_u64)
    );

    let mut loaded = test_app();
    loaded
        .import_json_save(&json)
        .expect("save should import cleanly");
    assert_eq!(loaded.rng_seed, source.rng_seed);
}

#[test]
fn legacy_custom_skill_labels_cannot_duplicate_visible_skill_rows_after_import() {
    let mut app = test_app();
    app.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());
    app.set_custom_occupation_required_skill_count(1);
    assert!(app.set_custom_occupation_skill(0, "Pilot".to_owned()));
    app.custom_occupation
        .skill_labels
        .insert("Pilot".to_owned(), "Library Use".to_owned());

    app.sanitize_state();

    assert!(!app.custom_occupation.skill_labels.contains_key("Pilot"));
    let row_names = app
        .sheet_math()
        .skill_rows
        .into_iter()
        .map(|row| row.name)
        .collect::<Vec<_>>();
    assert_eq!(
        row_names
            .iter()
            .filter(|name| name.as_str() == "Library Use")
            .count(),
        1
    );
    assert_eq!(
        row_names
            .iter()
            .filter(|name| name.as_str() == "Pilot")
            .count(),
        1
    );
}

#[test]
fn setting_custom_skill_sanitizes_latent_legacy_label_collisions() {
    let mut app = test_app();
    app.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());
    app.set_custom_occupation_required_skill_count(2);
    app.custom_occupation
        .skill_labels
        .insert("Pilot".to_owned(), "Library Use".to_owned());

    assert!(app.set_custom_occupation_skill(1, "Pilot".to_owned()));

    assert!(!app.custom_occupation.skill_labels.contains_key("Pilot"));
    let row_names = app
        .sheet_math()
        .skill_rows
        .into_iter()
        .map(|row| row.name)
        .collect::<Vec<_>>();
    assert_eq!(
        row_names
            .iter()
            .filter(|name| name.as_str() == "Library Use")
            .count(),
        1
    );
    assert_eq!(
        row_names
            .iter()
            .filter(|name| name.as_str() == "Pilot")
            .count(),
        1
    );
}

#[test]
fn summary_reports_impossible_custom_occupation_budget_when_caps_are_too_low() {
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
            ("EDU", 80),
        ],
    );
    app.set_occupation(CUSTOM_OCCUPATION_ID.to_owned());
    app.set_custom_occupation_required_skill_count(1);
    assert!(app.set_custom_occupation_skill(0, "Library Use".to_owned()));

    let math = app.sheet_math();
    let occupation_capacity = CoC7eApp::occupation_budget_capacity_from(&math);
    assert!(occupation_capacity < math.occupation_budget);

    let blockers = app.summary_blockers_for(&math);
    assert!(
        blockers
            .iter()
            .any(|blocker| blocker.starts_with("occupation points impossible:")),
        "expected impossible occupation budget blocker, got {blockers:?}"
    );
}

#[test]
fn json_import_tolerates_malformed_custom_occupation_and_choice_field_types() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");
    let mut value: serde_json::Value = serde_json::from_str(&json).expect("save should parse");
    value["occupation_id"] = serde_json::json!("Nurse");
    value["custom_occupation"]["required_skill_count"] = serde_json::json!(-1);
    value["custom_occupation"]["skills"] = serde_json::json!([123, "Library Use"]);
    value["occupation_choices"] = serde_json::json!([
        {
            "id": "nurse-interpersonal",
            "index": "0",
            "value": "Persuade"
        },
        {
            "id": 123,
            "index": -1,
            "value": false
        },
        false
    ]);

    let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
    let mut loaded = test_app();
    let report = loaded
        .import_json_save(&edited_json)
        .expect("malformed custom occupation and choice field types should not be fatal");

    assert_eq!(loaded.custom_occupation.required_skill_count, 1);
    assert_eq!(loaded.custom_occupation.skills[0], "");
    assert_eq!(loaded.custom_occupation.skills[1], "Library Use");
    assert_eq!(
        loaded
            .occupation_choices
            .get(&ChoiceKey::new("nurse-interpersonal", 0)),
        Some(&"Persuade".to_owned())
    );
    for expected in [
        "custom_occupation.required_skill_count: expected non-negative integer",
        "custom_occupation.skills[0]: non-string skill",
        "occupation_choices[1].id: expected string",
        "occupation_choices[1].index: expected non-negative integer",
        "occupation_choices[1].value: expected string",
        "occupation_choices[2]: expected object",
    ] {
        assert!(
            report
                .removed_unknown_import_entries
                .iter()
                .any(|entry| entry.contains(expected)),
            "expected {expected:?} report, got {report:?}"
        );
    }
}

#[test]
fn json_import_tolerates_repairable_top_level_field_types() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");
    let mut value: serde_json::Value = serde_json::from_str(&json).expect("save should parse");

    value["concept"]["name"] = serde_json::json!(123);
    value["concept"]["age"] = serde_json::json!("40");
    value["char_method"] = serde_json::json!("not-a-method");
    value["chars"]["STR"] = serde_json::json!("52");
    value["chars"]["CON"] = serde_json::json!(false);
    value["age_deductions"]["STR"] = serde_json::json!("5");
    value["char_rolls"]["STR"] = serde_json::json!({
        "rolls": [1, 1, false],
        "plus_six": false,
        "value": 15,
        "kept": null
    });
    value["rng_seed"] = serde_json::json!("12345");
    value["rng_roll_sides"] = serde_json::json!([6, "6", 0, -1, 10]);
    value["luck_state"] = serde_json::json!({
        "value": "15",
        "rolls": [false]
    });
    value["edu_bonus"] = serde_json::json!("999");
    value["edu_check_rolls"] = serde_json::json!([
        {
            "d100": "100",
            "improved": "true",
            "gain": "7",
            "resulting_edu": "99"
        },
        false
    ]);
    value["occupation_id"] = serde_json::json!(123);
    value["formula_key"] = serde_json::json!("not-a-formula");
    value["allocations"] = serde_json::json!(false);
    value["backstory"]["Traits"] = serde_json::json!(123);

    let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
    let mut loaded = test_app();
    let report = loaded
        .import_json_save(&edited_json)
        .expect("repairable top-level field type errors should not be fatal");

    assert_eq!(loaded.concept.name, "");
    assert_eq!(loaded.concept.age, 40);
    assert_eq!(loaded.char_method, CharMethod::Roll);
    assert_eq!(loaded.chars.get_char(Characteristic::Str), 50);
    assert_eq!(loaded.chars.get_char(Characteristic::Con), 0);
    assert!(loaded.char_rolls.is_empty());
    assert_eq!(loaded.rng_seed, 12345);
    assert_eq!(loaded.rng_roll_sides, vec![6, 10]);
    assert_eq!(loaded.luck_state.value, None);
    assert!(loaded.luck_state.rolls.is_empty());
    assert_eq!(loaded.edu_bonus, 0);
    assert!(loaded.edu_check_rolls.is_empty());
    assert_eq!(loaded.occupation_id, "");
    assert_eq!(loaded.formula_key, FormulaKey::Edu4);
    assert!(loaded.allocations.occupation_points.is_empty());
    assert!(!loaded.backstory.contains_key("Traits"));

    for expected in [
        "concept.name: expected string",
        "char_method: unknown method",
        "chars.CON: expected integer",
        "char_rolls[STR]: malformed roll evidence",
        "rng_roll_sides[1]: expected positive integer",
        "rng_roll_sides[2]: expected positive integer",
        "rng_roll_sides[3]: expected positive integer",
        "luck_state.rolls[0]: malformed roll evidence",
        "edu_check_rolls[1]: expected object",
        "occupation_id: expected string",
        "formula_key: unknown formula",
        "allocations: expected object",
        "backstory[Traits]: expected string",
    ] {
        assert!(
            report
                .removed_unknown_import_entries
                .iter()
                .any(|entry| entry.contains(expected)),
            "expected {expected:?} report, got {report:?}"
        );
    }
}

#[test]
fn summary_missing_occupation_does_not_report_occupation_points() {
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
            ("EDU", 80),
        ],
    );

    let math = app.sheet_math();
    let blockers = app.summary_blockers_for(&math);

    assert!(
        blockers
            .iter()
            .any(|blocker| blocker == "occupation missing")
    );
    assert!(
        !blockers
            .iter()
            .any(|blocker| blocker.starts_with("occupation points")),
        "occupation point blocker should wait for an occupation, got {blockers:?}"
    );
}

#[test]
fn json_file_helpers_reject_unexpanded_home_and_bad_paths() {
    let app = test_app();
    let home_error = app
        .save_json_to_path(std::path::Path::new("~/investigator.json"))
        .expect_err("unexpanded home paths should be rejected explicitly");
    assert!(home_error.contains("~ is not expanded"));

    let relative_tilde_file_name = format!("~draft_{}.json", std::process::id());
    let relative_tilde_path = std::path::Path::new(&relative_tilde_file_name);
    let _ = std::fs::remove_file(relative_tilde_path);
    app.save_json_to_path(relative_tilde_path)
        .expect("relative filenames that merely start with ~ should still be valid");
    let _ = std::fs::remove_file(relative_tilde_path);

    let mut missing_parent = std::env::temp_dir();
    missing_parent.push(format!("coc7e_missing_parent_{}_save", std::process::id()));
    let _ = std::fs::remove_dir_all(&missing_parent);
    let mut missing_parent_file = missing_parent.clone();
    missing_parent_file.push("investigator.json");
    let missing_parent_error = app
        .save_json_to_path(&missing_parent_file)
        .expect_err("saves should explain missing parent directories");
    assert!(missing_parent_error.contains("save directory does not exist"));

    let mut missing_file = std::env::temp_dir();
    missing_file.push(format!(
        "coc7e_missing_file_{}_load.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&missing_file);
    let mut loaded = test_app();
    let missing_file_error = loaded
        .load_json_from_path(&missing_file)
        .expect_err("loads should explain missing files before reading");
    assert!(missing_file_error.contains("save file does not exist"));
}

#[test]
fn json_import_drops_edu_check_rolls_without_valid_d100() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");
    let mut value: serde_json::Value = serde_json::from_str(&json).expect("save should parse");

    value["concept"]["age"] = serde_json::json!(40);
    value["chars"]["EDU"] = serde_json::json!(40);
    value["edu_check_rolls"] = serde_json::json!([
        {},
        {
            "gain": 7,
            "resulting_edu": 47
        },
        {
            "d100": false,
            "gain": 7,
            "resulting_edu": 47
        },
        {
            "d100": "100"
        },
        {
            "d100": 200,
            "gain": 7
        },
        {
            "d100": "100",
            "gain": 0
        },
        {
            "d100": 100,
            "gain": 11
        },
        {
            "d100": "100",
            "improved": false,
            "gain": "7",
            "resulting_edu": 99
        }
    ]);

    let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
    let mut loaded = test_app();
    let report = loaded
        .import_json_save(&edited_json)
        .expect("missing EDU check evidence should be reported and dropped");

    assert_eq!(loaded.edu_check_rolls.len(), 1);
    assert_eq!(loaded.edu_check_rolls[0].d100, 100);
    assert!(loaded.edu_check_rolls[0].improved);
    assert_eq!(loaded.edu_check_rolls[0].gain, 7);
    assert_eq!(loaded.edu_check_rolls[0].resulting_edu, 47);

    for expected in [
        "edu_check_rolls[0]: missing required d100",
        "edu_check_rolls[1]: missing required d100",
        "edu_check_rolls[2].d100: expected integer",
        "edu_check_rolls[3]: missing required gain",
        "edu_check_rolls[4].d100: expected 1..=100",
        "gain: expected 1..=10 for improved check",
        "edu_check_rolls[6].gain: expected 0..=10",
    ] {
        assert!(
            report
                .removed_unknown_import_entries
                .iter()
                .any(|entry| entry.contains(expected)),
            "expected {expected:?} report, got {report:?}"
        );
    }
}

#[test]
fn json_import_reports_wrong_length_legacy_characteristic_arrays() {
    let app = test_app();
    let json = app.export_json_save().expect("save should serialize");
    let mut value: serde_json::Value = serde_json::from_str(&json).expect("save should parse");

    value["chars"] = serde_json::json!([50, 55]);
    value["age_deductions"] = serde_json::json!({
        "values": [0, 5]
    });

    let edited_json = serde_json::to_string(&value).expect("edited save should serialize");
    let mut loaded = test_app();
    let report = loaded
        .import_json_save(&edited_json)
        .expect("wrong-length legacy arrays should still import tolerantly");

    assert_eq!(loaded.chars.get_char(Characteristic::Str), 50);
    assert_eq!(loaded.chars.get_char(Characteristic::Con), 55);
    assert_eq!(loaded.chars.get_char(Characteristic::Siz), 0);

    for expected in [
        "chars: expected 8 ordered values, got 2",
        "age_deductions.values: expected 8 ordered values, got 2",
    ] {
        assert!(
            report
                .removed_unknown_import_entries
                .iter()
                .any(|entry| entry.contains(expected)),
            "expected {expected:?} report, got {report:?}"
        );
    }
}
