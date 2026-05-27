use super::super::data::*;
use super::super::models::*;
use super::super::occupations::*;
use super::super::ruleset::*;
use eframe::egui;
use egui::RichText;
use std::collections::HashSet;

impl CoC7eApp {
    pub(crate) fn render_occupation(&mut self, ui: &mut egui::Ui) {
        heading(
            ui,
            "III. Occupation",
            "Choose an occupation, resolve choice slots, or define a Keeper-approved custom occupation with eight unique occupation skills.",
        );

        if self.occupation_id == CUSTOM_OCCUPATION_ID {
            self.normalize_custom_occupation_skills();
        }

        card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Occupation").small().color(MUTED).strong());
                    let mut next = self.occupation_id.clone();
                    egui::ComboBox::from_id_salt("occupation_select")
                        .selected_text(if next.is_empty() {
                            "— Choose —".to_owned()
                        } else if next == CUSTOM_OCCUPATION_ID {
                            "Custom Occupation".to_owned()
                        } else {
                            next.clone()
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut next, String::new(), "— Choose —");
                            ui.selectable_value(
                                &mut next,
                                CUSTOM_OCCUPATION_ID.to_owned(),
                                "Custom Occupation",
                            );
                            let mut names: Vec<&str> = self
                                .occupations
                                .iter()
                                .map(|occ| occ.name.as_str())
                                .collect();
                            names.sort_unstable();
                            for name in names {
                                ui.selectable_value(&mut next, name.to_owned(), name);
                            }
                        });
                    if next != self.occupation_id {
                        self.set_occupation(next);
                    }
                });

                if self.occupation_id != CUSTOM_OCCUPATION_ID
                    && let Some(occupation) = self.selected_occupation()
                {
                    self.normalize_formula_key_for(Some(&occupation));
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Occupation skill point formula")
                                .small()
                                .color(MUTED)
                                .strong(),
                        );
                        let mut next_formula = self.formula_key;
                        egui::ComboBox::from_id_salt("formula_select")
                            .selected_text(next_formula.label())
                            .show_ui(ui, |ui| {
                                for formula in occupation.formula_keys {
                                    ui.selectable_value(
                                        &mut next_formula,
                                        formula,
                                        formula.label(),
                                    );
                                }
                            });
                        if self.formula_key != next_formula {
                            self.set_formula_key(next_formula);
                        }
                    });
                }
            });
        });

        if self.occupation_id == CUSTOM_OCCUPATION_ID {
            self.render_custom_occupation(ui);
        }

        if let Some(occupation) = self.selected_occupation() {
            self.prune_occupation_choices_for(&occupation);
        }

        let math = self.sheet_math();
        if let Some(occupation) = math.selected_occupation.as_ref() {
            let unresolved = math.unresolved_choices;
            let shortfall = math.occupation_shortfall;
            let resolved = self.resolved_occupation_skills_for(occupation);
            let (credit_min, credit_max) = math.credit_range;
            let occ_budget = math.occupation_budget;
            let personal_budget = math.personal_budget;
            let used_occ = CoC7eApp::used_occupation_points_from(&math.skill_rows);
            let fixed = fixed_skill_set_for(occupation);

            card(ui, |ui| {
                ui.label(
                    RichText::new(self.selected_occupation_name())
                        .size(17.0)
                        .strong(),
                );
                ui.horizontal_wrapped(|ui| {
                    pill(
                        ui,
                        format!("Occupation {used_occ} / {occ_budget}"),
                        if used_occ > occ_budget { RED } else { MUTED },
                    );
                    pill(
                        ui,
                        format!("Personal interest: INT × 2 = {personal_budget}"),
                        GREEN,
                    );
                    pill(
                        ui,
                        format!("Credit Rating {credit_min}–{credit_max}"),
                        AMBER,
                    );
                    if unresolved > 0 {
                        pill(
                            ui,
                            format!(
                                "{unresolved} choice slot{} unresolved",
                                if unresolved > 1 { "s" } else { "" }
                            ),
                            RED,
                        );
                    } else {
                        pill(ui, "Choices resolved", GREEN);
                    }
                    if shortfall > 0 && unresolved == 0 {
                        pill(
                            ui,
                            format!(
                                "Need {shortfall} more unique occupation skill{}",
                                if shortfall > 1 { "s" } else { "" }
                            ),
                            AMBER,
                        );
                    }
                });

                ui.add_space(10.0);
                for slot in &occupation.slots {
                    match slot {
                        Slot::Skill(skill) => {
                            pill(ui, skill.name(), MUTED);
                            ui.add_space(4.0);
                        }
                        Slot::Choice {
                            id,
                            label,
                            options,
                            count,
                        } => self.render_choice_slot(ui, id, label, options, *count, &fixed),
                    }
                }

                ui.separator();
                ui.label(
                    RichText::new("Resolved occupation skills")
                        .small()
                        .color(MUTED)
                        .strong(),
                );
                ui.horizontal_wrapped(|ui| {
                    if resolved.is_empty() {
                        ui.label(
                            RichText::new("No skills resolved yet.")
                                .small()
                                .color(MUTED),
                        );
                    } else {
                        for skill in resolved {
                            pill(ui, skill.name(), ACCENT);
                        }
                    }
                    pill(ui, "Credit Rating", AMBER);
                });
            });
        }

        self.navigation(ui);
    }

    pub(crate) fn render_custom_occupation(&mut self, ui: &mut egui::Ui) {
        self.normalize_custom_occupation_skills();

        card(ui, |ui| {
            ui.label(
                RichText::new("Custom occupation builder")
                    .size(16.0)
                    .strong(),
            );
            egui::Grid::new("custom_occ_grid")
                .num_columns(2)
                .spacing([16.0, 10.0])
                .show(ui, |ui| {
                    let mut custom_name = self.custom_occupation.name.clone();
                    labeled_text(ui, "Occupation name", &mut custom_name, "Custom Occupation");
                    if custom_name != self.custom_occupation.name {
                        self.set_custom_occupation_name(custom_name);
                    }
                    let mut credit_min = self.custom_occupation.credit_min;
                    if labeled_i32(ui, "Credit min", &mut credit_min, 0, 99, 1.0).changed() {
                        self.set_custom_occupation_credit_min(credit_min);
                    }
                    ui.end_row();
                    let mut credit_max = self.custom_occupation.credit_max;
                    if labeled_i32(ui, "Credit max", &mut credit_max, 0, 99, 1.0).changed() {
                        self.set_custom_occupation_credit_max(credit_max);
                    }
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Formula").small().color(MUTED).strong());
                        let mut next = self.custom_occupation.formula_key;
                        egui::ComboBox::from_id_salt("custom_formula")
                            .selected_text(next.label())
                            .show_ui(ui, |ui| {
                                for formula in ALL_FORMULAS {
                                    ui.selectable_value(&mut next, *formula, formula.label());
                                }
                            });
                        if next != self.custom_occupation.formula_key {
                            self.set_custom_formula_key(next);
                        }
                    });
                    ui.end_row();
                });

            ui.add_space(8.0);
            ui.label(
                RichText::new(format!(
                    "Choose {CUSTOM_OCCUPATION_SKILL_COUNT} occupation skills"
                ))
                .small()
                .color(MUTED)
                .strong(),
            );
            if self.custom_occupation.credit_min > self.custom_occupation.credit_max {
                ui.label(RichText::new("Credit min is greater than credit max; the generated range will be normalized until corrected.").small().color(AMBER));
            }

            let selected: HashSet<String> = self
                .custom_occupation
                .skills
                .iter()
                .take(CUSTOM_OCCUPATION_SKILL_COUNT)
                .map(|skill| skill.trim())
                .filter(|skill| !skill.is_empty())
                .map(str::to_owned)
                .collect();
            let options = occupation_selectable_skills();
            egui::Grid::new("custom_skills_grid")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    for index in 0..CUSTOM_OCCUPATION_SKILL_COUNT {
                        let current_trimmed =
                            self.custom_occupation.skills[index].trim().to_owned();
                        let mut next = current_trimmed.clone();
                        egui::ComboBox::from_id_salt(format!("custom_skill_{index}"))
                            .selected_text(if next.is_empty() {
                                format!("— Skill {} —", index + 1)
                            } else {
                                next.clone()
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut next,
                                    String::new(),
                                    format!("— Skill {} —", index + 1),
                                );
                                for option in options {
                                    let unavailable =
                                        selected.contains(*option) && *option != current_trimmed;
                                    ui.add_enabled_ui(!unavailable, |ui| {
                                        ui.selectable_value(
                                            &mut next,
                                            (*option).to_owned(),
                                            *option,
                                        );
                                    });
                                }
                            });

                        let normalized = next.trim().to_owned();
                        if normalized != self.custom_occupation.skills[index] {
                            self.set_custom_occupation_skill(index, normalized);
                        }

                        if index % 2 == 1 {
                            ui.end_row();
                        }
                    }
                });
        });
    }

    pub(crate) fn render_choice_slot(
        &mut self,
        ui: &mut egui::Ui,
        id: &str,
        label: &str,
        options: &[Skill],
        count: usize,
        fixed: &HashSet<Skill>,
    ) {
        ui.label(
            RichText::new(if count > 1 {
                format!("{label} × {count}")
            } else {
                label.to_owned()
            })
            .small()
            .color(MUTED)
            .strong(),
        );
        ui.horizontal_wrapped(|ui| {
            for index in 0..count {
                let key = ChoiceKey::new(id.to_owned(), index);
                let current = self
                    .occupation_choices
                    .get(&key)
                    .cloned()
                    .unwrap_or_default();
                let current_trimmed = current.trim().to_owned();
                let mut next = current_trimmed.clone();
                let chosen_elsewhere: HashSet<String> = self
                    .occupation_choices
                    .iter()
                    .filter(|(choice_key, value)| *choice_key != &key && !value.trim().is_empty())
                    .map(|(_, value)| value.trim().to_owned())
                    .collect();

                egui::ComboBox::from_id_salt(key.widget_id())
                    .selected_text(if next.is_empty() {
                        "— Choose —".to_owned()
                    } else {
                        next.clone()
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut next, String::new(), "— Choose —");
                        for option in options {
                            let option_name = option.name();
                            let unavailable = (fixed.contains(option)
                                || chosen_elsewhere.contains(option_name))
                                && option_name != current_trimmed;
                            ui.add_enabled_ui(!unavailable, |ui| {
                                ui.selectable_value(&mut next, option_name.to_owned(), option_name);
                            });
                        }
                    });

                let normalized = next.trim().to_owned();
                if normalized != current {
                    self.set_occupation_choice(key, normalized);
                }
            }
        });
        ui.add_space(8.0);
    }
}
