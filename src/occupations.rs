use super::data::*;
use super::ruleset::*;
use std::collections::HashSet;

pub(crate) const ALL_SKILL_NAMES: &[&str] = &[
    "Accounting",
    "Anthropology",
    "Appraise",
    "Archaeology",
    "Art/Craft",
    "Art/Craft (Acting)",
    "Art/Craft (Fine Art)",
    "Art/Craft (Forgery)",
    "Art/Craft (Literature)",
    "Art/Craft (Photography)",
    "Art/Craft (Technical Drawing)",
    "Art/Craft (Writing)",
    "Charm",
    "Climb",
    "Credit Rating",
    "Cthulhu Mythos",
    "Disguise",
    "Dodge",
    "Drive Auto",
    "Electrical Repair",
    "Fast Talk",
    "Fighting (Brawl)",
    "Firearms (Handgun)",
    "Firearms (Rifle/Shotgun)",
    "First Aid",
    "History",
    "Intimidate",
    "Jump",
    "Language (Other)",
    "Language (Own)",
    "Law",
    "Library Use",
    "Listen",
    "Locksmith",
    "Mechanical Repair",
    "Medicine",
    "Natural World",
    "Navigate",
    "Occult",
    "Operate Heavy Machinery",
    "Persuade",
    "Pilot",
    "Psychoanalysis",
    "Psychology",
    "Ride",
    "Science (Astronomy)",
    "Science (Biology)",
    "Science (Botany)",
    "Science (Chemistry)",
    "Science (Cryptography)",
    "Science (Engineering)",
    "Science (Forensics)",
    "Science (Geology)",
    "Science (Mathematics)",
    "Science (Pharmacy)",
    "Science (Physics)",
    "Science (Zoology)",
    "Sleight of Hand",
    "Spot Hidden",
    "Stealth",
    "Survival",
    "Swim",
    "Throw",
    "Track",
];

pub(crate) const OCCUPATION_SELECTABLE_SKILLS: &[&str] = &[
    "Accounting",
    "Anthropology",
    "Appraise",
    "Archaeology",
    "Art/Craft",
    "Art/Craft (Acting)",
    "Art/Craft (Fine Art)",
    "Art/Craft (Forgery)",
    "Art/Craft (Literature)",
    "Art/Craft (Photography)",
    "Art/Craft (Technical Drawing)",
    "Art/Craft (Writing)",
    "Charm",
    "Climb",
    "Disguise",
    "Dodge",
    "Drive Auto",
    "Electrical Repair",
    "Fast Talk",
    "Fighting (Brawl)",
    "Firearms (Handgun)",
    "Firearms (Rifle/Shotgun)",
    "First Aid",
    "History",
    "Intimidate",
    "Jump",
    "Language (Other)",
    "Language (Own)",
    "Law",
    "Library Use",
    "Listen",
    "Locksmith",
    "Mechanical Repair",
    "Medicine",
    "Natural World",
    "Navigate",
    "Occult",
    "Operate Heavy Machinery",
    "Persuade",
    "Pilot",
    "Psychoanalysis",
    "Psychology",
    "Ride",
    "Science (Astronomy)",
    "Science (Biology)",
    "Science (Botany)",
    "Science (Chemistry)",
    "Science (Cryptography)",
    "Science (Engineering)",
    "Science (Forensics)",
    "Science (Geology)",
    "Science (Mathematics)",
    "Science (Pharmacy)",
    "Science (Physics)",
    "Science (Zoology)",
    "Sleight of Hand",
    "Spot Hidden",
    "Stealth",
    "Survival",
    "Swim",
    "Throw",
    "Track",
];

pub(crate) const ART_SKILLS: &[&str] = &[
    "Art/Craft",
    "Art/Craft (Acting)",
    "Art/Craft (Fine Art)",
    "Art/Craft (Forgery)",
    "Art/Craft (Literature)",
    "Art/Craft (Photography)",
    "Art/Craft (Technical Drawing)",
    "Art/Craft (Writing)",
];

