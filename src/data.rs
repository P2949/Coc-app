use eframe::egui::Color32;
use rand::{RngExt, SeedableRng, rngs::SmallRng};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error as DeError, MapAccess, SeqAccess, Visitor},
    ser::SerializeMap,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::ops::Index;

pub(crate) const CUSTOM_OCCUPATION_ID: &str = "__custom__";
pub(crate) const INVESTIGATOR_SAVE_VERSION: u32 = 2;
pub(crate) const CUSTOM_OCCUPATION_MIN_SKILL_COUNT: usize = 1;
pub(crate) const CUSTOM_OCCUPATION_SKILL_COUNT: usize = 8;
pub(crate) const DEFAULT_RNG_SEED: u64 = 0xC0C7_E7E5_1234_5678;
pub(crate) const MAX_RNG_ROLL_HISTORY: usize = 4096;
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

    pub(crate) fn reseed_from_stream(&mut self) -> u64 {
        let seed = match self.inner.random::<u64>() {
            0 => DEFAULT_RNG_SEED,
            seed => seed,
        };
        self.inner = SmallRng::seed_from_u64(seed);
        seed
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
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

pub(crate) const CHARACTERISTIC_SAVE_KEYS: &[Characteristic; Characteristic::COUNT] = &[
    Characteristic::Str,
    Characteristic::Con,
    Characteristic::Siz,
    Characteristic::Dex,
    Characteristic::App,
    Characteristic::Int,
    Characteristic::Pow,
    Characteristic::Edu,
];

pub(crate) const CHARACTERISTIC_SAVE_FIELD_NAMES: &[&str] = &[
    "STR", "CON", "SIZ", "DEX", "APP", "INT", "POW", "EDU", "values",
];

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

impl Serialize for CharacteristicValues {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(Characteristic::COUNT))?;
        for key in CHARACTERISTIC_SAVE_KEYS {
            map.serialize_entry(key.key(), &self.get_char(*key))?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for CharacteristicValues {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CharacteristicValuesVisitor;

        impl CharacteristicValuesVisitor {
            fn from_ordered_values<E>(
                values: [i32; Characteristic::COUNT],
            ) -> Result<CharacteristicValues, E>
            where
                E: DeError,
            {
                Ok(CharacteristicValues { values })
            }
        }

        impl<'de> Visitor<'de> for CharacteristicValuesVisitor {
            type Value = CharacteristicValues;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a characteristic map keyed by STR/CON/SIZ/DEX/APP/INT/POW/EDU, a legacy { values: [...] } object, or a legacy ordered array")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut values = [0; Characteristic::COUNT];
                for value in &mut values {
                    *value = seq
                        .next_element()?
                        .ok_or_else(|| DeError::invalid_length(Characteristic::COUNT, &self))?;
                }
                Self::from_ordered_values(values)
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut out = CharacteristicValues::default();
                let mut seen = [false; Characteristic::COUNT];
                let mut saw_legacy_values = false;

                while let Some(key) = map.next_key::<String>()? {
                    if key == "values" {
                        if saw_legacy_values {
                            return Err(DeError::duplicate_field("values"));
                        }
                        if seen.iter().any(|value_seen| *value_seen) {
                            return Err(DeError::custom(
                                "legacy characteristic values object cannot be mixed with named characteristic fields",
                            ));
                        }

                        let values = map.next_value::<[i32; Characteristic::COUNT]>()?;
                        out = Self::from_ordered_values(values)?;
                        saw_legacy_values = true;
                        continue;
                    }

                    if saw_legacy_values {
                        return Err(DeError::custom(
                            "legacy characteristic values object cannot be mixed with named characteristic fields",
                        ));
                    }

                    let characteristic =
                        Characteristic::from_key(key.as_str()).ok_or_else(|| {
                            DeError::unknown_field(key.as_str(), CHARACTERISTIC_SAVE_FIELD_NAMES)
                        })?;
                    let index = characteristic.index();
                    if seen[index] {
                        return Err(DeError::duplicate_field(characteristic.key()));
                    }
                    seen[index] = true;
                    let value = map.next_value::<i32>()?;
                    out.set_char(characteristic, value);
                }

                if !saw_legacy_values {
                    for key in CHARACTERISTIC_SAVE_KEYS {
                        if !seen[key.index()] {
                            return Err(DeError::missing_field(key.key()));
                        }
                    }
                }

                Ok(out)
            }
        }

        deserializer.deserialize_any(CharacteristicValuesVisitor)
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

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
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

    pub(crate) fn custom_label_hint(self) -> Option<&'static str> {
        match self {
            Self::ArtCraft
            | Self::ArtCraftActing
            | Self::ArtCraftFineArt
            | Self::ArtCraftForgery
            | Self::ArtCraftLiterature
            | Self::ArtCraftPhotography
            | Self::ArtCraftTechnicalDrawing
            | Self::ArtCraftWriting => Some("Art/Craft (Photography)"),
            Self::FightingBrawl => Some("Fighting (Sword)"),
            Self::FirearmsHandgun | Self::FirearmsRifleShotgun => Some("Firearms (SMG)"),
            Self::LanguageOther => Some("Language (Latin)"),
            Self::Pilot => Some("Pilot (Boat)"),
            Self::ScienceAstronomy
            | Self::ScienceBiology
            | Self::ScienceBotany
            | Self::ScienceChemistry
            | Self::ScienceCryptography
            | Self::ScienceEngineering
            | Self::ScienceForensics
            | Self::ScienceGeology
            | Self::ScienceMathematics
            | Self::SciencePharmacy
            | Self::SciencePhysics
            | Self::ScienceZoology => Some("Science (Astrobiology)"),
            Self::Survival => Some("Survival (Desert)"),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
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
    Skill(Skill),
    Choice {
        id: String,
        label: String,
        options: Vec<Skill>,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DiceResult {
    pub(crate) rolls: Vec<u32>,
    pub(crate) plus_six: bool,
    pub(crate) value: i32,
    pub(crate) kept: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

fn default_custom_occupation_required_skill_count() -> usize {
    CUSTOM_OCCUPATION_SKILL_COUNT
}

fn parse_usize_label_map(
    raw: &serde_json::Map<String, serde_json::Value>,
) -> BTreeMap<usize, String> {
    raw.iter()
        .filter_map(|(index, label)| {
            let label = label.as_str()?;
            index
                .parse::<usize>()
                .ok()
                .map(|index| (index, label.to_owned()))
        })
        .collect()
}

fn parse_string_label_map(
    raw: &serde_json::Map<String, serde_json::Value>,
) -> BTreeMap<String, String> {
    raw.iter()
        .filter_map(|(skill, label)| {
            label
                .as_str()
                .map(|label| (skill.clone(), label.to_owned()))
        })
        .collect()
}

fn deserialize_skill_labels<'de, D>(deserializer: D) -> Result<BTreeMap<String, String>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = serde_json::Value::deserialize(deserializer)?;
    let Some(object) = raw.as_object() else {
        return Ok(BTreeMap::new());
    };
    Ok(parse_string_label_map(object))
}

fn deserialize_skill_slot_labels<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<usize, String>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = serde_json::Value::deserialize(deserializer)?;
    let Some(object) = raw.as_object() else {
        return Ok(BTreeMap::new());
    };
    Ok(parse_usize_label_map(object))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CustomOccupation {
    pub(crate) name: String,
    pub(crate) credit_min: i32,
    pub(crate) credit_max: i32,
    pub(crate) formula_key: FormulaKey,
    #[serde(default = "default_custom_occupation_required_skill_count")]
    pub(crate) required_skill_count: usize,
    // Legacy v1 custom skill labels keyed by canonical skill name.
    // New saves prefer `skill_slot_labels` so duplicate specialties can be
    // represented as independent custom occupation skill instances.
    #[serde(default, deserialize_with = "deserialize_skill_labels")]
    pub(crate) skill_labels: BTreeMap<String, String>,
    #[serde(default, deserialize_with = "deserialize_skill_slot_labels")]
    pub(crate) skill_slot_labels: BTreeMap<usize, String>,
    pub(crate) skills: Vec<String>,
}

impl Default for CustomOccupation {
    fn default() -> Self {
        Self {
            name: "Custom Occupation".to_owned(),
            credit_min: 9,
            credit_max: 60,
            formula_key: FormulaKey::Edu4,
            required_skill_count: CUSTOM_OCCUPATION_SKILL_COUNT,
            skill_labels: BTreeMap::new(),
            skill_slot_labels: BTreeMap::new(),
            skills: vec![String::new(); CUSTOM_OCCUPATION_SKILL_COUNT],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct EduCheckRoll {
    pub(crate) d100: i32,
    pub(crate) improved: bool,
    pub(crate) gain: i32,
    pub(crate) resulting_edu: i32,
}

#[derive(Clone, Debug)]
pub(crate) struct SkillRow {
    pub(crate) id: Skill,
    pub(crate) custom_index: Option<usize>,
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
    pub(crate) occupation_skill_set: HashSet<Skill>,
    pub(crate) occupation_budget: i32,
    pub(crate) personal_budget: i32,
    pub(crate) credit_rating: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LuckState {
    pub(crate) value: Option<i32>,
    pub(crate) rolls: Vec<DiceResult>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct AllocationState {
    pub(crate) occupation_points: HashMap<Skill, i32>,
    pub(crate) personal_points: HashMap<Skill, i32>,
    pub(crate) custom_occupation_points: HashMap<usize, i32>,
    pub(crate) custom_personal_points: HashMap<usize, i32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SanitizeReport {
    pub(crate) removed_allocations: Vec<String>,
    pub(crate) clamped_allocations: Vec<String>,
    pub(crate) removed_unknown_import_entries: Vec<String>,
    pub(crate) clamped_characteristics: Vec<String>,
    pub(crate) removed_characteristic_rolls: Vec<String>,
    pub(crate) reset_luck: bool,
    pub(crate) normalized_luck: bool,
    pub(crate) normalized_edu_checks: bool,
    pub(crate) normalized_age_deductions: bool,
    pub(crate) removed_backstory_categories: Vec<String>,
    pub(crate) normalized_formula: bool,
    pub(crate) removed_occupation_choices: Vec<String>,
    pub(crate) normalized_custom_occupation: bool,
    pub(crate) normalized_rng_state: bool,
    pub(crate) normalized_import_fields: Vec<String>,
}

impl SanitizeReport {
    pub(crate) fn is_clean(&self) -> bool {
        self.removed_allocations.is_empty()
            && self.clamped_allocations.is_empty()
            && self.removed_unknown_import_entries.is_empty()
            && self.clamped_characteristics.is_empty()
            && self.removed_characteristic_rolls.is_empty()
            && !self.reset_luck
            && !self.normalized_luck
            && !self.normalized_edu_checks
            && !self.normalized_age_deductions
            && self.removed_backstory_categories.is_empty()
            && !self.normalized_formula
            && self.removed_occupation_choices.is_empty()
            && !self.normalized_custom_occupation
            && !self.normalized_rng_state
            && self.normalized_import_fields.is_empty()
    }

    pub(crate) fn summary(&self) -> String {
        let mut parts = Vec::new();
        if !self.clamped_allocations.is_empty() {
            parts.push(format!(
                "clamped {} allocation{}",
                self.clamped_allocations.len(),
                if self.clamped_allocations.len() == 1 {
                    ""
                } else {
                    "s"
                }
            ));
        }
        if !self.removed_allocations.is_empty() {
            parts.push(format!(
                "removed {} allocation{}",
                self.removed_allocations.len(),
                if self.removed_allocations.len() == 1 {
                    ""
                } else {
                    "s"
                }
            ));
        }
        if !self.removed_unknown_import_entries.is_empty() {
            let mut details = self
                .removed_unknown_import_entries
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>();
            if self.removed_unknown_import_entries.len() > details.len() {
                details.push(format!(
                    "{} more",
                    self.removed_unknown_import_entries.len() - details.len()
                ));
            }
            parts.push(format!(
                "removed invalid import entries: {}",
                details.join("; ")
            ));
        }
        if !self.clamped_characteristics.is_empty() {
            parts.push(format!(
                "clamped {} characteristic{}",
                self.clamped_characteristics.len(),
                if self.clamped_characteristics.len() == 1 {
                    ""
                } else {
                    "s"
                }
            ));
        }
        if !self.removed_characteristic_rolls.is_empty() {
            parts.push(format!(
                "removed {} stale characteristic roll{}",
                self.removed_characteristic_rolls.len(),
                if self.removed_characteristic_rolls.len() == 1 {
                    ""
                } else {
                    "s"
                }
            ));
        }
        if self.reset_luck {
            parts.push("reset invalid Luck roll evidence".to_owned());
        } else if self.normalized_luck {
            parts.push("normalized Luck roll evidence".to_owned());
        }
        if self.normalized_edu_checks {
            parts.push("normalized EDU age checks".to_owned());
        }
        if self.normalized_age_deductions {
            parts.push("normalized age deductions".to_owned());
        }
        if !self.removed_backstory_categories.is_empty() {
            parts.push(format!(
                "removed {} backstory entr{}",
                self.removed_backstory_categories.len(),
                if self.removed_backstory_categories.len() == 1 {
                    "y"
                } else {
                    "ies"
                }
            ));
        }
        if self.normalized_formula {
            parts.push("normalized occupation formula".to_owned());
        }
        if !self.removed_occupation_choices.is_empty() {
            parts.push(format!(
                "removed {} occupation choice{}",
                self.removed_occupation_choices.len(),
                if self.removed_occupation_choices.len() == 1 {
                    ""
                } else {
                    "s"
                }
            ));
        }
        if self.normalized_custom_occupation {
            parts.push("normalized custom occupation".to_owned());
        }
        if self.normalized_rng_state {
            parts.push("normalized RNG state".to_owned());
        }
        if !self.normalized_import_fields.is_empty() {
            let mut details = self
                .normalized_import_fields
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>();
            if self.normalized_import_fields.len() > details.len() {
                details.push(format!(
                    "{} more",
                    self.normalized_import_fields.len() - details.len()
                ));
            }
            parts.push(format!(
                "normalized imported fields: {}",
                details.join("; ")
            ));
        }
        if parts.is_empty() {
            "no corrections".to_owned()
        } else {
            parts.join(", ")
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SerializableAllocationState {
    #[serde(default, deserialize_with = "deserialize_allocation_map")]
    occupation_points: BTreeMap<String, serde_json::Value>,
    #[serde(default, deserialize_with = "deserialize_allocation_map")]
    personal_points: BTreeMap<String, serde_json::Value>,
    #[serde(default, deserialize_with = "deserialize_allocation_map")]
    custom_occupation_points: BTreeMap<String, serde_json::Value>,
    #[serde(default, deserialize_with = "deserialize_allocation_map")]
    custom_personal_points: BTreeMap<String, serde_json::Value>,
}

fn deserialize_allocation_map<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, serde_json::Value>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = serde_json::Value::deserialize(deserializer)?;
    let Some(object) = raw.as_object() else {
        return Ok(BTreeMap::new());
    };
    Ok(object
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect())
}

fn parse_allocation_value(value: &serde_json::Value) -> Option<i32> {
    value.as_i64().and_then(|value| i32::try_from(value).ok())
}

fn parse_skill_allocation_points(raw: BTreeMap<String, serde_json::Value>) -> HashMap<Skill, i32> {
    raw.into_iter()
        .filter_map(|(skill, value)| Skill::from_name(&skill).zip(parse_allocation_value(&value)))
        .collect()
}

fn parse_custom_allocation_points(raw: BTreeMap<String, serde_json::Value>) -> HashMap<usize, i32> {
    raw.into_iter()
        .filter_map(|(index, value)| {
            index
                .parse::<usize>()
                .ok()
                .zip(parse_allocation_value(&value))
        })
        .collect()
}

impl Serialize for AllocationState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let raw = SerializableAllocationState {
            occupation_points: self
                .occupation_points
                .iter()
                .map(|(skill, value)| (skill.name().to_owned(), serde_json::Value::from(*value)))
                .collect(),
            personal_points: self
                .personal_points
                .iter()
                .map(|(skill, value)| (skill.name().to_owned(), serde_json::Value::from(*value)))
                .collect(),
            custom_occupation_points: self
                .custom_occupation_points
                .iter()
                .map(|(index, value)| (index.to_string(), serde_json::Value::from(*value)))
                .collect(),
            custom_personal_points: self
                .custom_personal_points
                .iter()
                .map(|(index, value)| (index.to_string(), serde_json::Value::from(*value)))
                .collect(),
        };
        raw.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AllocationState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = SerializableAllocationState::deserialize(deserializer)?;
        Ok(Self {
            occupation_points: parse_skill_allocation_points(raw.occupation_points),
            personal_points: parse_skill_allocation_points(raw.personal_points),
            custom_occupation_points: parse_custom_allocation_points(raw.custom_occupation_points),
            custom_personal_points: parse_custom_allocation_points(raw.custom_personal_points),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SavedOccupationChoice {
    pub(crate) id: String,
    pub(crate) index: usize,
    pub(crate) value: String,
}

fn deserialize_rng_roll_sides<'de, D>(deserializer: D) -> Result<Vec<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = serde_json::Value::deserialize(deserializer)?;
    let Some(items) = raw.as_array() else {
        return Ok(vec![0]);
    };

    Ok(items
        .iter()
        .map(|item| {
            item.as_u64()
                .and_then(|side| u32::try_from(side).ok())
                .unwrap_or(0)
        })
        .collect())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct InvestigatorSaveFile {
    pub(crate) version: u32,
    pub(crate) concept: Concept,
    pub(crate) char_method: CharMethod,
    pub(crate) chars: CharacteristicValues,
    pub(crate) char_rolls: BTreeMap<String, DiceResult>,
    #[serde(default)]
    pub(crate) rng_seed: u64,
    #[serde(default, deserialize_with = "deserialize_rng_roll_sides")]
    pub(crate) rng_roll_sides: Vec<u32>,
    pub(crate) luck_state: LuckState,
    pub(crate) age_deductions: CharacteristicValues,
    pub(crate) edu_bonus: i32,
    pub(crate) edu_check_rolls: Vec<EduCheckRoll>,
    pub(crate) occupation_id: String,
    pub(crate) formula_key: FormulaKey,
    pub(crate) occupation_choices: Vec<SavedOccupationChoice>,
    pub(crate) custom_occupation: CustomOccupation,
    pub(crate) allocations: AllocationState,
    pub(crate) backstory: BTreeMap<String, String>,
}

#[derive(Debug)]
pub(crate) struct CoC7eApp {
    pub(super) step: usize,
    pub(super) concept: Concept,
    pub(super) char_method: CharMethod,
    pub(super) chars: CharacteristicValues,
    pub(super) char_rolls: HashMap<String, DiceResult>,
    pub(super) luck_state: LuckState,
    pub(super) age_deductions: CharacteristicValues,
    pub(super) edu_bonus: i32,
    pub(super) edu_check_rolls: Vec<EduCheckRoll>,
    pub(super) occupation_id: String,
    pub(super) formula_key: FormulaKey,
    pub(super) occupation_choices: HashMap<ChoiceKey, String>,
    pub(super) custom_occupation: CustomOccupation,
    pub(super) allocations: AllocationState,
    pub(super) backstory: HashMap<String, String>,
    pub(super) import_json_text: String,
    pub(super) save_load_path: String,
    pub(super) save_load_message: Option<String>,
    pub(super) occupations: Vec<Occupation>,
    pub(super) startup_validation_errors: Vec<String>,
    pub(super) last_age_bracket_index: usize,
    pub(super) frame_max_reachable_step: usize,
    pub(super) rng_seed: u64,
    pub(super) rng_roll_sides: Vec<u32>,
    pub(super) rng: AppRng,
}
