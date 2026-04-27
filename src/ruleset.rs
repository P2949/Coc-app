use super::data::{
    AgeBracket, Characteristic, CharacteristicDef, DamageRow, DiceKind, FormulaKey, Skill,
    SkillBase, SkillSpec,
};

pub(crate) const CHARACTERISTICS: &[CharacteristicDef] = &[
    CharacteristicDef {
        key: Characteristic::Str,
        name: "Strength",
        dice: DiceKind::ThreeD6,
        min: 15,
        max: 90,
        desc: "Physical power",
    },
    CharacteristicDef {
        key: Characteristic::Con,
        name: "Constitution",
        dice: DiceKind::ThreeD6,
        min: 15,
        max: 90,
        desc: "Health and resilience",
    },
    CharacteristicDef {
        key: Characteristic::Siz,
        name: "Size",
        dice: DiceKind::TwoD6Plus6,
        min: 40,
        max: 90,
        desc: "Height and mass",
    },
    CharacteristicDef {
        key: Characteristic::Dex,
        name: "Dexterity",
        dice: DiceKind::ThreeD6,
        min: 15,
        max: 90,
        desc: "Agility and reflexes",
    },
    CharacteristicDef {
        key: Characteristic::App,
        name: "Appearance",
        dice: DiceKind::ThreeD6,
        min: 15,
        max: 90,
        desc: "Presence and looks",
    },
    CharacteristicDef {
        key: Characteristic::Int,
        name: "Intelligence",
        dice: DiceKind::TwoD6Plus6,
        min: 40,
        max: 90,
        desc: "Reasoning and intuition",
    },
    CharacteristicDef {
        key: Characteristic::Pow,
        name: "Power",
        dice: DiceKind::ThreeD6,
        min: 15,
        max: 90,
        desc: "Willpower and mental stability",
    },
    CharacteristicDef {
        key: Characteristic::Edu,
        name: "Education",
        dice: DiceKind::TwoD6Plus6,
        min: 40,
        max: 90,
        desc: "Knowledge and schooling",
    },
];

pub(crate) const STEPS: &[&str] = &[
    "Concept",
    "Characteristics",
    "Occupation",
    "Skills",
    "Backstory",
    "Summary",
];

pub(crate) const AGE_BRACKETS: &[AgeBracket] = &[
    AgeBracket {
        label: "15–19",
        min: 15,
        max: 19,
        edu_checks: 0,
        edu_penalty: 5,
        app_penalty: 0,
        physical_deduct: 5,
        physical_from: &[Characteristic::Str, Characteristic::Siz],
        luck_advantage: true,
        mov_penalty: 0,
        note: "Deduct 5 total from STR or SIZ. Deduct 5 from EDU. Roll Luck twice and keep the higher result.",
    },
    AgeBracket {
        label: "20–39",
        min: 20,
        max: 39,
        edu_checks: 1,
        edu_penalty: 0,
        app_penalty: 0,
        physical_deduct: 0,
        physical_from: &[],
        luck_advantage: false,
        mov_penalty: 0,
        note: "Make 1 EDU improvement check.",
    },
    AgeBracket {
        label: "40–49",
        min: 40,
        max: 49,
        edu_checks: 2,
        edu_penalty: 0,
        app_penalty: 5,
        physical_deduct: 5,
        physical_from: &[
            Characteristic::Str,
            Characteristic::Con,
            Characteristic::Dex,
        ],
        luck_advantage: false,
        mov_penalty: 1,
        note: "2 EDU checks. Deduct 5 total from STR, CON, or DEX. Deduct 5 from APP. MOV −1.",
    },
    AgeBracket {
        label: "50–59",
        min: 50,
        max: 59,
        edu_checks: 3,
        edu_penalty: 0,
        app_penalty: 10,
        physical_deduct: 10,
        physical_from: &[
            Characteristic::Str,
            Characteristic::Con,
            Characteristic::Dex,
        ],
        luck_advantage: false,
        mov_penalty: 2,
        note: "3 EDU checks. Deduct 10 total from STR, CON, or DEX. Deduct 10 from APP. MOV −2.",
    },
    AgeBracket {
        label: "60–69",
        min: 60,
        max: 69,
        edu_checks: 4,
        edu_penalty: 0,
        app_penalty: 15,
        physical_deduct: 20,
        physical_from: &[
            Characteristic::Str,
            Characteristic::Con,
            Characteristic::Dex,
        ],
        luck_advantage: false,
        mov_penalty: 3,
        note: "4 EDU checks. Deduct 20 total from STR, CON, or DEX. Deduct 15 from APP. MOV −3.",
    },
    AgeBracket {
        label: "70–79",
        min: 70,
        max: 79,
        edu_checks: 4,
        edu_penalty: 0,
        app_penalty: 20,
        physical_deduct: 40,
        physical_from: &[
            Characteristic::Str,
            Characteristic::Con,
            Characteristic::Dex,
        ],
        luck_advantage: false,
        mov_penalty: 4,
        note: "4 EDU checks. Deduct 40 total from STR, CON, or DEX. Deduct 20 from APP. MOV −4.",
    },
    AgeBracket {
        label: "80–89",
        min: 80,
        max: 89,
        edu_checks: 4,
        edu_penalty: 0,
        app_penalty: 25,
        physical_deduct: 80,
        physical_from: &[
            Characteristic::Str,
            Characteristic::Con,
            Characteristic::Dex,
        ],
        luck_advantage: false,
        mov_penalty: 5,
        note: "4 EDU checks. Deduct 80 total from STR, CON, or DEX. Deduct 25 from APP. MOV −5.",
    },
];

