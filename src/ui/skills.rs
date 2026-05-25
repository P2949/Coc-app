use super::super::data::*;
use super::super::models::*;
use super::skill_accepts_personal_points;
use eframe::egui;
use egui::RichText;

impl CoC7eApp {
    pub(crate) fn render_skills(&mut self, ui: &mut egui::Ui) {
        self.prune_personal_allocations();

        heading(
            ui,
            "IV. Allocate Skills",
            "Occupation points can only go into resolved occupation skills plus Credit Rating. Personal interest points can go into non-Mythos, non-Credit-Rating skills. Skill totals are capped at 99 for creation.",
        );

        let mut math = self.sheet_math();
        let initial_no_occupation = math.selected_occupation.is_none();
        let initial_unresolved = math.unresolved_choices;
        let initial_shortfall = math.occupation_shortfall;

        card(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let quick_response = ui.add_enabled(
                    !initial_no_occupation && initial_unresolved == 0 && initial_shortfall == 0,
                    egui::Button::new("Replace with starter skill spread"),
                );
                if quick_response.clicked() {
                    self.apply_quick_skill_package();
                    math = self.sheet_math();
                }
                quick_response.on_hover_text("Replaces current occupation allocations. Sets Credit Rating to the occupation minimum (skipped if the minimum is 0), applies a starter target spread to eligible skills sorted by current base value, then fills remaining occupation points while skill caps allow.");
                if ui.button("Clear occupation points").clicked() {
                    self.clear_occupation_allocations();
                    math = self.sheet_math();
                }
                if ui.button("Clear personal points").clicked() {
                    self.clear_personal_allocations();
                    math = self.sheet_math();
                }
            });

            ui.add_space(8.0);

            let occ_budget = math.occupation_budget;
            let personal_budget = math.personal_budget;
            let used_occ = CoC7eApp::used_occupation_points_from(&math.skill_rows);
            let used_personal = CoC7eApp::used_personal_points_from(&math.skill_rows);
            let credit = math.credit_rating;
            let (credit_min, credit_max) = math.credit_range;
            let no_occupation = math.selected_occupation.is_none();
            let unresolved = math.unresolved_choices;
            let shortfall = math.occupation_shortfall;
            let skill_rows = &math.skill_rows;
            let skills_over_99 = skill_rows.iter().any(|row| row.total > MAX_CREATION_VALUE);

            ui.horizontal_wrapped(|ui| {
                pill(
                    ui,
                    format!("Occupation {used_occ} / {occ_budget}"),
                    if used_occ > occ_budget {
                        RED
                    } else if used_occ < occ_budget {
                        AMBER
                    } else {
                        GREEN
                    },
                );
                pill(
                    ui,
                    format!("Personal {used_personal} / {personal_budget}"),
                    if used_personal > personal_budget {
                        RED
                    } else if used_personal < personal_budget {
                        AMBER
                    } else {
                        GREEN
                    },
                );
                pill(
                    ui,
                    format!(
                        "Credit Rating {credit}%{}",
                        if !no_occupation {
                            format!(" (needs {credit_min}–{credit_max})")
                        } else {
                            String::new()
                        }
                    ),
                    if !no_occupation && (credit < credit_min || credit > credit_max) {
                        RED
                    } else {
                        AMBER
                    },
                );
                if skills_over_99 {
                    pill(ui, "A skill exceeds 99", RED);
                }
                if no_occupation {
                    pill(ui, "Choose an occupation first", RED);
                }
                if unresolved > 0 {
                    pill(ui, "Resolve occupation choices", RED);
                }
                if shortfall > 0 {
                    pill(
                        ui,
                        format!("Choose {shortfall} more occupation skill(s)"),
                        RED,
                    );
                }
            });
        });

        let no_occupation = math.selected_occupation.is_none();
        let allowed_occ = &math.occupation_skill_set;
        let skill_rows = &math.skill_rows;

        card(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("skills_grid")
                        .num_columns(5)
                        .spacing([12.0, 6.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Skill").small().color(MUTED).monospace());
                            ui.label(RichText::new("Base").small().color(MUTED).monospace());
                            ui.label(RichText::new("Occupation").small().color(MUTED).monospace());
                            ui.label(RichText::new("Personal").small().color(MUTED).monospace());
                            ui.label(RichText::new("Total").small().color(MUTED).monospace());
                            ui.end_row();

                            for row in skill_rows {
                                let can_occ = allowed_occ.contains(&row.name);
                                let can_personal = skill_accepts_personal_points(&row.name);
                                let occ_max =
                                    (MAX_CREATION_VALUE - row.base - row.personal_add).max(0);
                                let personal_max =
                                    (MAX_CREATION_VALUE - row.base - row.occ_add).max(0);
                                let note = if row.name == "Credit Rating" {
                                    "occupation-only at creation"
                                } else if row.name == "Cthulhu Mythos" {
                                    "locked at creation"
                                } else if can_occ {
                                    "occupation eligible"
                                } else {
                                    "personal only"
                                };

                                ui.vertical(|ui| {
                                    ui.label(RichText::new(&row.name).color(if can_occ {
                                        TEXT
                                    } else {
                                        MUTED
                                    }));
                                    ui.label(RichText::new(note).small().color(FAINT));
                                });
                                ui.label(
                                    RichText::new(format!("{}%", row.base))
                                        .monospace()
                                        .color(MUTED),
                                );

                                let mut occ_value = if can_occ { row.occ_add } else { 0 };
                                if ui
                                    .add_enabled(
                                        can_occ && !no_occupation,
                                        egui::DragValue::new(&mut occ_value)
                                            .range(0..=occ_max)
                                            .speed(1.0),
                                    )
                                    .changed()
                                {
                                    self.set_occupation_allocation(&row.name, occ_value);
                                }

                                let mut personal_value =
                                    if can_personal { row.personal_add } else { 0 };
                                if ui
                                    .add_enabled(
                                        can_personal,
                                        egui::DragValue::new(&mut personal_value)
                                            .range(0..=personal_max)
                                            .speed(1.0),
                                    )
                                    .changed()
                                {
                                    self.set_personal_allocation(&row.name, personal_value);
                                }

                                ui.label(
                                    RichText::new(format!("{}%", row.total))
                                        .monospace()
                                        .strong()
                                        .color(if row.total > MAX_CREATION_VALUE {
                                            RED
                                        } else if row.total >= 70 {
                                            ACCENT
                                        } else if row.total >= 50 {
                                            GREEN
                                        } else {
                                            MUTED
                                        }),
                                );
                                ui.end_row();
                            }
                        });
                });
        });

        self.navigation(ui);
    }
}
