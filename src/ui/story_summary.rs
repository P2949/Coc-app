use super::super::data::*;
use super::super::models::*;
use super::super::occupations::backstory_hint;
use super::super::ruleset::*;
use eframe::egui;
use egui::RichText;

impl CoC7eApp {
    pub(crate) fn render_backstory(&mut self, ui: &mut egui::Ui) {
        heading(
            ui,
            "V. Backstory",
            "Short entries are usually more useful at the table than a full biography. Two or three strong entries are enough.",
        );

        for (index, category) in BACKSTORY_CATEGORIES.iter().enumerate() {
            card(ui, |ui| {
                ui.label(
                    RichText::new(format!("{}. {category}", index + 1))
                        .small()
                        .color(if index < 6 { MUTED } else { FAINT })
                        .strong(),
                );
                let mut entry = self.backstory.get(*category).cloned().unwrap_or_default();
                let response = ui.add_sized(
                    [ui.available_width(), 72.0],
                    egui::TextEdit::multiline(&mut entry).hint_text(backstory_hint(category)),
                );

                if response.changed() {
                    if entry.trim().is_empty() {
                        self.backstory.remove(*category);
                    } else {
                        self.backstory.insert((*category).to_owned(), entry);
                    }
                }
            });
        }

        self.navigation(ui);
    }

