#![feature(let_chains)]

use std::io::Read;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use cpclib_bndbuild::deps::Rule;
use cpclib_bndbuild::BndBuilder;
use eframe::egui::{self, RichText};
use eframe::epaint::ahash::HashMap;
use eframe::epaint::Color32;
use egui_file::{self, FileDialog};
use itertools::Itertools;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct BndBuildApp {
    /// The provided filename by the user
    filename: Option<std::path::PathBuf>,
    #[serde(skip)] // need to be rebuild at loading
    /// The corresponding builder
    builder_and_layers: Option<BuilderAndCache>,

    /// The content of stdout
    #[serde(skip)]
    logs: String,

    /// Message error related to file
    #[serde(skip)]
    file_error: Option<String>,

    /// Message error related to build
    #[serde(skip)]
    build_error: Option<String>,

    /// Open file window
    #[serde(skip)]
    open_file_dialog: Option<FileDialog>,

    /// Target to build requested by button
    #[serde(skip)]
    requested_target: Option<PathBuf>,

    /// stdout redirection
    #[serde(skip)]
    gag: gag::BufferRedirect,

    #[serde(skip)]
    request_reload: bool,

    #[serde(skip)]
    job: Option<std::thread::JoinHandle<Result<(), cpclib_bndbuild::BndBuilderError>>>
}

impl Default for BndBuildApp {
    fn default() -> Self {
        BndBuildApp {
            filename: None,
            builder_and_layers: None,
            file_error: None,
            build_error: None,
            open_file_dialog: None,
            requested_target: None,
            logs: String::default(),
            request_reload: false,
            job: None,
            gag: gag::BufferRedirect::stdout().unwrap()
        }
    }
}

/// Store the list of targets level per level
struct Layers<'builder>(Vec<Vec<&'builder Path>>);
/// Cache up to date information to not recompute it 60 times per seconds
struct UpToDate<'builder>(HashMap<&'builder Rule, bool>);

impl<'builder> Deref for Layers<'builder> {
    type Target = Vec<Vec<&'builder Path>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'builder> Deref for UpToDate<'builder> {
    type Target = HashMap<&'builder Rule, bool>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

struct BuilderCache<'builder> {
    layers: Layers<'builder>,
    up_to_date: UpToDate<'builder>
}

impl<'builder> From<&'builder BndBuilder> for UpToDate<'builder> {
    fn from(builder: &'builder BndBuilder) -> Self {
        let mut map = HashMap::default();
        for rule in builder.rules().iter() {
            map.insert(rule, rule.is_up_to_date());
        }
        UpToDate(map)
    }
}

impl<'builder> From<&'builder BndBuilder> for Layers<'builder> {
    fn from(builder: &'builder BndBuilder) -> Self {
        Layers(
            builder
                .get_layered_dependencies()
                .into_iter()
                .map(|set| {
                    let mut vec = set.into_iter().collect_vec();
                    vec.sort();
                    vec
                })
                .collect_vec()
        )
    }
}

impl<'builder> From<&'builder BndBuilder> for BuilderCache<'builder> {
    fn from(builder: &'builder BndBuilder) -> Self {
        BuilderCache {
            layers: builder.into(),
            up_to_date: builder.into()
        }
    }
}

self_cell::self_cell! {
    /// WARNING the BndBuilder changes the current working directory.
    /// This is probably a problematic behavior. Need to think about it later
    struct BuilderAndCache {
        owner: BndBuilder,
        #[covariant]
        dependent: BuilderCache,
    }
}

impl From<BndBuilder> for BuilderAndCache {
    fn from(builder: BndBuilder) -> Self {
        BuilderAndCache::new(builder, |builder| builder.into())
    }
}

impl BndBuildApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            let mut app: BndBuildApp =
                eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
            app.build_error = None;
            app.file_error = None;
            app.builder_and_layers = None;
            app.open_file_dialog = None;
            app.requested_target = None;
            if let Some(fname) = &app.filename {
                app.load(fname.clone());
            }
            app
        }
        else {
            Default::default()
        }
    }

    pub fn load(&mut self, path: PathBuf) {
        match cpclib_bndbuild::BndBuilder::from_fname(path.clone()) {
            Ok(builder) => {
                self.filename = path.into();
                self.builder_and_layers = BuilderAndCache::from(builder).into()
            }
            Err(err) => {
                self.file_error = Some(err.to_string());
            }
        }
    }

    pub fn update_cache(&mut self) {
        // TODO reimplement it in a way that does not necessitate to reload the file
        let path = self.filename.as_mut().unwrap().clone();
        self.load(path);
    }
}

