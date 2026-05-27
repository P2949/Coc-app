#[path = "allocations.rs"]
pub(crate) mod allocations;
#[path = "character_state.rs"]
pub(crate) mod character_state;
#[path = "data.rs"]
pub(crate) mod data;
#[path = "models.rs"]
pub(crate) mod models;
#[path = "occupation_state.rs"]
pub(crate) mod occupation_state;
#[path = "occupations.rs"]
pub(crate) mod occupations;
#[path = "quick_package.rs"]
pub(crate) mod quick_package;
#[path = "ruleset.rs"]
pub(crate) mod ruleset;
#[path = "save.rs"]
pub(crate) mod save;
#[path = "summary_rules.rs"]
pub(crate) mod summary_rules;
#[path = "ui.rs"]
pub(crate) mod ui;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub use self::ui::run;
