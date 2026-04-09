mod model;
mod settings;
mod writer;

use chrono::Local;
use eframe::egui::{self, Key, RichText};
use model::NoteSection;
use settings::{AppSettings, NoteWriteMode};

fn main() -> anyhow::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([760.0, 620.0])
            .with_min_inner_size([560.0, 420.0])
            .with_title("Trace for Windows"),
        ..Default::default()
    };

    let settings = AppSettings::load().unwrap_or_default();
    let app = TraceWinApp::new(settings);

    eframe::run_native(
        "Trace for Windows",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .map_err(|err| anyhow::anyhow!(err.to_string()))
}

struct TraceWinApp {
    settings: AppSettings,
    selected: NoteSection,
    input: String,
    status: String,
    settings_open: bool,
}

impl TraceWinApp {
    fn new(settings: AppSettings) -> Self {
        Self {
            settings,
            selected: NoteSection::Note,
            input: String::new(),
            status: "Ready".to_string(),
            settings_open: true,
        }
    }

    fn save_current_note(&mut self) {
        match writer::save_note(Local::now(), &self.input, self.selected, &self.settings) {
            Ok(path) => {
                self.input.clear();
                self.status = format!("Saved to {}", path.display());
            }
            Err(err) => {
                self.status = format!("Save failed: {err}");
            }
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|input| {
            if input.modifiers.ctrl && input.key_pressed(Key::Enter) {
                self.save_current_note();
            }

            if input.modifiers.ctrl && input.key_pressed(Key::Num1) {
                self.selected = NoteSection::Note;
            } else if input.modifiers.ctrl && input.key_pressed(Key::Num2) {
                self.selected = NoteSection::Clip;
            } else if input.modifiers.ctrl && input.key_pressed(Key::Num3) {
                self.selected = NoteSection::Link;
            } else if input.modifiers.ctrl && input.key_pressed(Key::Num4) {
                self.selected = NoteSection::Task;
            } else if input.modifiers.ctrl && input.key_pressed(Key::Num5) {
                self.selected = NoteSection::Project;
            }
        });
    }

    fn save_settings(&mut self) {
        self.settings.normalize();
        match self.settings.save() {
            Ok(_) => self.status = "Settings saved".to_string(),
            Err(err) => self.status = format!("Settings save failed: {err}"),
        }
    }
}

impl eframe::App for TraceWinApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Trace for Windows").strong().size(22.0));
                ui.separator();
                ui.label("Thought is leverage, Leave a trace.");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(if self.settings_open {
                            "Hide Settings"
                        } else {
                            "Show Settings"
                        })
                        .clicked()
                    {
                        self.settings_open = !self.settings_open;
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(6.0);

            ui.horizontal_wrapped(|ui| {
                for section in NoteSection::ALL {
                    let is_selected = self.selected == section;
                    let title = self.settings.title_for(section).to_string();
                    let button = egui::SelectableLabel::new(
                        is_selected,
                        format!("{} ({})", title, section.shortcut_label()),
                    );
                    if ui.add(button).clicked() {
                        self.selected = section;
                    }
                }
            });

            ui.add_space(10.0);
            ui.label(RichText::new("Capture").strong());
            ui.label(format!(
                "Current write mode: {}",
                self.settings.note_write_mode.title()
            ));
            ui.label("Ctrl+Enter 保存；Ctrl+1~5 切换模块");
            ui.add(
                egui::TextEdit::multiline(&mut self.input)
                    .hint_text("Write your thought here...")
                    .desired_rows(14)
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Save Note (Ctrl+Enter)").clicked() {
                    self.save_current_note();
                }
                if ui.button("Clear").clicked() {
                    self.input.clear();
                }
            });

            ui.add_space(6.0);
            ui.label(RichText::new(&self.status).italics());

            if self.settings_open {
                ui.separator();
                ui.label(RichText::new("Settings").strong());

                ui.horizontal(|ui| {
                    ui.label("Vault Path");
                    ui.text_edit_singleline(&mut self.settings.vault_path);
                });
                ui.horizontal(|ui| {
                    ui.label("Write Mode");
                    for mode in [NoteWriteMode::Dimension, NoteWriteMode::File] {
                        let selected = self.settings.note_write_mode == mode;
                        let button = egui::Button::new(mode.title())
                            .min_size(egui::vec2(130.0, 26.0))
                            .fill(if selected {
                                egui::Color32::from_rgb(108, 49, 227)
                            } else {
                                ui.visuals().widgets.inactive.bg_fill
                            })
                            .stroke(egui::Stroke::new(
                                1.0,
                                if selected {
                                    egui::Color32::from_rgb(160, 121, 255)
                                } else {
                                    ui.visuals().widgets.inactive.bg_stroke.color
                                },
                            ));
                        if ui.add(button).clicked() {
                            self.settings.note_write_mode = mode;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Daily Folder");
                    ui.text_edit_singleline(&mut self.settings.daily_folder_name);
                });
                ui.horizontal(|ui| {
                    ui.label("Inbox Folder");
                    ui.text_edit_singleline(&mut self.settings.inbox_folder_name);
                });
                ui.horizontal(|ui| {
                    ui.label("File Date Format");
                    ui.text_edit_singleline(&mut self.settings.daily_file_date_format);
                });

                ui.add_space(8.0);
                ui.label("Section Titles");
                for section in NoteSection::ALL {
                    let mut current = self.settings.title_for(section).to_string();
                    ui.horizontal(|ui| {
                        ui.label(format!("{}", section.shortcut_label()));
                        if ui.text_edit_singleline(&mut current).changed() {
                            self.settings.set_title_for(section, current.clone());
                        }
                    });
                }

                ui.horizontal(|ui| {
                    if ui.button("Save Settings").clicked() {
                        self.save_settings();
                    }
                    if ui.button("Reset Default Titles").clicked() {
                        self.settings = AppSettings {
                            section_titles: NoteSection::ALL
                                .iter()
                                .map(|it| it.default_title().to_string())
                                .collect(),
                            ..self.settings.clone()
                        };
                        self.status = "Section titles reset".to_string();
                    }
                });
            }
        });
    }
}
