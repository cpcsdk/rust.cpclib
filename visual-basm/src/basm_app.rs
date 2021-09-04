
use eframe::{egui::{self, ScrollArea}, epi::{self, App}};
use cpclib_asm::{basm_utils::*, error::AssemblerError};

enum AssembleState {
	Ok,
	Error,
	Waiting
}
pub struct BasmApp {
	fname: String,
	error: Option<String>,
	warnings: Vec<String>,
	lst_file: temp_file::TempFile,
	lst_content: String,
	case_insensitive: bool,
	assemble_state: AssembleState,
	generate_sna: bool
}

impl Default for BasmApp {
	fn default() -> Self {
		Self {
			fname: String::default(),
			error: None,
			warnings: Vec::new(),
			lst_file: temp_file::empty(),
			lst_content: String::new(),
    		case_insensitive: false,
			assemble_state: AssembleState::Waiting,
			generate_sna: false
		}
	}
}


impl BasmApp {
	fn build_command_line(&self) -> Vec<String> {
		let mut command = Vec::new();
		let fname = std::env::current_dir().unwrap().join(&self.fname).display().to_string();

		command.push("basm".to_owned());

		command.push("-I".to_owned()) ; command.push(std::env::current_dir().unwrap().display().to_string());

		command.push("--lst".to_owned()) ; command.push(self.lst_file.path().display().to_string());

		if self.case_insensitive {
			command.push("-i".to_owned());
		}

		command.push("-o".to_owned());
		if self.generate_sna {
			command.push(fname.clone() + ".sna");
		} else {
			command.push(fname.clone() + "o");
		}

		command.push(fname);

		command
	}
	fn assemble(&mut self) {
		let cmd = self.build_command_line();
		eprintln!("Assemble with: {:?}", cmd);
		let matches = build_args_parser()
					.get_matches_from(cmd);

		match process(&matches) {
			Ok((env, warnings)) => {
				self.error = None;
				self.warnings = warnings.iter()
					.map(|w| {
						String::from_utf8_lossy(&strip_ansi_escapes::strip(format!("{}", w)).unwrap()).to_string()
					}).collect::<Vec<_>>();
				self.lst_content.clear();
				let mut lst = std::fs::read_to_string(self.lst_file.path()).unwrap_or_default();
				std::mem::swap(&mut lst, &mut self.lst_content);
				self.assemble_state = AssembleState::Ok;
				
			}
			Err(e) => {
				self.error = Some(String::from_utf8_lossy(&strip_ansi_escapes::strip(format!("{}", e)).unwrap()).to_string());
				self.warnings.clear();
				self.lst_content.clear();
				self.assemble_state = AssembleState::Error;
			}
		}
	}
}

impl epi::App for BasmApp {
    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
		let Self{fname, error, warnings, lst_content, 
			case_insensitive,assemble_state,
			generate_sna,
			..} = self;
		let mut assemble = false;


		if let Some(fname2) = ctx.input().raw.dropped_files.first() {
			match &fname2.path {
				Some(path) => {
					if path.is_file() {
						let fname2 = path.file_name().unwrap();
						let dir = path.parent().unwrap();

						*fname = fname2.to_str().unwrap().to_owned();
						std::env::set_current_dir(dir);

					}
				},
				None => {},
			}
		}
		

		egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                egui::menu::menu(ui, "File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }

					if !fname.is_empty(){
						if ui.button("Assemble").clicked() {
							assemble = true;
						}
					}
                });
            });
        });

		egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
			ui.horizontal(|ui| {
				ui.label(format!("Working directory: {}", std::env::current_dir().unwrap().display()));
			})
		});


		egui::SidePanel::left("left_panel").show(ctx, |ui| {
            ui.heading("Basm");

            ui.horizontal(|ui| {
                ui.label("Source: ");
                let text = ui.text_edit_singleline(fname).on_hover_text("File to assemble");
				if ui.memory().is_anything_being_dragged() {
					text.on_hover_cursor(egui::CursorIcon::Move);
				}
            });

			ui.horizontal(|ui| {
                ui.checkbox(case_insensitive, "Case insensitive");
            });

			ui.horizontal(|ui| {
                ui.checkbox(generate_sna, "Build sna");
            });

			if ui.button("Assemble").clicked() {
				assemble = true;
			}

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add(
                    egui::Hyperlink::new("https://github.com/cpcsdk/rust.cpclib").text("powered by basm"),
                );
            });
        });


		egui::SidePanel::right("right_panel").show(ctx, |ui| {
			ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
				ui.set_enabled(false);

				match assemble_state {
					AssembleState::Ok => {ui.label("Assembling successfull");},
					AssembleState::Error => {ui.label("Assembling error");},
					AssembleState::Waiting => {},
				}

				if let Some(error) = error {
					 ui.text_edit_multiline(error); // todo set read only
				}
				for warning in warnings.iter_mut() {
					ui.text_edit_multiline(warning); // todo set read only
				}


			})
		});


		egui::CentralPanel::default()
		.show(ctx, |ui| {
			
		//	ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
				ScrollArea::auto_sized()
					.always_show_scroll(true)
					.show(ui, |ui| {
						ui.vertical(|ui| {
							ui.set_enabled(false);
							ui.max_rect();
							ui.text_edit_multiline(lst_content);
						});
				});
				
			});
	//	});


		if assemble {
			self.assemble();
		}

    }

    fn name(&self) -> &str {
        "Visual BASM"
    }
}