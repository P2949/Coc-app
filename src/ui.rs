use super::data::*;
use super::models::*;
use super::occupations::*;
use super::ruleset::*;
use eframe::egui;
use egui::RichText;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "ui/characteristics.rs"]
mod characteristics;
#[path = "ui/concept.rs"]
mod concept;
#[path = "ui/occupation.rs"]
mod occupation;
#[path = "ui/skills.rs"]
mod skills;
#[path = "ui/story_summary.rs"]
mod story_summary;

pub fn run() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([APP_INITIAL_WIDTH, APP_INITIAL_HEIGHT])
            .with_min_inner_size([APP_MIN_WINDOW_WIDTH, APP_MIN_WINDOW_HEIGHT]),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "CoC7e Investigator Creator",
        options,
        Box::new(|cc| Ok(Box::new(CoC7eApp::new(cc)))),
    )
}

impl CoC7eApp {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_dark_theme(&cc.egui_ctx);

        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(DEFAULT_RNG_SEED);

        let occupations = build_occupations();
        let mut startup_validation_errors = skill_constant_validation_errors();
        startup_validation_errors.extend(occupation_validation_errors(&occupations));

        let mut app = Self::fresh(occupations, seed);
        app.startup_validation_errors = startup_validation_errors;
        app
    }

    pub(crate) fn fresh(occupations: Vec<Occupation>, rng_state: u64) -> Self {
        let age_index = get_age_bracket_index(30);

        Self {
            step: 1,
            concept: Concept::default(),
            char_method: CharMethod::Roll,
            chars: CharacteristicValues::default(),
            char_rolls: HashMap::new(),
            luck_state: LuckState::default(),
            age_deductions: empty_deductions_for(AGE_BRACKETS[age_index]),
            edu_bonus: 0,
            edu_check_rolls: Vec::new(),
            occupation_id: String::new(),
            formula_key: FormulaKey::Edu4,
            occupation_choices: HashMap::new(),
            custom_occupation: CustomOccupation::default(),
            allocations: AllocationState::default(),
            backstory: HashMap::new(),
            import_json_text: String::new(),
            save_load_path: String::new(),
            save_load_message: None,
            occupations,
            startup_validation_errors: Vec::new(),
            last_age_bracket_index: age_index,
            frame_max_reachable_step: 2,
            rng_seed: rng_state,
            rng_roll_sides: Vec::new(),
            rng: AppRng::seeded(rng_state),
        }
    }

    pub(crate) fn reset_investigator(&mut self) {
        let occupations = std::mem::take(&mut self.occupations);
        let startup_validation_errors = std::mem::take(&mut self.startup_validation_errors);
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(DEFAULT_RNG_SEED);
        *self = Self::fresh(occupations, seed);
        self.startup_validation_errors = startup_validation_errors;
    }

    pub(crate) fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(12.0);
            ui.label(
                RichText::new("Call of Cthulhu 7th Edition · Unofficial helper")
                    .size(11.0)
                    .color(FAINT)
                    .strong(),
            );
            ui.label(
                RichText::new("Investigator Creator")
                    .size(32.0)
                    .color(TEXT)
                    .strong(),
            );
            ui.label(
                RichText::new("Unofficial fan-made rules-aware character creation helper")
                    .size(14.0)
                    .color(MUTED),
            );
            ui.add_space(10.0);
        });
    }

    pub(crate) fn step_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            let max_reachable = self.frame_max_reachable_step;
            for (index, label) in STEPS.iter().enumerate() {
                let step_num = index + 1;
                let selected = self.step == step_num;
                let enabled = step_num <= max_reachable;
                let text = if step_num < self.step {
                    format!("✓ {label}")
                } else {
                    format!("{step_num}. {label}")
                };
                let response = ui.selectable_label(
                    selected,
                    RichText::new(text).color(if selected {
                        ACCENT
                    } else if enabled {
                        MUTED
                    } else {
                        FAINT
                    }),
                );
                if enabled && response.clicked() {
                    self.step = step_num;
                }
            }
        });
        ui.add_space(10.0);
    }

    pub(crate) fn save_load_panel(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Save / Load JSON")
            .default_open(false)
            .show(ui, |ui| {
                card(ui, |ui| {
                    ui.label(
                        RichText::new(
                            "Copy a JSON save to preserve editable investigator state, or paste one here to load it back into the creator.",
                        )
                        .small()
                        .color(MUTED),
                    );
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Copy JSON save").clicked() {
                            match self.export_json_save() {
                                Ok(json) => {
                                    ui.ctx().copy_text(json);
                                    self.save_load_message =
                                        Some("Copied JSON save to clipboard.".to_owned());
                                }
                                Err(error) => {
                                    self.save_load_message =
                                        Some(format!("Could not build JSON save: {error}"));
                                }
                            }
                        }

                        let load_response = ui.add_enabled(
                            !self.import_json_text.trim().is_empty(),
                            egui::Button::new("Load pasted JSON"),
                        );
                        if load_response.clicked() {
                            let input = self.import_json_text.clone();
                            match self.import_json_save(&input) {
                                Ok(report) => {
                                    self.import_json_text.clear();
                                    self.save_load_message = Some(if report.is_clean() {
                                        "Loaded JSON save.".to_owned()
                                    } else {
                                        format!(
                                            "Loaded JSON save and corrected invalid data: {}.",
                                            report.summary()
                                        )
                                    });
                                }
                                Err(error) => {
                                    self.save_load_message = Some(error);
                                }
                            }
                        } else if self.import_json_text.trim().is_empty() {
                            load_response.on_hover_text("Paste a JSON save before loading.");
                        }
                    });
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("File path").small().color(MUTED).strong());
                        ui.add_sized(
                            [520.0, 26.0],
                            egui::TextEdit::singleline(&mut self.save_load_path)
                                .hint_text("/path/to/investigator.json"),
                        );
                        let path = PathBuf::from(self.save_load_path.trim());
                        let path_present = !self.save_load_path.trim().is_empty();

                        let save_response = ui.add_enabled(
                            path_present,
                            egui::Button::new("Save JSON to file"),
                        );
                        if save_response.clicked() {
                            match self.save_json_to_path(&path) {
                                Ok(()) => {
                                    self.save_load_message =
                                        Some(format!("Saved JSON to {}.", path.display()));
                                }
                                Err(error) => self.save_load_message = Some(error),
                            }
                        } else if !path_present {
                            save_response.on_hover_text("Enter a file path before saving.");
                        }

                        let load_file_response = ui.add_enabled(
                            path_present,
                            egui::Button::new("Load JSON from file"),
                        );
                        if load_file_response.clicked() {
                            match self.load_json_from_path(&path) {
                                Ok(report) => {
                                    self.import_json_text.clear();
                                    self.save_load_message = Some(if report.is_clean() {
                                        format!("Loaded JSON from {}.", path.display())
                                    } else {
                                        format!(
                                            "Loaded JSON from {} and corrected invalid data: {}.",
                                            path.display(),
                                            report.summary()
                                        )
                                    });
                                }
                                Err(error) => self.save_load_message = Some(error),
                            }
                        } else if !path_present {
                            load_file_response.on_hover_text("Enter a file path before loading.");
                        }
                    });
                    if let Some(message) = &self.save_load_message {
                        ui.label(RichText::new(message).small().color(AMBER));
                    }
                    ui.add_sized(
                        [ui.available_width(), 120.0],
                        egui::TextEdit::multiline(&mut self.import_json_text)
                            .hint_text("Paste JSON save here, then press Load JSON save"),
                    );
                });
            });
        ui.add_space(8.0);
    }

    pub(crate) fn navigation(&mut self, ui: &mut egui::Ui) {
        self.refresh_reachability();
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.step < STEPS.len()
                    && ui
                        .add_enabled(
                            self.step < self.frame_max_reachable_step,
                            egui::Button::new("Continue →"),
                        )
                        .clicked()
                {
                    self.step = (self.step + 1).min(STEPS.len());
                }
                if self.step > 1 && ui.button("← Back").clicked() {
                    self.step = self.step.saturating_sub(1).max(1);
                }
            });
        });
    }

    pub(crate) fn render_startup_validation_error(&self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            ui.label(
                RichText::new("Internal ruleset validation failed")
                    .size(22.0)
                    .color(RED)
                    .strong(),
            );
            ui.label(
                RichText::new(
                    "The built-in skill or occupation data is inconsistent, so normal character creation has been disabled instead of panicking during startup.",
                )
                .color(MUTED),
            );
            ui.add_space(8.0);
            for error in &self.startup_validation_errors {
                ui.label(RichText::new(format!("• {error}")).small().color(AMBER));
            }
        });
    }
}

impl eframe::App for CoC7eApp {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.startup_validation_errors.is_empty() {
            return;
        }

        self.sync_age_bracket();
        self.sanitize_state();
        self.refresh_reachability();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG))
            .show_inside(ui, |ui| {
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .min_scrolled_width(APP_CONTENT_WIDTH)
                    .show(ui, |ui| {
                        ui.set_min_width(APP_CONTENT_WIDTH);
                        ui.set_max_width(APP_CONTENT_WIDTH);

                        self.top_bar(ui);

                        if !self.startup_validation_errors.is_empty() {
                            self.render_startup_validation_error(ui);
                            return;
                        }

                        self.step_bar(ui);
                        self.save_load_panel(ui);

                        match self.step {
                            1 => self.render_concept(ui),
                            2 => self.render_characteristics(ui),
                            3 => self.render_occupation(ui),
                            4 => self.render_skills(ui),
                            5 => self.render_backstory(ui),
                            6 => self.render_summary(ui),
                            _ => self.step = 1,
                        }
                    });
            });
    }
}
