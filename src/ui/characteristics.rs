use super::super::data::*;
use super::super::models::*;
use super::super::ruleset::*;
use eframe::egui;
use egui::{RichText, Stroke};

impl CoC7eApp {
    pub(crate) fn render_characteristics(&mut self, ui: &mut egui::Ui) {
        heading(
            ui,
            "II. Characteristics, Luck, and Age Effects",
            "Roll characteristics, use the optional 460-budget helper preset, or apply the quick fixed array. Final values include age modifiers.",
        );

        card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Roll all").clicked() {
                    self.roll_all_characteristics();
                }
                if ui.button("Optional 460-budget preset").clicked() {
                    self.apply_characteristic_preset(
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
                }
                if ui.button("Quick array preset").clicked() {
                    self.apply_characteristic_preset(
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
                }
            });

            match self.char_method {
                CharMethod::PointBuy => {
                    ui.add_space(8.0);
                    let spent = self.point_buy_spent();
                    let remaining = POINT_BUY_BUDGET - spent;
                    pill(
                        ui,
                        format!(
                            "Optional budget: {spent} / {POINT_BUY_BUDGET} ({})",
                            if remaining >= 0 {
                                format!("{remaining} left")
                            } else {
                                format!("{} over", -remaining)
                            }
                        ),
                        if remaining == 0 {
                            GREEN
                        } else if remaining < 0 {
                            RED
                        } else {
                            AMBER
                        },
                    );
                    ui.label(RichText::new("Manual inputs remain clamped to normal human roll ranges and step by 5.").small().color(FAINT));
                }
                CharMethod::QuickArray => {
                    ui.add_space(8.0);
                    pill(ui, "Quick values: 40, 50, 50, 50, 60, 60, 70, 80", BLUE);
                }
                CharMethod::Mixed => {
                    ui.add_space(8.0);
                    pill(ui, "Mixed manual / preset / rolled values", AMBER);
                }
                CharMethod::Roll => {}
            }
        });

        let final_chars = self.final_chars();
        card(ui, |ui| {
            for def in CHARACTERISTICS {
                let raw = self.char_value(def.key.key());
                let final_value = final_chars.get_char(def.key);
                let changed_by_age = raw > 0 && raw != final_value;

                egui::Frame::new()
                    .fill(PANEL_2)
                    .stroke(Stroke::new(1.0, LINE))
                    .corner_radius(egui::CornerRadius::same(10))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(def.key.key())
                                        .color(ACCENT)
                                        .monospace()
                                        .strong(),
                                );
                                ui.label(RichText::new(def.name).color(TEXT));
                                ui.label(
                                    RichText::new(format!(
                                        "{} × 5 · {}",
                                        def.dice.label(),
                                        def.desc
                                    ))
                                    .small()
                                    .color(FAINT)
                                    .monospace(),
                                );
                            });

                            ui.separator();

                            if let Some(result) = self.char_rolls.get(def.key.key()) {
                                dice_display(ui, result, None);
                            } else {
                                ui.label(
                                    RichText::new("Manual / preset value").small().color(FAINT),
                                );
                            }

                            ui.separator();

                            let mut value = raw;
                            let response = ui.add(
                                egui::DragValue::new(&mut value)
                                    .range(def.min..=def.max)
                                    .speed(5.0)
                                    .prefix("Value "),
                            );
                            if response.changed() {
                                self.set_char_value(def.key.key(), value);
                            }

                            if ui.button("Roll").clicked() {
                                self.roll_single_characteristic(def.key.key());
                            }
                        });

                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            pill(
                                ui,
                                format!(
                                    "Raw {}",
                                    if raw > 0 {
                                        raw.to_string()
                                    } else {
                                        "—".to_owned()
                                    }
                                ),
                                if changed_by_age { AMBER } else { MUTED },
                            );
                            pill(
                                ui,
                                format!(
                                    "Final {}",
                                    if final_value > 0 {
                                        final_value.to_string()
                                    } else {
                                        "—".to_owned()
                                    }
                                ),
                                ACCENT,
                            );
                            if final_value > 0 {
                                pill(
                                    ui,
                                    format!(
                                        "½ {} · ⅕ {}",
                                        floor_half(final_value),
                                        floor_fifth(final_value)
                                    ),
                                    MUTED,
                                );
                            }
                        });
                    });
                ui.add_space(8.0);
            }
        });

        self.render_age_controls(ui);
        self.render_luck(ui);

        let final_chars = self.final_chars();
        self.render_derived_preview(ui, final_chars);
        self.navigation(ui);
    }

    pub(crate) fn render_age_controls(&mut self, ui: &mut egui::Ui) {
        let bracket = self.age_bracket();
        card(ui, |ui| {
            ui.label(RichText::new("Age modifier controls").size(16.0).strong());
            ui.label(RichText::new(bracket.note).color(MUTED));

            if bracket.physical_deduct > 0 {
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    let total_before = self.physical_deduction_total();
                    for key in bracket.physical_from {
                        let current = self.age_deductions.get_char(*key);
                        let current_effective = self.effective_physical_deduction_for(*key);
                        let other_effective_total = total_before - current_effective;
                        let remaining_effective =
                            (bracket.physical_deduct - other_effective_total).max(0);
                        let max_effective_for_key =
                            max_physical_deduction_for_raw(self.chars.get_char(*key));
                        let max_for_key = remaining_effective.min(max_effective_for_key).max(0);
                        let mut value = clamp_step_5(current, 0, max_for_key);

                        if value != current {
                            self.age_deductions.set_char(*key, value);
                        }

                        ui.label(key.key());
                        if ui
                            .add(
                                egui::DragValue::new(&mut value)
                                    .range(0..=max_for_key)
                                    .speed(5.0),
                            )
                            .changed()
                        {
                            let snapped = clamp_step_5(value, 0, max_for_key);
                            self.age_deductions.set_char(*key, snapped);
                        }
                    }
                });
                let total = self.physical_deduction_total();
                let assigned_total = self.assigned_physical_deduction_total();
                ui.label(
                    RichText::new(format!(
                        "Effective physical deduction: {total} / {}",
                        bracket.physical_deduct
                    ))
                    .small()
                    .color(MUTED)
                    .strong(),
                );
                if assigned_total != total {
                    ui.label(RichText::new(format!("Assigned {assigned_total}, but only {total} is effective because age deductions are capped to the nearest valid 5-point floor.")).small().color(AMBER));
                }
                let ok = total == bracket.physical_deduct;
                ui.label(
                    RichText::new(if ok {
                        "Physical age deductions are fully assigned."
                    } else {
                        "Assign the exact total deduction before finalizing the sheet."
                    })
                    .small()
                    .color(if ok { GREEN } else { AMBER }),
                );
            }

            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if bracket.edu_checks > 0
                    && ui
                        .add_enabled(
                            self.char_value("EDU") > 0,
                            egui::Button::new(format!(
                                "Roll {} EDU check{}",
                                bracket.edu_checks,
                                if bracket.edu_checks > 1 { "s" } else { "" }
                            )),
                        )
                        .clicked()
                {
                    self.roll_edu_age_checks();
                }
                pill(ui, format!("APP penalty {}", bracket.app_penalty), MUTED);
                pill(ui, format!("EDU penalty {}", bracket.edu_penalty), MUTED);
                pill(ui, format!("EDU bonus +{}", self.edu_bonus), MUTED);
            });

            if bracket.edu_checks > 0 && !self.edu_age_checks_complete() {
                ui.label(
                    RichText::new(format!(
                        "Summary unlocks after rolling the required EDU age check{} ({}/{} complete).",
                        if bracket.edu_checks > 1 { "s" } else { "" },
                        self.edu_check_rolls.len(),
                        bracket.edu_checks
                    ))
                    .small()
                    .color(AMBER),
                );
            }

            for (index, roll) in self.edu_check_rolls.iter().enumerate() {
                let text = if roll.improved {
                    format!(
                        "Check {}: d100 {} → +{} EDU (now {})",
                        index + 1,
                        roll.d100,
                        roll.gain,
                        roll.resulting_edu
                    )
                } else {
                    format!("Check {}: d100 {} → no improvement", index + 1, roll.d100)
                };
                ui.label(
                    RichText::new(text)
                        .small()
                        .monospace()
                        .color(if roll.improved { GREEN } else { MUTED }),
                );
            }
        });
    }

    pub(crate) fn render_luck(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            ui.label(RichText::new("Luck").size(16.0).strong());
            ui.label(
                RichText::new("Luck is 3D6×5. Ages 15–19 roll twice and keep the higher value.")
                    .color(MUTED),
            );
            ui.horizontal(|ui| {
                if ui.button("Roll Luck").clicked() {
                    self.roll_luck();
                }
                ui.label(
                    RichText::new(
                        self.luck_state
                            .value
                            .map_or("—".to_owned(), |v| v.to_string()),
                    )
                    .size(28.0)
                    .monospace()
                    .color(AMBER)
                    .strong(),
                );
            });
            let show_luck_labels = self.luck_state.rolls.len() > 1;
            for attempt in &self.luck_state.rolls {
                let label = if show_luck_labels {
                    Some(if attempt.kept.unwrap_or(false) {
                        "kept"
                    } else {
                        "discarded"
                    })
                } else {
                    None
                };
                dice_display(ui, attempt, label);
            }
        });
    }

    pub(crate) fn render_derived_preview(
        &mut self,
        ui: &mut egui::Ui,
        final_chars: CharacteristicValues,
    ) {
        card(ui, |ui| {
            ui.label(RichText::new("Derived preview").size(16.0).strong());
            if !self.has_all_chars() {
                ui.label(
                    RichText::new(
                        "Enter or roll all eight characteristics to calculate derived attributes.",
                    )
                    .color(MUTED),
                );
                return;
            }

            let math = self.sheet_math_from(final_chars);
            let d = math.derived;
            egui::Grid::new("derived_preview")
                .num_columns(4)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    stat_box(ui, "HP", d.hp, RED);
                    stat_box(ui, "Major Wound", d.major_wound, RED);
                    stat_box(ui, "SAN", d.san, ACCENT);
                    stat_box(ui, "MP", d.mp, BLUE);
                    ui.end_row();
                    stat_box(ui, "MOV", d.mov, GREEN);
                    stat_box(ui, "Dodge", format!("{}%", d.dodge), AMBER);
                    stat_box(ui, "DB", d.db, AMBER);
                    stat_box(
                        ui,
                        "Build",
                        if d.build >= 0 {
                            format!("+{}", d.build)
                        } else {
                            d.build.to_string()
                        },
                        AMBER,
                    );
                    ui.end_row();
                });
        });
    }
}