    pub(crate) fn render_summary(&mut self, ui: &mut egui::Ui) {
        heading(
            ui,
            "VI. Investigator Summary",
            "Review final values. “Final” characteristics include age adjustments.",
        );

        let occupation_name = self.selected_occupation_name();
        let math = self.sheet_math();
        let final_chars = &math.final_chars;
        let derived = &math.derived;
        let skill_rows = &math.skill_rows;
        let point_spent = self.point_buy_spent();
        let physical_total = self.physical_deduction_total();
        let bracket = self.age_bracket();
        let occ_budget = math.occupation_budget;
        let personal_budget = math.personal_budget;
        let used_occ = CoC7eApp::used_occupation_points_from(skill_rows);
        let occupation_capacity = CoC7eApp::occupation_budget_capacity_from(&math);
        let used_personal = CoC7eApp::used_personal_points_from(skill_rows);
        let credit = math.credit_rating;
        let (credit_min, credit_max) = math.credit_range;
        let has_occupation = math.selected_occupation.is_some();
        let unresolved = math.unresolved_choices;
        let shortfall = math.occupation_shortfall;
        let credit_out = has_occupation && (credit < credit_min || credit > credit_max);
        let skills_over = skill_rows.iter().any(|row| row.total > MAX_CREATION_VALUE);
        let age_deduction_label = if self.physical_deduction_is_possible() {
            format!(
                "Age deductions {physical_total}/{}",
                bracket.physical_deduct
            )
        } else {
            format!(
                "Age deductions {physical_total}/{} impossible; max {}",
                bracket.physical_deduct,
                self.max_possible_physical_deduction()
            )
        };
        let summary_blockers = self.summary_blockers_for(&math);
        let name = self.concept.name.trim();
        let pronouns = self.concept.pronouns.trim();
        let residence = self.concept.residence.trim();
        let birthplace = self.concept.birthplace.trim();

        card(ui, |ui| {
            ui.label(
                RichText::new(if name.is_empty() {
                    "Unnamed Investigator"
                } else {
                    name
                })
                .size(28.0)
                .strong()
                .color(TEXT),
            );
            let mut line = format!("{occupation_name} · Age {}", self.concept.age);
            if !pronouns.is_empty() {
                line.push_str(&format!(" · {pronouns}"));
            }
            ui.label(RichText::new(line).color(MUTED));
            let mut place = String::new();
            if !residence.is_empty() {
                place.push_str(&format!("Residence: {residence}"));
            }
            if !birthplace.is_empty() {
                if !place.is_empty() {
                    place.push_str(" · ");
                }
                place.push_str(&format!("Born: {birthplace}"));
            }
            if !place.is_empty() {
                ui.label(RichText::new(place).small().color(FAINT));
            }
        });

        card(ui, |ui| {
            ui.label(RichText::new("Rules checks").size(16.0).strong());
            ui.horizontal_wrapped(|ui| {
                rule_check(
                    ui,
                    if self.has_all_chars() {
                        CheckState::Pass
                    } else {
                        CheckState::Fail
                    },
                    format!(
                        "Characteristics {}",
                        if self.has_all_chars() {
                            "complete"
                        } else {
                            "missing"
                        }
                    ),
                );
                rule_check(
                    ui,
                    if physical_total == bracket.physical_deduct {
                        CheckState::Pass
                    } else {
                        CheckState::Fail
                    },
                    age_deduction_label,
                );
                rule_check(
                    ui,
                    if self.edu_age_checks_complete() {
                        CheckState::Pass
                    } else {
                        CheckState::Fail
                    },
                    if bracket.edu_checks == 0 {
                        "EDU age checks none".to_owned()
                    } else {
                        format!(
                            "EDU age checks {}/{}",
                            self.edu_check_rolls.len(),
                            bracket.edu_checks
                        )
                    },
                );
                rule_check(
                    ui,
                    if self.char_method != CharMethod::PointBuy {
                        CheckState::Warn
                    } else if point_spent == POINT_BUY_BUDGET {
                        CheckState::Pass
                    } else {
                        CheckState::Fail
                    },
                    format!("Point budget {point_spent}/{POINT_BUY_BUDGET}"),
                );
                rule_check(
                    ui,
                    if self.luck_state.value.is_some() {
                        CheckState::Pass
                    } else {
                        CheckState::Fail
                    },
                    format!(
                        "Luck {}",
                        self.luck_state
                            .value
                            .map_or("not rolled".to_owned(), |v| v.to_string())
                    ),
                );
                rule_check(
                    ui,
                    if has_occupation && unresolved == 0 && shortfall == 0 {
                        CheckState::Pass
                    } else {
                        CheckState::Fail
                    },
                    if !has_occupation {
                        "Occupation missing".to_owned()
                    } else if unresolved > 0 {
                        format!("Occupation choices unresolved: {unresolved}")
                    } else if shortfall > 0 {
                        format!("Occupation skills shortfall: {shortfall}")
                    } else {
                        "Occupation resolved".to_owned()
                    },
                );
                rule_check(
                    ui,
                    if used_occ == occ_budget {
                        CheckState::Pass
                    } else {
                        CheckState::Fail
                    },
                    if has_occupation && occ_budget > occupation_capacity {
                        format!(
                            "Occupation points impossible: cap {occupation_capacity}/{occ_budget}"
                        )
                    } else {
                        format!("Occupation points {used_occ}/{occ_budget}")
                    },
                );
                rule_check(
                    ui,
                    if used_personal == personal_budget {
                        CheckState::Pass
                    } else {
                        CheckState::Fail
                    },
                    format!("Personal points {used_personal}/{personal_budget}"),
                );
                rule_check(
                    ui,
                    if !credit_out {
                        CheckState::Pass
                    } else {
                        CheckState::Fail
                    },
                    format!("Credit Rating {credit}%"),
                );
                rule_check(
                    ui,
                    if !skills_over {
                        CheckState::Pass
                    } else {
                        CheckState::Fail
                    },
                    "Skill cap 99",
                );
            });
        });

        card(ui, |ui| {
            ui.label(RichText::new("Characteristics").size(16.0).strong());
            egui::Grid::new("summary_chars")
                .num_columns(4)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    for (index, def) in CHARACTERISTICS.iter().enumerate() {
                        let value = final_chars.get_char(def.key);
                        stat_box(
                            ui,
                            def.key.key(),
                            if value > 0 {
                                value.to_string()
                            } else {
                                "—".to_owned()
                            },
                            TEXT,
                        );
                        if index % 4 == 3 {
                            ui.end_row();
                        }
                    }
                });
        });

        card(ui, |ui| {
            ui.label(RichText::new("Derived Attributes").size(16.0).strong());
            egui::Grid::new("summary_derived")
                .num_columns(4)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    stat_box(ui, "HP", derived.hp, RED);
                    stat_box(
                        ui,
                        "Major Wound",
                        if derived.major_wound > 0 {
                            derived.major_wound.to_string()
                        } else {
                            "—".to_owned()
                        },
                        RED,
                    );
                    stat_box(ui, "SAN", derived.san, ACCENT);
                    stat_box(ui, "Max SAN (99−Mythos)", derived.max_san, ACCENT);
                    ui.end_row();
                    stat_box(ui, "MP", derived.mp, BLUE);
                    stat_box(
                        ui,
                        "Luck",
                        self.luck_state
                            .value
                            .map_or("—".to_owned(), |v| v.to_string()),
                        AMBER,
                    );
                    stat_box(ui, "MOV", derived.mov, GREEN);
                    stat_box(ui, "Dodge", format!("{}%", derived.dodge), AMBER);
                    ui.end_row();
                    stat_box(ui, "DB", &derived.db, AMBER);
                    stat_box(
                        ui,
                        "Build",
                        if derived.build >= 0 {
                            format!("+{}", derived.build)
                        } else {
                            derived.build.to_string()
                        },
                        AMBER,
                    );
                    ui.end_row();
                });
            ui.label(
                RichText::new(
                    "Max SAN starts at 99 and decreases as Cthulhu Mythos increases during play.",
                )
                .small()
                .color(FAINT),
            );
        });

        card(ui, |ui| {
            ui.label(RichText::new("Credit Rating").size(16.0).strong());
            ui.horizontal_wrapped(|ui| {
                pill(ui, format!("{credit}%"), AMBER);
                pill(ui, get_credit_tier(credit), MUTED);
                if has_occupation {
                    pill(
                        ui,
                        format!("Occupation range {credit_min}–{credit_max}"),
                        if credit_out { RED } else { GREEN },
                    );
                }
            });
        });

        card(ui, |ui| {
            ui.label(RichText::new("Skills").size(16.0).strong());
            let final_skills: Vec<_> = skill_rows
                .iter()
                .filter(|row| row.total > row.base || SUMMARY_ALWAYS_SHOW.contains(&row.id.name()))
                .collect();
            egui::Grid::new("summary_skills")
                .num_columns(4)
                .spacing([20.0, 6.0])
                .show(ui, |ui| {
                    for (index, row) in final_skills.iter().enumerate() {
                        ui.label(RichText::new(&row.name).color(TEXT));
                        ui.label(
                            RichText::new(format!(
                                "{}% ({}/{})",
                                row.total,
                                floor_half(row.total),
                                floor_fifth(row.total)
                            ))
                            .monospace()
                            .strong()
                            .color(
                                if row.total > MAX_CREATION_VALUE {
                                    RED
                                } else if row.total >= 70 {
                                    ACCENT
                                } else if row.total >= 50 {
                                    GREEN
                                } else {
                                    MUTED
                                },
                            ),
                        );
                        if index % 2 == 1 {
                            ui.end_row();
                        }
                    }
                });
        });

        if self
            .backstory
            .values()
            .any(|value| !value.trim().is_empty())
        {
            card(ui, |ui| {
                ui.label(RichText::new("Backstory").size(16.0).strong());
                for category in BACKSTORY_CATEGORIES {
                    if let Some(value) = self
                        .backstory
                        .get(*category)
                        .filter(|v| !v.trim().is_empty())
                    {
                        ui.add_space(6.0);
                        ui.label(RichText::new(*category).small().monospace().color(ACCENT));
                        ui.label(RichText::new(value.trim()).color(TEXT));
                    }
                }
            });
        }

        ui.horizontal_wrapped(|ui| {
            let copy_response = ui.add_enabled(
                summary_blockers.is_empty(),
                egui::Button::new("Copy plaintext summary"),
            );
            if copy_response.clicked() {
                ui.ctx().copy_text(self.plaintext_summary());
            }
            if !summary_blockers.is_empty() {
                copy_response.on_hover_text(format!(
                    "Resolve these first: {}",
                    summary_blockers.join(", ")
                ));
                ui.label(
                    RichText::new(format!(
                        "Copy is disabled until resolved: {}",
                        summary_blockers.join(", ")
                    ))
                    .small()
                    .color(AMBER),
                );
            }
            if ui.button("← Back to concept").clicked() {
                self.step = 1;
            }
            if ui.button("New investigator").clicked() {
                self.reset_investigator();
            }
        });
    }

    pub(crate) fn plaintext_summary(&self) -> String {
        let math = self.sheet_math();
        let final_chars = &math.final_chars;
        let derived = &math.derived;
        let skill_rows = &math.skill_rows;
        let mut out = String::new();
        let name = self.concept.name.trim();
        let pronouns = self.concept.pronouns.trim();
        let residence = self.concept.residence.trim();
        let birthplace = self.concept.birthplace.trim();

        push_line(
            &mut out,
            if name.is_empty() {
                "Unnamed Investigator"
            } else {
                name
            },
        );
        push_line(
            &mut out,
            format!("Occupation: {}", self.selected_occupation_name()),
        );
        push_line(&mut out, format!("Age: {}", self.concept.age));
        if !pronouns.is_empty() {
            push_line(&mut out, format!("Pronouns/Gender: {pronouns}"));
        }
        if !residence.is_empty() {
            push_line(&mut out, format!("Residence: {residence}"));
        }
        if !birthplace.is_empty() {
            push_line(&mut out, format!("Birthplace: {birthplace}"));
        }

        push_blank_line(&mut out);
        push_line(&mut out, "Characteristics");
        for def in CHARACTERISTICS {
            let value = final_chars.get_char(def.key);
            push_line(
                &mut out,
                format!(
                    "{}: {} ({}/{})",
                    def.key.key(),
                    value,
                    floor_half(value),
                    floor_fifth(value)
                ),
            );
        }

        push_blank_line(&mut out);
        push_line(&mut out, "Derived Attributes");
        push_line(&mut out, format!("HP: {}", derived.hp));
        push_line(&mut out, format!("Major Wound: {}", derived.major_wound));
        push_line(&mut out, format!("SAN: {}", derived.san));
        push_line(&mut out, format!("Max SAN: {}", derived.max_san));
        push_line(&mut out, format!("MP: {}", derived.mp));
        push_line(
            &mut out,
            format!(
                "Luck: {}",
                self.luck_state
                    .value
                    .map_or("—".to_owned(), |v| v.to_string())
            ),
        );
        push_line(&mut out, format!("MOV: {}", derived.mov));
        push_line(&mut out, format!("Dodge: {}%", derived.dodge));
        push_line(&mut out, format!("DB: {}", derived.db));
        push_line(
            &mut out,
            format!(
                "Build: {}",
                if derived.build >= 0 {
                    format!("+{}", derived.build)
                } else {
                    derived.build.to_string()
                }
            ),
        );

        let credit = math.credit_rating;
        push_blank_line(&mut out);
        push_line(&mut out, "Credit Rating");
        push_line(
            &mut out,
            format!("{}% ({})", credit, get_credit_tier(credit)),
        );

        push_blank_line(&mut out);
        push_line(&mut out, "Skills");
        for row in skill_rows
            .iter()
            .filter(|row| row.total > row.base || SUMMARY_ALWAYS_SHOW.contains(&row.id.name()))
        {
            push_line(
                &mut out,
                format!(
                    "{}: {}% ({}/{})",
                    row.name,
                    row.total,
                    floor_half(row.total),
                    floor_fifth(row.total)
                ),
            );
        }

        if self
            .backstory
            .values()
            .any(|value| !value.trim().is_empty())
        {
            push_blank_line(&mut out);
            push_line(&mut out, "Backstory");
            for category in BACKSTORY_CATEGORIES {
                if let Some(value) = self
                    .backstory
                    .get(*category)
                    .filter(|value| !value.trim().is_empty())
                {
                    push_line(&mut out, format!("{}: {}", category, value.trim()));
                }
            }
        }

        out
    }
}