pub(crate) const SCIENCE_SKILLS: &[&str] = &[
    "Science (Astronomy)",
    "Science (Biology)",
    "Science (Botany)",
    "Science (Chemistry)",
    "Science (Cryptography)",
    "Science (Engineering)",
    "Science (Forensics)",
    "Science (Geology)",
    "Science (Mathematics)",
    "Science (Pharmacy)",
    "Science (Physics)",
    "Science (Zoology)",
];

pub(crate) const INTERPERSONAL_SKILLS: &[&str] = &["Charm", "Fast Talk", "Intimidate", "Persuade"];
pub(crate) const FIREARMS_SKILLS: &[&str] = &["Firearms (Handgun)", "Firearms (Rifle/Shotgun)"];

pub(crate) fn skill_options(options: &[&str]) -> Vec<String> {
    options.iter().map(|skill| (*skill).to_owned()).collect()
}

pub(crate) fn occupation_selectable_skills() -> &'static [&'static str] {
    OCCUPATION_SELECTABLE_SKILLS
}

pub(crate) fn art_skills() -> &'static [&'static str] {
    ART_SKILLS
}

pub(crate) fn science_skills() -> &'static [&'static str] {
    SCIENCE_SKILLS
}

pub(crate) fn interpersonal_skills() -> &'static [&'static str] {
    INTERPERSONAL_SKILLS
}

pub(crate) fn firearms_skills() -> &'static [&'static str] {
    FIREARMS_SKILLS
}

pub(crate) fn fixed(name: &str) -> Slot {
    Slot::Skill(name.to_owned())
}

pub(crate) fn choice(id: &str, label: &str, options: Vec<String>, count: usize) -> Slot {
    Slot::Choice {
        id: id.to_owned(),
        label: label.to_owned(),
        options,
        count,
    }
}

pub(crate) fn any_skill(id: &str, count: usize, label: &str) -> Slot {
    choice(
        id,
        label,
        skill_options(occupation_selectable_skills()),
        count,
    )
}
pub(crate) fn interpersonal(id: &str, count: usize) -> Slot {
    choice(
        id,
        "Interpersonal skill",
        skill_options(interpersonal_skills()),
        count,
    )
}

pub(crate) fn occupation(
    name: &str,
    credit: (i32, i32),
    formula_keys: Vec<FormulaKey>,
    slots: Vec<Slot>,
) -> Occupation {
    Occupation {
        name: name.to_owned(),
        credit,
        formula_keys,
        slots,
    }
}

pub(crate) fn fixed_skill_set_for(occupation: &Occupation) -> HashSet<&str> {
    occupation
        .slots
        .iter()
        .filter_map(|slot| match slot {
            Slot::Skill(name) => Some(name.as_str()),
            Slot::Choice { .. } => None,
        })
        .collect()
}

// Simple augmenting-path bipartite matching: each required choice pick must be
// assigned to one unique, non-fixed skill option.
pub(crate) fn choice_pools_have_full_matching(pools: &[Vec<&str>]) -> bool {
    fn option_index(option_names: &[&str], target: &str) -> usize {
        option_names
            .iter()
            .position(|candidate| *candidate == target)
            .expect("choice option should have been collected")
    }

    fn assign_pick(
        pick_index: usize,
        pools: &[Vec<&str>],
        option_names: &[&str],
        option_to_pick: &mut [Option<usize>],
        seen_options: &mut [bool],
    ) -> bool {
        for option in &pools[pick_index] {
            let option_idx = option_index(option_names, option);

            if seen_options[option_idx] {
                continue;
            }
            seen_options[option_idx] = true;

            if option_to_pick[option_idx].is_none()
                || assign_pick(
                    option_to_pick[option_idx].unwrap(),
                    pools,
                    option_names,
                    option_to_pick,
                    seen_options,
                )
            {
                option_to_pick[option_idx] = Some(pick_index);
                return true;
            }
        }

        false
    }

    let mut seen = HashSet::new();
    let mut option_names = Vec::new();
    for pool in pools {
        for option in pool {
            if seen.insert(*option) {
                option_names.push(*option);
            }
        }
    }

    let mut option_to_pick = vec![None; option_names.len()];

    for pick_index in 0..pools.len() {
        let mut seen_options = vec![false; option_names.len()];
        if !assign_pick(
            pick_index,
            pools,
            &option_names,
            &mut option_to_pick,
            &mut seen_options,
        ) {
            return false;
        }
    }

    true
}

