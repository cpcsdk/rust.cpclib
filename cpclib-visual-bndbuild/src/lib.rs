#![feature(let_chains)]

use std::io::Read;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use cpclib_bndbuild::deps::Rule;
use cpclib_bndbuild::BndBuilder;
use eframe::egui::{self, RichText};
use eframe::epaint::ahash::HashMap;
use eframe::epaint::Color32;
use egui_file::{self, FileDialog};
use itertools::Itertools;

use crate::egui::{Button, Key, KeyboardShortcut, Modifiers, TextEdit};

static CTRL_O: KeyboardShortcut = KeyboardShortcut {
    modifiers: Modifiers::COMMAND,
    key: Key::O
};
static CTRL_S: KeyboardShortcut = KeyboardShortcut {
    modifiers: Modifiers::COMMAND,
    key: Key::S
};
static CTRL_Q: KeyboardShortcut = KeyboardShortcut {
    modifiers: Modifiers::COMMAND,
    key: Key::Q
};
static CTRL_R: KeyboardShortcut = KeyboardShortcut {
    modifiers: Modifiers::COMMAND,
    key: Key::R
};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct BndBuildApp {
    /// The provided filename by the user
    filename: Option<std::path::PathBuf>,

    /// The content of the file loaded
    #[serde(skip)]
    file_content: Option<String>,

    /// Set to true if the rules has been modified but not saved
    #[serde(skip)]
    is_dirty: bool,

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
    gags: (gag::BufferRedirect, gag::BufferRedirect),

    #[serde(skip)]
    request_reload: bool,

    #[serde(skip)]
    request_save: bool,

    /// No need to update the output too often
    #[serde(skip)]
    last_tick: SystemTime,

    #[serde(skip)]
    job: Option<std::thread::JoinHandle<Result<(), cpclib_bndbuild::BndBuilderError>>>
}

impl Default for BndBuildApp {
    fn default() -> Self {
        BndBuildApp {
            filename: None,
            file_content: None,
            is_dirty: false,
            builder_and_layers: None,
            file_error: None,
            build_error: None,
            open_file_dialog: None,
            requested_target: None,
            logs: String::default(),
            request_reload: false,
            request_save: false,
            job: None,
            last_tick: SystemTime::now(),
            gags: (
                gag::BufferRedirect::stdout().unwrap(),
                gag::BufferRedirect::stderr().unwrap()
            )
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
                self.file_content = std::fs::read_to_string(self.filename.as_ref().unwrap()).ok(); // read a second time, but the file exists
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
                    if ui
                        .add(Button::new("Open").shortcut_text(ctx.format_shortcut(&CTRL_O)))
                        .clicked()
                        || ui.input_mut(|i| i.consume_shortcut(&CTRL_O))
                    {
                        let mut dialog = egui_file::FileDialog::open_file(self.filename.clone());
                        dialog.open();
                        self.open_file_dialog = dialog.into();
                        self.file_error = None;
                        ui.close_menu();
                    };

                    if self.filename.is_some() {
                        if ui
                            .add(Button::new("Save").shortcut_text(ctx.format_shortcut(&CTRL_S)))
                            .clicked()
                            || ui.input_mut(|i| i.consume_shortcut(&CTRL_S))
                        {
                            self.request_save = true;
                            ui.close_menu();
                        }
                        if ui
                            .add(Button::new("Reload").shortcut_text(ctx.format_shortcut(&CTRL_R)))
                            .clicked()
                            || ui.input_mut(|i| i.consume_shortcut(&CTRL_R))
                        {
                            self.request_reload = true;
                            ui.close_menu();
                        }
                    }

                    if ui
                        .add(Button::new("Quit").shortcut_text(ctx.format_shortcut(&CTRL_Q)))
                        .clicked()
                        || ui.input_mut(|i| i.consume_shortcut(&CTRL_Q))
                    {
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
            .stick_to_right(false)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.code(&self.logs);
            })
           // .scroll_to_me(Some(egui::Align::Max))
            
            ;
    }

    fn update_code(&mut self, _ctx: &egui::Context, ui: &mut eframe::egui::Ui) {
        ui.vertical_centered(|ui| {
            if self.is_dirty {
                ui.heading("Definition *");
            }
            else {
                ui.heading("Definition");
            }
            if let Some(code) = self.file_content.as_mut() {
                let editor = TextEdit::multiline(code)
                    .code_editor()
                    .hint_text("Expect the yaml rules to build the project.")
                    .desired_width(f32::INFINITY);

                egui::ScrollArea::new([true, true])
                    .max_height(f32::INFINITY)
                    .max_width(f32::INFINITY)
                    .show(ui, |ui| {
                        let output = editor.show(ui);
                        if output.response.changed() {
                            self.is_dirty = true;
                        }
                    });
            }
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
            if let Some(error) = &self.file_error {
                ui.colored_label(Color32::RED, error);
            };

            ui.columns(2, |columns| {
                columns[0].vertical(|ui| {
                    self.update_targets(ctx, ui);
                    self.update_code(ctx, ui);
                });
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

        if self.request_save {
            self.request_save = false;
            let r = std::fs::write(
                self.filename.as_ref().unwrap(),
                self.file_content.as_ref().unwrap()
            );

            if let Some(e) = r.err() {
                self.file_error = e.to_string().into();
            }
            else {
                self.is_dirty = false;
                self.update_cache();
            }
        }

        // Handle target
        if let Some(tgt) = self.requested_target.take() {
            if let Some(builder) = &self.builder_and_layers {
                let builder: &'static BuilderAndCache = unsafe { std::mem::transmute(builder) }; // cheat on lifetime as we know if will live all the time
                self.job = std::thread::spawn(|| builder.borrow_owner().execute(tgt)).into();
                self.logs.clear();
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
        const HZ: u128 = 1000 / 20;
        if self.last_tick.elapsed().unwrap().as_millis() >= HZ {
            self.gags.0.read_to_string(&mut self.logs).unwrap();
            self.gags.1.read_to_string(&mut self.logs).unwrap();
            self.last_tick = SystemTime::now();
        }
    }
}
