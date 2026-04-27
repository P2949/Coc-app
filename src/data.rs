/*
Cargo.toml

[package]
name = "coc7e_investigator_creator"
version = "0.1.0"
edition = "2024"

[dependencies]
eframe = "0.34"
rand = { version = "0.10", features = ["small_rng"] }

Run:
  cargo fmt
  cargo test
  cargo run --release
*/

use eframe::egui::Color32;
use rand::{RngExt, SeedableRng, rngs::SmallRng};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::ops::Index;

pub(crate) const CUSTOM_OCCUPATION_ID: &str = "__custom__";
// Optional helper budget, not an official CoC 7e point-buy rule.
// 460 is the total used by this app's balanced adjustable preset.
pub(crate) const POINT_BUY_BUDGET: i32 = 460;
pub(crate) const MAX_CREATION_VALUE: i32 = 99;
pub(crate) const APP_CONTENT_WIDTH: f32 = 1020.0;
pub(crate) const APP_INITIAL_WIDTH: f32 = 1120.0;
pub(crate) const APP_INITIAL_HEIGHT: f32 = 920.0;
pub(crate) const APP_MIN_WINDOW_WIDTH: f32 = 760.0;
pub(crate) const APP_MIN_WINDOW_HEIGHT: f32 = 560.0;

pub(crate) const BG: Color32 = Color32::from_rgb(8, 9, 12);
pub(crate) const PANEL: Color32 = Color32::from_rgb(17, 19, 24);
pub(crate) const PANEL_2: Color32 = Color32::from_rgb(21, 24, 33);
pub(crate) const LINE: Color32 = Color32::from_rgb(37, 42, 54);
pub(crate) const TEXT: Color32 = Color32::from_rgb(230, 232, 238);
pub(crate) const MUTED: Color32 = Color32::from_rgb(139, 146, 163);
pub(crate) const FAINT: Color32 = Color32::from_rgb(95, 102, 117);
pub(crate) const ACCENT: Color32 = Color32::from_rgb(155, 140, 255);
pub(crate) const ACCENT_DIM: Color32 = Color32::from_rgb(111, 101, 200);
pub(crate) const GREEN: Color32 = Color32::from_rgb(114, 214, 162);
pub(crate) const AMBER: Color32 = Color32::from_rgb(216, 184, 109);
pub(crate) const RED: Color32 = Color32::from_rgb(225, 119, 119);
pub(crate) const BLUE: Color32 = Color32::from_rgb(130, 170, 255);

#[derive(Clone)]
pub(crate) struct AppRng {
    pub(crate) inner: SmallRng,
}

impl fmt::Debug for AppRng {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AppRng { SmallRng }")
    }
}

impl AppRng {
    pub(crate) fn seeded(seed: u64) -> Self {
        Self {
            inner: SmallRng::seed_from_u64(seed),
        }
    }