pub(crate) const DB_BUILD_TABLE: &[DamageRow] = &[
    DamageRow {
        min: 2,
        max: 64,
        db: "−2",
        build: -2,
    },
    DamageRow {
        min: 65,
        max: 84,
        db: "−1",
        build: -1,
    },
    DamageRow {
        min: 85,
        max: 124,
        db: "None",
        build: 0,
    },
    DamageRow {
        min: 125,
        max: 164,
        db: "+1D4",
        build: 1,
    },
    DamageRow {
        min: 165,
        max: 204,
        db: "+1D6",
        build: 2,
    },
    DamageRow {
        min: 205,
        max: 284,
        db: "+2D6",
        build: 3,
    },
    DamageRow {
        min: 285,
        max: 364,
        db: "+3D6",
        build: 4,
    },
    DamageRow {
        min: 365,
        max: 444,
        db: "+4D6",
        build: 5,
    },
    DamageRow {
        min: 445,
        max: 524,
        db: "+5D6",
        build: 6,
    },
];

pub(crate) const SKILL_SPECS: &[SkillSpec] = &[
    SkillSpec {
        id: Skill::Accounting,
        name: "Accounting",
        base: SkillBase::Fixed(5),
    },
    SkillSpec {
        id: Skill::Anthropology,
        name: "Anthropology",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::Appraise,
        name: "Appraise",
        base: SkillBase::Fixed(5),
    },
    SkillSpec {
        id: Skill::Archaeology,
        name: "Archaeology",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::ArtCraft,
        name: "Art/Craft",
        base: SkillBase::Fixed(5),
    },
    SkillSpec {
        id: Skill::ArtCraftActing,
        name: "Art/Craft (Acting)",
        base: SkillBase::Fixed(5),
    },
    SkillSpec {
        id: Skill::ArtCraftFineArt,
        name: "Art/Craft (Fine Art)",
        base: SkillBase::Fixed(5),
    },
    SkillSpec {
        id: Skill::ArtCraftForgery,
        name: "Art/Craft (Forgery)",
        base: SkillBase::Fixed(5),
    },
    SkillSpec {
        id: Skill::ArtCraftLiterature,
        name: "Art/Craft (Literature)",
        base: SkillBase::Fixed(5),
    },
    SkillSpec {
        id: Skill::ArtCraftPhotography,
        name: "Art/Craft (Photography)",
        base: SkillBase::Fixed(5),
    },
    SkillSpec {
        id: Skill::ArtCraftTechnicalDrawing,
        name: "Art/Craft (Technical Drawing)",
        base: SkillBase::Fixed(5),
    },
    SkillSpec {
        id: Skill::ArtCraftWriting,
        name: "Art/Craft (Writing)",
        base: SkillBase::Fixed(5),
    },
    SkillSpec {
        id: Skill::Charm,
        name: "Charm",
        base: SkillBase::Fixed(15),
    },
    SkillSpec {
        id: Skill::Climb,
        name: "Climb",
        base: SkillBase::Fixed(20),
    },
    SkillSpec {
        id: Skill::CreditRating,
        name: "Credit Rating",
        base: SkillBase::Fixed(0),
    },
    SkillSpec {
        id: Skill::CthulhuMythos,
        name: "Cthulhu Mythos",
        base: SkillBase::Fixed(0),
    },
    SkillSpec {
        id: Skill::Disguise,
        name: "Disguise",
        base: SkillBase::Fixed(5),
    },
    SkillSpec {
        id: Skill::Dodge,
        name: "Dodge",
        base: SkillBase::HalfDex,
    },
    SkillSpec {
        id: Skill::DriveAuto,
        name: "Drive Auto",
        base: SkillBase::Fixed(20),
    },
    SkillSpec {
        id: Skill::ElectricalRepair,
        name: "Electrical Repair",
        base: SkillBase::Fixed(10),
    },
    SkillSpec {
        id: Skill::FastTalk,
        name: "Fast Talk",
        base: SkillBase::Fixed(5),
    },
    SkillSpec {
        id: Skill::FightingBrawl,
        name: "Fighting (Brawl)",
        base: SkillBase::Fixed(25),
    },
    SkillSpec {
        id: Skill::FirearmsHandgun,
        name: "Firearms (Handgun)",
        base: SkillBase::Fixed(20),
    },
    SkillSpec {
        id: Skill::FirearmsRifleShotgun,
        name: "Firearms (Rifle/Shotgun)",
        base: SkillBase::Fixed(25),
    },
    SkillSpec {
        id: Skill::FirstAid,
        name: "First Aid",
        base: SkillBase::Fixed(30),
    },
    SkillSpec {
        id: Skill::History,
        name: "History",
        base: SkillBase::Fixed(5),
    },
    SkillSpec {
        id: Skill::Intimidate,
        name: "Intimidate",
        base: SkillBase::Fixed(15),
    },
    SkillSpec {
        id: Skill::Jump,
        name: "Jump",
        base: SkillBase::Fixed(20),
    },
    SkillSpec {
        id: Skill::LanguageOther,
        name: "Language (Other)",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::LanguageOwn,
        name: "Language (Own)",
        base: SkillBase::Edu,
    },
    SkillSpec {
        id: Skill::Law,
        name: "Law",
        base: SkillBase::Fixed(5),
    },
    SkillSpec {
        id: Skill::LibraryUse,
        name: "Library Use",
        base: SkillBase::Fixed(20),
    },
    SkillSpec {
        id: Skill::Listen,
        name: "Listen",
        base: SkillBase::Fixed(20),
    },
    SkillSpec {
        id: Skill::Locksmith,
        name: "Locksmith",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::MechanicalRepair,
        name: "Mechanical Repair",
        base: SkillBase::Fixed(10),
    },
    SkillSpec {
        id: Skill::Medicine,
        name: "Medicine",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::NaturalWorld,
        name: "Natural World",
        base: SkillBase::Fixed(10),
    },
    SkillSpec {
        id: Skill::Navigate,
        name: "Navigate",
        base: SkillBase::Fixed(10),
    },
    SkillSpec {
        id: Skill::Occult,
        name: "Occult",
        base: SkillBase::Fixed(5),
    },
    SkillSpec {
        id: Skill::OperateHeavyMachinery,
        name: "Operate Heavy Machinery",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::Persuade,
        name: "Persuade",
        base: SkillBase::Fixed(10),
    },
    SkillSpec {
        id: Skill::Pilot,
        name: "Pilot",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::Psychoanalysis,
        name: "Psychoanalysis",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::Psychology,
        name: "Psychology",
        base: SkillBase::Fixed(10),
    },
    SkillSpec {
        id: Skill::Ride,
        name: "Ride",
        base: SkillBase::Fixed(5),
    },
    SkillSpec {
        id: Skill::ScienceAstronomy,
        name: "Science (Astronomy)",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::ScienceBiology,
        name: "Science (Biology)",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::ScienceBotany,
        name: "Science (Botany)",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::ScienceChemistry,
        name: "Science (Chemistry)",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::ScienceCryptography,
        name: "Science (Cryptography)",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::ScienceEngineering,
        name: "Science (Engineering)",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::ScienceForensics,
        name: "Science (Forensics)",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::ScienceGeology,
        name: "Science (Geology)",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::ScienceMathematics,
        name: "Science (Mathematics)",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::SciencePharmacy,
        name: "Science (Pharmacy)",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::SciencePhysics,
        name: "Science (Physics)",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::ScienceZoology,
        name: "Science (Zoology)",
        base: SkillBase::Fixed(1),
    },
    SkillSpec {
        id: Skill::SleightOfHand,
        name: "Sleight of Hand",
        base: SkillBase::Fixed(10),
    },
    SkillSpec {
        id: Skill::SpotHidden,
        name: "Spot Hidden",
        base: SkillBase::Fixed(25),
    },
    SkillSpec {
        id: Skill::Stealth,
        name: "Stealth",
        base: SkillBase::Fixed(20),
    },
    SkillSpec {
        id: Skill::Survival,
        name: "Survival",
        base: SkillBase::Fixed(10),
    },
    SkillSpec {
        id: Skill::Swim,
        name: "Swim",
        base: SkillBase::Fixed(20),
    },
    SkillSpec {
        id: Skill::Throw,
        name: "Throw",
        base: SkillBase::Fixed(20),
    },
    SkillSpec {
        id: Skill::Track,
        name: "Track",
        base: SkillBase::Fixed(10),
    },
];

pub(crate) const BACKSTORY_CATEGORIES: &[&str] = &[
    "Personal Description",
    "Ideology/Beliefs",
    "Significant People",
    "Meaningful Locations",
    "Treasured Possessions",
    "Traits",
    "Injuries & Scars",
    "Phobias & Manias",
    "Arcane Tomes/Spells/Artifacts",
    "Encounters with Strange Entities",
];

pub(crate) const SUMMARY_ALWAYS_SHOW: &[&str] =
    &["Credit Rating", "Cthulhu Mythos", "Dodge", "Language (Own)"];

pub(crate) const ALL_FORMULAS: &[FormulaKey] = &[
    FormulaKey::Edu4,
    FormulaKey::Edu2Dex2,
    FormulaKey::Edu2App2,
    FormulaKey::Edu2Str2,
    FormulaKey::Edu2Pow2,
];
