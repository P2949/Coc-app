use super::super::data::*;
use super::super::models::*;
use eframe::egui;
use egui::RichText;

impl CoC7eApp {
    pub(crate) fn render_concept(&mut self, ui: &mut egui::Ui) {
        heading(
            ui,
            "I. Investigator Concept",
            "Start with the investigator’s identity. Mechanical age effects are applied on the next step.",
        );

        card(ui, |ui| {
            egui::Grid::new("concept_grid")
                .num_columns(2)
                .spacing([16.0, 10.0])
                .show(ui, |ui| {
                    labeled_text(
                        ui,
                        "Investigator name",
                        &mut self.concept.name,
                        "Harvey Walters",
                    );
                    let mut age = self.concept.age;
                    if labeled_i32(ui, "Age 15–89", &mut age, 15, 89, 1.0).changed() {
                        self.set_age(age);
                    }
                    ui.end_row();
                    labeled_text(
                        ui,
                        "Pronouns / Gender",
                        &mut self.concept.pronouns,
                        "Optional",
                    );
                    labeled_text(ui, "Residence", &mut self.concept.residence, "Arkham, MA");
                    ui.end_row();
                    labeled_text(
                        ui,
                        "Birthplace",
                        &mut self.concept.birthplace,
                        "Boston, Massachusetts",
                    );
                    ui.end_row();
                });
        });

        let bracket = self.age_bracket();
        card(ui, |ui| {
            ui.label(
                RichText::new(format!("Age bracket: {}", bracket.label))
                    .size(16.0)
                    .color(ACCENT)
                    .strong(),
            );
            ui.label(RichText::new(bracket.note).color(MUTED));
        });

        self.navigation(ui);
    }
}
