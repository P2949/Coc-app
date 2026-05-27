#[path = "allocations.rs"]
pub(crate) mod allocations;
#[path = "data.rs"]
pub(crate) mod data;
#[path = "models.rs"]
pub(crate) mod models;
#[path = "occupations.rs"]
pub(crate) mod occupations;
#[path = "ruleset.rs"]
pub(crate) mod ruleset;
#[path = "save.rs"]
pub(crate) mod save;
#[path = "ui.rs"]
pub(crate) mod ui;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub use self::ui::run;