pub(crate) fn choice_value_is_valid(options: &[String], value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && options.iter().any(|option| option == value)
}

pub(crate) fn validate_occupations(occupations: &[Occupation]) {
    let known: HashSet<&str> = SKILL_SPECS.iter().map(|skill| skill.name).collect();
    let mut occupation_names = HashSet::new();

    for occupation in occupations {
        assert!(
            !occupation.name.trim().is_empty(),
            "occupation with empty name"
        );
        assert!(
            occupation_names.insert(occupation.name.as_str()),
            "duplicate occupation name `{}`",
            occupation.name
        );
        assert!(
            !occupation.formula_keys.is_empty(),
            "occupation `{}` has no skill formula",
            occupation.name
        );
        let mut formula_set = HashSet::new();
        for formula in &occupation.formula_keys {
            assert!(
                formula_set.insert(*formula),
                "occupation `{}` has duplicate formula `{}`",
                occupation.name,
                formula.label()
            );
        }
        assert!(
            occupation.credit.0 <= occupation.credit.1,
            "occupation `{}` has inverted Credit Rating range",
            occupation.name
        );
        assert!(
            (0..=99).contains(&occupation.credit.0) && (0..=99).contains(&occupation.credit.1),
            "occupation `{}` has out-of-range Credit Rating",
            occupation.name
        );

        let slot_count: usize = occupation
            .slots
            .iter()
            .map(|slot| match slot {
                Slot::Skill(_) => 1,
                Slot::Choice { count, .. } => *count,
            })
            .sum();
        assert_eq!(
            slot_count, 8,
            "occupation `{}` should resolve to exactly 8 occupation skill slots",
            occupation.name
        );

        let fixed_names_all: HashSet<&str> = occupation
            .slots
            .iter()
            .filter_map(|slot| match slot {
                Slot::Skill(name) => Some(name.as_str()),
                Slot::Choice { .. } => None,
            })
            .collect();

        let mut choice_ids = HashSet::new();
        let mut fixed_names = HashSet::new();
        let mut choice_pools: Vec<Vec<&str>> = Vec::new();

        for slot in &occupation.slots {
            match slot {
                Slot::Skill(name) => {
                    assert!(
                        known.contains(name.as_str()),
                        "unknown fixed skill `{name}` in occupation `{}`",
                        occupation.name
                    );
                    assert_ne!(
                        name.as_str(),
                        "Credit Rating",
                        "occupation `{}` should not list Credit Rating as a fixed slot",
                        occupation.name
                    );
                    assert_ne!(
                        name.as_str(),
                        "Cthulhu Mythos",
                        "occupation `{}` should not grant Cthulhu Mythos at creation",
                        occupation.name
                    );
                    assert!(
                        fixed_names.insert(name.as_str()),
                        "duplicate fixed skill `{name}` in occupation `{}`",
                        occupation.name
                    );
                }
                Slot::Choice {
                    id,
                    label,
                    options,
                    count,
                } => {
                    assert!(
                        !id.trim().is_empty(),
                        "occupation `{}` has a choice with an empty id",
                        occupation.name
                    );
                    assert!(
                        choice_ids.insert(id.as_str()),
                        "duplicate choice id `{id}` in occupation `{}`",
                        occupation.name
                    );
                    assert!(
                        !label.trim().is_empty(),
                        "choice `{id}` in occupation `{}` has an empty label",
                        occupation.name
                    );
                    assert!(
                        *count > 0,
                        "choice `{id}` in occupation `{}` has zero count",
                        occupation.name
                    );
                    assert!(
                        options.len() >= *count,
                        "choice `{id}` in occupation `{}` asks for more picks than it offers",
                        occupation.name
                    );

                    let mut seen = HashSet::new();
                    for option in options {
                        assert!(
                            known.contains(option.as_str()),
                            "unknown choice skill `{option}` in occupation `{}`",
                            occupation.name
                        );
                        assert_ne!(
                            option.as_str(),
                            "Credit Rating",
                            "choice `{id}` in occupation `{}` should not include Credit Rating",
                            occupation.name
                        );
                        assert_ne!(
                            option.as_str(),
                            "Cthulhu Mythos",
                            "choice `{id}` in occupation `{}` should not include Cthulhu Mythos",
                            occupation.name
                        );
                        assert!(
                            seen.insert(option.as_str()),
                            "duplicate option `{option}` in choice `{id}` for occupation `{}`",
                            occupation.name
                        );
                    }

                    let usable_options: Vec<&str> = options
                        .iter()
                        .map(|option| option.as_str())
                        .filter(|option| !fixed_names_all.contains(*option))
                        .collect();
                    assert!(
                        usable_options.len() >= *count,
                        "choice `{id}` in occupation `{}` has only {} selectable non-fixed option(s) for {count} required pick(s)",
                        occupation.name,
                        usable_options.len()
                    );

                    for _ in 0..*count {
                        choice_pools.push(usable_options.clone());
                    }
                }
            }
        }

        assert!(
            choice_pools_have_full_matching(&choice_pools),
            "occupation `{}` has choice slots that cannot be resolved to unique non-fixed skills",
            occupation.name
        );
    }
}