    pub(crate) fn roll_inclusive(&mut self, sides: u32) -> u32 {
        debug_assert!(sides > 0);
        self.inner.random_range(1..=sides)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Characteristic {
    Str,
    Con,
    Siz,
    Dex,
    App,
    Int,
    Pow,
    Edu,
}

impl Characteristic {
    pub(crate) const COUNT: usize = 8;

    pub(crate) fn index(self) -> usize {
        match self {
            Self::Str => 0,
            Self::Con => 1,
            Self::Siz => 2,
            Self::Dex => 3,
            Self::App => 4,
            Self::Int => 5,
            Self::Pow => 6,
            Self::Edu => 7,
        }
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Str => "STR",
            Self::Con => "CON",
            Self::Siz => "SIZ",
            Self::Dex => "DEX",
            Self::App => "APP",
            Self::Int => "INT",
            Self::Pow => "POW",
            Self::Edu => "EDU",
        }
    }

    pub(crate) fn from_key(key: &str) -> Option<Self> {
        match key {
            "STR" => Some(Self::Str),
            "CON" => Some(Self::Con),
            "SIZ" => Some(Self::Siz),
            "DEX" => Some(Self::Dex),
            "APP" => Some(Self::App),
            "INT" => Some(Self::Int),
            "POW" => Some(Self::Pow),
            "EDU" => Some(Self::Edu),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CharacteristicValues {
    pub(crate) values: [i32; Characteristic::COUNT],
}

impl Default for CharacteristicValues {
    fn default() -> Self {
        Self {
            values: [0; Characteristic::COUNT],
        }
    }
}

impl CharacteristicValues {
    pub(crate) fn get_char(&self, key: Characteristic) -> i32 {
        self.values[key.index()]
    }

    pub(crate) fn set_char(&mut self, key: Characteristic, value: i32) {
        self.values[key.index()] = value;
    }

    pub(crate) fn get(&self, key: &str) -> Option<&i32> {
        Characteristic::from_key(key).map(|id| &self.values[id.index()])
    }
}

impl Index<&str> for CharacteristicValues {
    type Output = i32;

    fn index(&self, key: &str) -> &Self::Output {
        self.get(key)
            .unwrap_or_else(|| panic!("unknown characteristic key: {key}"))
    }
}

impl Index<Characteristic> for CharacteristicValues {
    type Output = i32;

    fn index(&self, key: Characteristic) -> &Self::Output {
        &self.values[key.index()]
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CharacteristicDef {
    pub(crate) key: Characteristic,
    pub(crate) name: &'static str,
    pub(crate) dice: DiceKind,
    pub(crate) min: i32,
    pub(crate) max: i32,
    pub(crate) desc: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DiceKind {
    ThreeD6,
    TwoD6Plus6,
}

impl DiceKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::ThreeD6 => "3d6",
            Self::TwoD6Plus6 => "2d6+6",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AgeBracket {
    pub(crate) label: &'static str,
    pub(crate) min: i32,
    pub(crate) max: i32,
    pub(crate) edu_checks: usize,
    pub(crate) edu_penalty: i32,
    pub(crate) app_penalty: i32,
    pub(crate) physical_deduct: i32,
    pub(crate) physical_from: &'static [Characteristic],
    pub(crate) luck_advantage: bool,
    pub(crate) mov_penalty: i32,
    pub(crate) note: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DamageRow {
    pub(crate) min: i32,
    pub(crate) max: i32,
    pub(crate) db: &'static str,
    pub(crate) build: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Skill {
    Accounting,
    Anthropology,
    Appraise,
    Archaeology,
    ArtCraft,
    ArtCraftActing,
    ArtCraftFineArt,
    ArtCraftForgery,
    ArtCraftLiterature,
    ArtCraftPhotography,
    ArtCraftTechnicalDrawing,
    ArtCraftWriting,
    Charm,
    Climb,
    CreditRating,
    CthulhuMythos,
    Disguise,
    Dodge,
    DriveAuto,
    ElectricalRepair,
    FastTalk,
    FightingBrawl,
    FirearmsHandgun,
    FirearmsRifleShotgun,
    FirstAid,
    History,
    Intimidate,
    Jump,
    LanguageOther,
    LanguageOwn,
    Law,
    LibraryUse,
    Listen,
    Locksmith,
    MechanicalRepair,
    Medicine,
    NaturalWorld,
    Navigate,
    Occult,
    OperateHeavyMachinery,
    Persuade,
    Pilot,
    Psychoanalysis,
    Psychology,
    Ride,
    ScienceAstronomy,
    ScienceBiology,
    ScienceBotany,
    ScienceChemistry,
    ScienceCryptography,
    ScienceEngineering,
    ScienceForensics,
    ScienceGeology,
    ScienceMathematics,
    SciencePharmacy,
    SciencePhysics,
    ScienceZoology,
    SleightOfHand,
    SpotHidden,
    Stealth,
    Survival,
    Swim,
    Throw,
    Track,
}

impl Skill {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Accounting => "Accounting",
            Self::Anthropology => "Anthropology",
            Self::Appraise => "Appraise",
            Self::Archaeology => "Archaeology",
            Self::ArtCraft => "Art/Craft",
            Self::ArtCraftActing => "Art/Craft (Acting)",
            Self::ArtCraftFineArt => "Art/Craft (Fine Art)",
            Self::ArtCraftForgery => "Art/Craft (Forgery)",
            Self::ArtCraftLiterature => "Art/Craft (Literature)",
            Self::ArtCraftPhotography => "Art/Craft (Photography)",
            Self::ArtCraftTechnicalDrawing => "Art/Craft (Technical Drawing)",
            Self::ArtCraftWriting => "Art/Craft (Writing)",
            Self::Charm => "Charm",
            Self::Climb => "Climb",
            Self::CreditRating => "Credit Rating",
            Self::CthulhuMythos => "Cthulhu Mythos",
            Self::Disguise => "Disguise",
            Self::Dodge => "Dodge",
            Self::DriveAuto => "Drive Auto",
            Self::ElectricalRepair => "Electrical Repair",
            Self::FastTalk => "Fast Talk",
            Self::FightingBrawl => "Fighting (Brawl)",
            Self::FirearmsHandgun => "Firearms (Handgun)",
            Self::FirearmsRifleShotgun => "Firearms (Rifle/Shotgun)",
            Self::FirstAid => "First Aid",
            Self::History => "History",
            Self::Intimidate => "Intimidate",
            Self::Jump => "Jump",
            Self::LanguageOther => "Language (Other)",
            Self::LanguageOwn => "Language (Own)",
            Self::Law => "Law",
            Self::LibraryUse => "Library Use",
            Self::Listen => "Listen",
            Self::Locksmith => "Locksmith",
            Self::MechanicalRepair => "Mechanical Repair",
            Self::Medicine => "Medicine",
            Self::NaturalWorld => "Natural World",
            Self::Navigate => "Navigate",
            Self::Occult => "Occult",
            Self::OperateHeavyMachinery => "Operate Heavy Machinery",
            Self::Persuade => "Persuade",
            Self::Pilot => "Pilot",
            Self::Psychoanalysis => "Psychoanalysis",
            Self::Psychology => "Psychology",
            Self::Ride => "Ride",
            Self::ScienceAstronomy => "Science (Astronomy)",
            Self::ScienceBiology => "Science (Biology)",
            Self::ScienceBotany => "Science (Botany)",
            Self::ScienceChemistry => "Science (Chemistry)",
            Self::ScienceCryptography => "Science (Cryptography)",
            Self::ScienceEngineering => "Science (Engineering)",
            Self::ScienceForensics => "Science (Forensics)",
            Self::ScienceGeology => "Science (Geology)",
            Self::ScienceMathematics => "Science (Mathematics)",
            Self::SciencePharmacy => "Science (Pharmacy)",
            Self::SciencePhysics => "Science (Physics)",
            Self::ScienceZoology => "Science (Zoology)",
            Self::SleightOfHand => "Sleight of Hand",
            Self::SpotHidden => "Spot Hidden",
            Self::Stealth => "Stealth",
            Self::Survival => "Survival",
            Self::Swim => "Swim",
            Self::Throw => "Throw",
            Self::Track => "Track",
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name {
            "Accounting" => Some(Self::Accounting),
            "Anthropology" => Some(Self::Anthropology),
            "Appraise" => Some(Self::Appraise),
            "Archaeology" => Some(Self::Archaeology),
            "Art/Craft" => Some(Self::ArtCraft),
            "Art/Craft (Acting)" => Some(Self::ArtCraftActing),
            "Art/Craft (Fine Art)" => Some(Self::ArtCraftFineArt),
            "Art/Craft (Forgery)" => Some(Self::ArtCraftForgery),
            "Art/Craft (Literature)" => Some(Self::ArtCraftLiterature),
            "Art/Craft (Photography)" => Some(Self::ArtCraftPhotography),
            "Art/Craft (Technical Drawing)" => Some(Self::ArtCraftTechnicalDrawing),
            "Art/Craft (Writing)" => Some(Self::ArtCraftWriting),
            "Charm" => Some(Self::Charm),
            "Climb" => Some(Self::Climb),
            "Credit Rating" => Some(Self::CreditRating),
            "Cthulhu Mythos" => Some(Self::CthulhuMythos),
            "Disguise" => Some(Self::Disguise),
            "Dodge" => Some(Self::Dodge),
            "Drive Auto" => Some(Self::DriveAuto),
            "Electrical Repair" => Some(Self::ElectricalRepair),
            "Fast Talk" => Some(Self::FastTalk),
            "Fighting (Brawl)" => Some(Self::FightingBrawl),
            "Firearms (Handgun)" => Some(Self::FirearmsHandgun),
            "Firearms (Rifle/Shotgun)" => Some(Self::FirearmsRifleShotgun),
            "First Aid" => Some(Self::FirstAid),
            "History" => Some(Self::History),
            "Intimidate" => Some(Self::Intimidate),
            "Jump" => Some(Self::Jump),
            "Language (Other)" => Some(Self::LanguageOther),
            "Language (Own)" => Some(Self::LanguageOwn),
            "Law" => Some(Self::Law),
            "Library Use" => Some(Self::LibraryUse),
            "Listen" => Some(Self::Listen),
            "Locksmith" => Some(Self::Locksmith),
            "Mechanical Repair" => Some(Self::MechanicalRepair),
            "Medicine" => Some(Self::Medicine),
            "Natural World" => Some(Self::NaturalWorld),
            "Navigate" => Some(Self::Navigate),
            "Occult" => Some(Self::Occult),
            "Operate Heavy Machinery" => Some(Self::OperateHeavyMachinery),
            "Persuade" => Some(Self::Persuade),
            "Pilot" => Some(Self::Pilot),
            "Psychoanalysis" => Some(Self::Psychoanalysis),
            "Psychology" => Some(Self::Psychology),
            "Ride" => Some(Self::Ride),
            "Science (Astronomy)" => Some(Self::ScienceAstronomy),
            "Science (Biology)" => Some(Self::ScienceBiology),
            "Science (Botany)" => Some(Self::ScienceBotany),
            "Science (Chemistry)" => Some(Self::ScienceChemistry),
            "Science (Cryptography)" => Some(Self::ScienceCryptography),
            "Science (Engineering)" => Some(Self::ScienceEngineering),
            "Science (Forensics)" => Some(Self::ScienceForensics),
            "Science (Geology)" => Some(Self::ScienceGeology),
            "Science (Mathematics)" => Some(Self::ScienceMathematics),
            "Science (Pharmacy)" => Some(Self::SciencePharmacy),
            "Science (Physics)" => Some(Self::SciencePhysics),
            "Science (Zoology)" => Some(Self::ScienceZoology),
            "Sleight of Hand" => Some(Self::SleightOfHand),
            "Spot Hidden" => Some(Self::SpotHidden),
            "Stealth" => Some(Self::Stealth),
            "Survival" => Some(Self::Survival),
            "Swim" => Some(Self::Swim),
            "Throw" => Some(Self::Throw),
            "Track" => Some(Self::Track),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum SkillBase {
    Fixed(i32),
    HalfDex,
    Edu,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SkillSpec {
    pub(crate) id: Skill,
    pub(crate) name: &'static str,
    pub(crate) base: SkillBase,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum FormulaKey {
    Edu4,
    Edu2Dex2,
    Edu2App2,
    Edu2Str2,
    Edu2Pow2,
}

impl FormulaKey {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Edu4 => "EDU × 4",
            Self::Edu2Dex2 => "EDU × 2 + DEX × 2",
            Self::Edu2App2 => "EDU × 2 + APP × 2",
            Self::Edu2Str2 => "EDU × 2 + STR × 2",
            Self::Edu2Pow2 => "EDU × 2 + POW × 2",
        }
    }

    pub(crate) fn calculate(self, c: &CharacteristicValues) -> i32 {
        match self {
            Self::Edu4 => c.get_char(Characteristic::Edu) * 4,
            Self::Edu2Dex2 => {
                c.get_char(Characteristic::Edu) * 2 + c.get_char(Characteristic::Dex) * 2
            }
            Self::Edu2App2 => {
                c.get_char(Characteristic::Edu) * 2 + c.get_char(Characteristic::App) * 2
            }
            Self::Edu2Str2 => {
                c.get_char(Characteristic::Edu) * 2 + c.get_char(Characteristic::Str) * 2
            }
            Self::Edu2Pow2 => {
                c.get_char(Characteristic::Edu) * 2 + c.get_char(Characteristic::Pow) * 2
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Slot {
    Skill(String),
    Choice {
        id: String,
        label: String,
        options: Vec<String>,
        count: usize,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct Occupation {
    pub(crate) name: String,
    pub(crate) credit: (i32, i32),
    pub(crate) formula_keys: Vec<FormulaKey>,
    pub(crate) slots: Vec<Slot>,
}

#[derive(Clone, Debug)]
pub(crate) struct DiceResult {
    pub(crate) rolls: Vec<u32>,
    pub(crate) plus_six: bool,
    pub(crate) value: i32,
    pub(crate) kept: Option<bool>,
}

#[derive(Clone, Debug)]
pub(crate) struct Concept {
    pub(crate) name: String,
    pub(crate) age: i32,
    pub(crate) pronouns: String,
    pub(crate) residence: String,
    pub(crate) birthplace: String,
}

impl Default for Concept {
    fn default() -> Self {
        Self {
            name: String::new(),
            age: 30,
            pronouns: String::new(),
            residence: String::new(),
            birthplace: String::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CustomOccupation {
    pub(crate) name: String,
    pub(crate) credit_min: i32,
    pub(crate) credit_max: i32,
    pub(crate) formula_key: FormulaKey,
    pub(crate) skills: Vec<String>,
}

impl Default for CustomOccupation {
    fn default() -> Self {
        Self {
            name: "Custom Occupation".to_owned(),
            credit_min: 9,
            credit_max: 60,
            formula_key: FormulaKey::Edu4,
            skills: vec![String::new(); 8],
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct EduCheckRoll {
    pub(crate) d100: i32,
    pub(crate) improved: bool,
    pub(crate) gain: i32,
    pub(crate) resulting_edu: i32,
}

#[derive(Clone, Debug)]
pub(crate) struct SkillRow {
    pub(crate) name: String,
    pub(crate) base: i32,
    pub(crate) occ_add: i32,
    pub(crate) personal_add: i32,
    pub(crate) total: i32,
}

#[derive(Clone, Debug)]
pub(crate) struct Derived {
    pub(crate) hp: i32,
    pub(crate) san: i32,
    pub(crate) max_san: i32,
    pub(crate) mp: i32,
    pub(crate) mov: i32,
    pub(crate) dodge: i32,
    pub(crate) db: String,
    pub(crate) build: i32,
    pub(crate) major_wound: i32,
}

#[derive(Clone, Debug)]
pub(crate) struct SheetMath {
    pub(crate) final_chars: CharacteristicValues,
    pub(crate) skill_rows: Vec<SkillRow>,
    pub(crate) derived: Derived,
    pub(crate) selected_occupation: Option<Occupation>,
    pub(crate) credit_range: (i32, i32),
    pub(crate) unresolved_choices: usize,
    pub(crate) occupation_shortfall: usize,
    pub(crate) occupation_skill_set: HashSet<String>,
    pub(crate) occupation_budget: i32,
    pub(crate) personal_budget: i32,
    pub(crate) credit_rating: i32,
}

#[derive(PartialEq, Eq, Debug)]
pub(crate) enum CharMethod {
    Roll,
    PointBuy,
    QuickArray,
    Mixed,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ChoiceKey {
    pub(crate) id: String,
    pub(crate) index: usize,
}

impl ChoiceKey {
    pub(crate) fn new(id: impl Into<String>, index: usize) -> Self {
        Self {
            id: id.into(),
            index,
        }
    }

    pub(crate) fn widget_id(&self) -> String {
        format!("{}:{}", self.id, self.index)
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct LuckState {
    pub(crate) value: Option<i32>,
    pub(crate) rolls: Vec<DiceResult>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct AllocationState {
    pub(crate) occupation_points: HashMap<String, i32>,
    pub(crate) personal_points: HashMap<String, i32>,
}

#[derive(Debug)]
pub(crate) struct CoC7eApp {
    pub(crate) step: usize,
    pub(crate) concept: Concept,
    pub(crate) char_method: CharMethod,
    pub(crate) chars: CharacteristicValues,
    pub(crate) char_rolls: HashMap<String, DiceResult>,
    pub(crate) luck_state: LuckState,
    pub(crate) age_deductions: CharacteristicValues,
    pub(crate) edu_bonus: i32,
    pub(crate) edu_check_rolls: Vec<EduCheckRoll>,
    pub(crate) occupation_id: String,
    pub(crate) formula_key: FormulaKey,
    pub(crate) occupation_choices: HashMap<ChoiceKey, String>,
    pub(crate) custom_occupation: CustomOccupation,
    pub(crate) allocations: AllocationState,
    pub(crate) backstory: HashMap<String, String>,
    pub(crate) occupations: Vec<Occupation>,
    pub(crate) last_age_bracket_index: usize,
    pub(crate) frame_max_reachable_step: usize,
    pub(crate) rng: AppRng,
}