impl BndBuildApp {
    fn update_menu(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            if self.job.is_some() {
                ui.set_enabled(false);
            }

            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        let mut dialog = egui_file::FileDialog::open_file(self.filename.clone());
                        dialog.open();
                        self.open_file_dialog = dialog.into();
                        self.file_error = None;
                    };

                    if self.filename.is_some() {
                        if ui.button("Reload").clicked() {
                            self.request_reload = true;
                        }
                    }

                    if ui.button("Quit").clicked() {
                        frame.close();
                    }
                });
            })
        });
    }

    fn update_status(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            match &self.filename {
                Some(fname) => ui.label(fname.display().to_string()),
                None => ui.label("No file loaded")
            };

            ui.separator();
        });
    }

    fn update_log(&mut self, _ctx: &egui::Context, ui: &mut eframe::egui::Ui) {
        if let Some(error) = self.build_error.as_ref() {
            let txt = RichText::new(error).color(Color32::RED).strong();
            ui.label(txt);
        }

        ui.heading("Output");
        egui::ScrollArea::new([true, true])
            .max_height(f32::INFINITY)
            .max_width(f32::INFINITY)
            //  .min_height(f32::INFINITY)
            //   .min_width(f32::INFINITY)
            .stick_to_right(true)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.code(&self.logs);
            });
    }

    fn update_targets(&mut self, _ctx: &egui::Context, ui: &mut eframe::egui::Ui) {
        if let Some(bnl) = &self.builder_and_layers {
            let default = bnl.borrow_owner().default_target();

            ui.vertical_centered(|ui| {
                ui.heading("Tasks");
                let cache = bnl.borrow_dependent();
                for layer in cache.layers.iter() {
                    ui.horizontal(|ui| {
                        for tgt in layer.iter() {
                            let rule = bnl.borrow_owner().get_rule(tgt);

                            let txt = RichText::new(tgt.display().to_string());
                            let txt = if let Some(default) = &default && default == tgt {
                                txt.strong()
                            } else {
                                txt
                            };
                            let txt = if let Some(rule) = &rule {
                                if *cache.up_to_date.get(rule).unwrap() {
                                    txt.color(Color32::LIGHT_BLUE)
                                }
                                else {
                                    txt.color(Color32::LIGHT_RED)
                                }
                            }
                            else {
                                txt.color(Color32::LIGHT_GREEN)
                            };

                            let mut button = ui.button(txt);
                            button = if let Some(rule) = rule {
                                if let Some(help) = rule.help() {
                                    button.on_hover_text(help)
                                }
                                else {
                                    button
                                }
                            }
                            else {
                                button.on_hover_text("Probably a leaf file")
                            };
                            if button.clicked() {
                                self.requested_target = Some(tgt.into());
                                self.logs.clear();
                            }
                        }
                    });
                }
            });
        }
    }

    fn update_inner(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.job.is_some() {
                ui.set_enabled(false);
            }

            if let Some(error) = &self.file_error {
                ui.colored_label(Color32::RED, error);
                return;
            };

            ui.columns(2, |columns| {
                self.update_targets(ctx, &mut columns[0]);
                self.update_log(ctx, &mut columns[1]);
            });
        });
    }
}

impl eframe::App for BndBuildApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.job.is_some() {
            ctx.set_cursor_icon(egui::CursorIcon::Progress);
        }

        self.update_menu(ctx, frame);
        self.update_inner(ctx, frame);
        self.update_status(ctx, frame);

        // Handle file opening
        if let Some(dialog) = &mut self.open_file_dialog {
            if dialog.show(ctx).selected() {
                if let Some(path) = dialog.path() {
                    if path.exists() {
                        self.load(path);
                    }
                    else {
                        self.file_error =
                            format!("{} does not exists.", path.display().to_string()).into();
                    }
                }
            }
        }

        // Handle reload
        if self.request_reload {
            self.request_reload = false;
            self.load(self.filename.clone().unwrap());
        }

        // Handle target
        if let Some(tgt) = self.requested_target.take() {
            if let Some(builder) = &self.builder_and_layers {
                let builder: &'static BuilderAndCache = unsafe { std::mem::transmute(builder) }; // cheat on lifetime as we know if will live all the time
                self.job = std::thread::spawn(|| builder.borrow_owner().execute(tgt)).into();
            }
        }

        // Handle task end
        if self
            .job
            .as_ref()
            .map(|job| job.is_finished())
            .unwrap_or(false)
        {
            let job = self.job.take().unwrap();
            if let Some(e) = job.join().unwrap().err() {
                self.build_error = Some(e.to_string());
            }
            self.update_cache();
        }

        // Handle print
        self.gag.read_to_string(&mut self.logs).unwrap();
    }
}