pub(crate) fn build_occupations() -> Vec<Occupation> {
    vec![
        occupation(
            "Antiquarian",
            (30, 70),
            vec![FormulaKey::Edu4],
            vec![
                fixed("Appraise"),
                choice(
                    "antiquarian-art",
                    "Art/Craft specialty",
                    skill_options(art_skills()),
                    1,
                ),
                fixed("History"),
                fixed("Library Use"),
                fixed("Language (Other)"),
                interpersonal("antiquarian-interpersonal", 1),
                fixed("Spot Hidden"),
                any_skill("antiquarian-any", 1, "Any occupation skill"),
            ],
        ),
        occupation(
            "Archaeologist",
            (10, 40),
            vec![FormulaKey::Edu4],
            vec![
                fixed("Appraise"),
                fixed("Archaeology"),
                fixed("History"),
                fixed("Language (Other)"),
                fixed("Library Use"),
                fixed("Spot Hidden"),
                fixed("Mechanical Repair"),
                choice(
                    "archaeologist-nav-science",
                    "Navigate or Science",
                    [vec!["Navigate".to_owned()], skill_options(science_skills())].concat(),
                    1,
                ),
            ],
        ),
        occupation(
            "Artist",
            (9, 50),
            vec![FormulaKey::Edu2Dex2, FormulaKey::Edu2App2],
            vec![
                choice(
                    "artist-art",
                    "Art/Craft specialty",
                    skill_options(art_skills()),
                    1,
                ),
                fixed("History"),
                fixed("Natural World"),
                fixed("Language (Other)"),
                fixed("Psychology"),
                fixed("Spot Hidden"),
                interpersonal("artist-interpersonal", 1),
                any_skill("artist-any", 1, "Any occupation skill"),
            ],
        ),
        occupation(
            "Author",
            (9, 30),
            vec![FormulaKey::Edu4],
            vec![
                fixed("Art/Craft (Writing)"),
                fixed("History"),
                fixed("Library Use"),
                choice(
                    "author-nature-occult",
                    "Natural World or Occult",
                    vec!["Natural World".to_owned(), "Occult".to_owned()],
                    1,
                ),
                fixed("Language (Other)"),
                fixed("Language (Own)"),
                fixed("Psychology"),
                any_skill("author-any", 1, "Any occupation skill"),
            ],
        ),
        occupation(
            "Cat Burglar",
            (5, 40),
            vec![FormulaKey::Edu2Dex2],
            vec![
                fixed("Appraise"),
                fixed("Climb"),
                choice(
                    "burglar-repair",
                    "Electrical or Mechanical Repair",
                    vec![
                        "Electrical Repair".to_owned(),
                        "Mechanical Repair".to_owned(),
                    ],
                    1,
                ),
                fixed("Listen"),
                fixed("Locksmith"),
                fixed("Sleight of Hand"),
                fixed("Stealth"),
                fixed("Spot Hidden"),
            ],
        ),
        occupation(
            "Clergy",
            (9, 60),
            vec![FormulaKey::Edu4],
            vec![
                fixed("Accounting"),
                fixed("History"),
                fixed("Library Use"),
                fixed("Listen"),
                fixed("Language (Other)"),
                fixed("Psychology"),
                fixed("Persuade"),
                any_skill("clergy-any", 1, "Any occupation skill"),
            ],
        ),
        occupation(
            "Criminal",
            (5, 65),
            vec![FormulaKey::Edu2Dex2, FormulaKey::Edu2Str2],
            vec![
                choice(
                    "criminal-art",
                    "Art/Craft specialty",
                    skill_options(art_skills()),
                    1,
                ),
                fixed("Disguise"),
                fixed("Fighting (Brawl)"),
                choice(
                    "criminal-firearms",
                    "Firearms specialty",
                    skill_options(firearms_skills()),
                    1,
                ),
                fixed("Locksmith"),
                fixed("Mechanical Repair"),
                fixed("Sleight of Hand"),
                fixed("Stealth"),
            ],
        ),
        occupation(
            "Dilettante",
            (50, 99),
            vec![FormulaKey::Edu2App2],
            vec![
                choice(
                    "dilettante-art",
                    "Art/Craft specialty",
                    skill_options(art_skills()),
                    1,
                ),
                choice(
                    "dilettante-firearms",
                    "Firearms specialty",
                    skill_options(firearms_skills()),
                    1,
                ),
                fixed("Language (Other)"),
                fixed("Ride"),
                interpersonal("dilettante-interpersonal", 1),
                any_skill("dilettante-any", 3, "Any three occupation skills"),
            ],
        ),
        occupation(
            "Doctor of Medicine",
            (30, 80),
            vec![FormulaKey::Edu4],
            vec![
                fixed("First Aid"),
                fixed("Medicine"),
                fixed("Language (Other)"),
                fixed("Psychology"),
                fixed("Science (Biology)"),
                fixed("Science (Pharmacy)"),
                any_skill("doctor-any", 2, "Any two academic/personal specialties"),
            ],
        ),
        occupation(
            "Drifter",
            (0, 5),
            vec![
                FormulaKey::Edu2App2,
                FormulaKey::Edu2Dex2,
                FormulaKey::Edu2Str2,
            ],
            vec![
                fixed("Climb"),
                fixed("Jump"),
                fixed("Listen"),
                fixed("Navigate"),
                interpersonal("drifter-interpersonal", 1),
                fixed("Stealth"),
                any_skill("drifter-any", 2, "Any two occupation skills"),
            ],
        ),
        occupation(
            "Engineer",
            (30, 60),
            vec![FormulaKey::Edu4],
            vec![
                fixed("Art/Craft (Technical Drawing)"),
                fixed("Electrical Repair"),
                fixed("Library Use"),
                fixed("Mechanical Repair"),
                fixed("Operate Heavy Machinery"),
                fixed("Science (Engineering)"),
                fixed("Science (Physics)"),
                any_skill("engineer-any", 1, "Any occupation skill"),
            ],
        ),
        occupation(
            "Entertainer",
            (9, 70),
            vec![FormulaKey::Edu2App2, FormulaKey::Edu2Dex2],
            vec![
                choice(
                    "entertainer-art",
                    "Art/Craft specialty",
                    skill_options(art_skills()),
                    1,
                ),
                fixed("Disguise"),
                fixed("Listen"),
                fixed("Psychology"),
                interpersonal("entertainer-interpersonal", 1),
                any_skill("entertainer-any", 3, "Any three occupation skills"),
            ],
        ),
        occupation(
            "Explorer",
            (55, 80),
            vec![
                FormulaKey::Edu2App2,
                FormulaKey::Edu2Dex2,
                FormulaKey::Edu2Str2,
            ],
            vec![
                choice(
                    "explorer-climb-swim",
                    "Climb or Swim",
                    vec!["Climb".to_owned(), "Swim".to_owned()],
                    1,
                ),
                choice(
                    "explorer-firearms",
                    "Firearms specialty",
                    skill_options(firearms_skills()),
                    1,
                ),
                fixed("History"),
                fixed("Jump"),
                fixed("Natural World"),
                fixed("Navigate"),
                fixed("Language (Other)"),
                fixed("Survival"),
            ],
        ),
        occupation(
            "Journalist",
            (9, 30),
            vec![FormulaKey::Edu4],
            vec![
                fixed("Art/Craft (Photography)"),
                fixed("History"),
                fixed("Library Use"),
                fixed("Language (Own)"),
                interpersonal("journalist-interpersonal", 1),
                fixed("Psychology"),
                any_skill("journalist-any", 2, "Any two occupation skills"),
            ],
        ),
        occupation(
            "Lawyer",
            (30, 80),
            vec![FormulaKey::Edu4],
            vec![
                fixed("Accounting"),
                fixed("Law"),
                fixed("Library Use"),
                fixed("Persuade"),
                fixed("Psychology"),
                any_skill("lawyer-any", 3, "Any three occupation skills"),
            ],
        ),
        occupation(
            "Librarian",
            (9, 35),
            vec![FormulaKey::Edu4],
            vec![
                fixed("Accounting"),
                fixed("Library Use"),
                fixed("Language (Other)"),
                fixed("Language (Own)"),
                any_skill("librarian-any", 4, "Any four occupation skills"),
            ],
        ),
        occupation(
            "Military Officer",
            (20, 70),
            vec![FormulaKey::Edu2Dex2, FormulaKey::Edu2Str2],
            vec![
                fixed("Accounting"),
                choice(
                    "military-firearms",
                    "Firearms specialty",
                    skill_options(firearms_skills()),
                    1,
                ),
                fixed("Navigate"),
                fixed("First Aid"),
                interpersonal("military-interpersonal", 1),
                any_skill("military-any", 3, "Any three occupation skills"),
            ],
        ),
        occupation(
            "Nurse",
            (9, 30),
            vec![FormulaKey::Edu4],
            vec![
                fixed("First Aid"),
                fixed("Listen"),
                fixed("Medicine"),
                interpersonal("nurse-interpersonal", 1),
                fixed("Psychology"),
                fixed("Science (Biology)"),
                fixed("Science (Chemistry)"),
                fixed("Spot Hidden"),
            ],
        ),
        occupation(
            "Occultist",
            (9, 65),
            vec![FormulaKey::Edu4],
            vec![
                fixed("Anthropology"),
                fixed("History"),
                fixed("Library Use"),
                fixed("Occult"),
                fixed("Language (Other)"),
                fixed("Science (Astronomy)"),
                any_skill("occultist-any", 2, "Any two occupation skills"),
            ],
        ),
        occupation(
            "Parapsychologist",
            (9, 30),
            vec![FormulaKey::Edu4],
            vec![
                fixed("Anthropology"),
                fixed("Art/Craft (Photography)"),
                fixed("History"),
                fixed("Library Use"),
                fixed("Occult"),
                fixed("Language (Other)"),
                fixed("Psychology"),
                any_skill("parapsychologist-any", 1, "Any occupation skill"),
            ],
        ),
        occupation(
            "Photographer",
            (9, 30),
            vec![FormulaKey::Edu4],
            vec![
                fixed("Art/Craft (Photography)"),
                fixed("Psychology"),
                fixed("Science (Chemistry)"),
                fixed("Stealth"),
                fixed("Spot Hidden"),
                any_skill("photographer-any", 3, "Any three occupation skills"),
            ],
        ),
        occupation(
            "Police Detective",
            (20, 50),
            vec![FormulaKey::Edu2Dex2, FormulaKey::Edu2Str2],
            vec![
                choice(
                    "police-acting-disguise",
                    "Acting or Disguise",
                    vec!["Art/Craft (Acting)".to_owned(), "Disguise".to_owned()],
                    1,
                ),
                choice(
                    "police-firearms",
                    "Firearms specialty",
                    skill_options(firearms_skills()),
                    1,
                ),
                fixed("Law"),
                fixed("Listen"),
                interpersonal("police-interpersonal", 1),
                fixed("Psychology"),
                fixed("Spot Hidden"),
                any_skill("police-any", 1, "Any occupation skill"),
            ],
        ),
        occupation(
            "Private Investigator",
            (9, 30),
            vec![FormulaKey::Edu2Dex2, FormulaKey::Edu2Str2],
            vec![
                fixed("Art/Craft (Photography)"),
                fixed("Disguise"),
                fixed("Law"),
                fixed("Library Use"),
                interpersonal("pi-interpersonal", 1),
                fixed("Psychology"),
                fixed("Spot Hidden"),
                any_skill("pi-any", 1, "Any occupation skill"),
            ],
        ),
        occupation(
            "Professor",
            (20, 70),
            vec![FormulaKey::Edu4],
            vec![
                fixed("Library Use"),
                fixed("Language (Other)"),
                fixed("Language (Own)"),
                fixed("Psychology"),
                any_skill("professor-any", 4, "Any four academic/personal specialties"),
            ],
        ),
        occupation(
            "Psychiatrist",
            (30, 60),
            vec![FormulaKey::Edu4],
            vec![
                fixed("Listen"),
                fixed("Medicine"),
                fixed("Language (Other)"),
                fixed("Persuade"),
                fixed("Psychoanalysis"),
                fixed("Psychology"),
                fixed("Science (Biology)"),
                any_skill("psychiatrist-any", 1, "Any occupation skill"),
            ],
        ),
        occupation(
            "Scientist",
            (9, 50),
            vec![FormulaKey::Edu4],
            vec![
                fixed("Library Use"),
                fixed("Language (Other)"),
                fixed("Language (Own)"),
                choice(
                    "scientist-science",
                    "Science specialties",
                    skill_options(science_skills()),
                    2,
                ),
                fixed("Spot Hidden"),
                any_skill("scientist-any", 2, "Any two occupation skills"),
            ],
        ),
        occupation(
            "Soldier",
            (9, 30),
            vec![FormulaKey::Edu2Dex2, FormulaKey::Edu2Str2],
            vec![
                choice(
                    "soldier-climb-swim",
                    "Climb or Swim",
                    vec!["Climb".to_owned(), "Swim".to_owned()],
                    1,
                ),
                fixed("Dodge"),
                fixed("Fighting (Brawl)"),
                choice(
                    "soldier-firearms",
                    "Firearms specialty",
                    skill_options(firearms_skills()),
                    1,
                ),
                fixed("Stealth"),
                fixed("Survival"),
                choice(
                    "soldier-two",
                    "Choose two",
                    vec![
                        "First Aid".to_owned(),
                        "Mechanical Repair".to_owned(),
                        "Language (Other)".to_owned(),
                    ],
                    2,
                ),
            ],
        ),
        occupation(
            "Student",
            (5, 10),
            vec![FormulaKey::Edu4],
            vec![
                fixed("Library Use"),
                fixed("Listen"),
                fixed("Language (Other)"),
                fixed("Language (Own)"),
                any_skill("student-any", 4, "Any four occupation skills"),
            ],
        ),
    ]
}

pub(crate) fn backstory_hint(category: &str) -> &'static str {
    match category {
        "Personal Description" => "Tall, gaunt, wire-rimmed glasses, always fidgeting...",
        "Ideology/Beliefs" => "Science can explain everything — or so they believed.",
        "Significant People" => "A mentor, rival, family member, old friend, or person you failed.",
        "Meaningful Locations" => {
            "A childhood home, university office, battlefield, library, asylum, or forbidden ruin."
        }
        "Treasured Possessions" => {
            "A photograph, medal, notebook, weapon, heirloom, or object tied to a secret."
        }
        "Traits" => "Nervous laugh, stubborn skeptic, compulsive note-taker, loyal to a fault.",
        "Injuries & Scars" => "Old bullet wound, burn scar, limp, missing finger, chronic cough.",
        "Phobias & Manias" => {
            "Claustrophobia, fear of the sea, obsession with clocks, fixation on old books."
        }
        "Arcane Tomes/Spells/Artifacts" => {
            "Leave blank unless the Keeper approves starting Mythos material."
        }
        "Encounters with Strange Entities" => {
            "Leave blank unless your Keeper wants pre-campaign Mythos exposure."
        }
        _ => "Write a table-ready detail.",
    }
}
