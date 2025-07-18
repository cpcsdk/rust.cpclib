#![feature(let_chains)]
#![feature(const_trait_impl)]

use std::collections::HashSet;
use std::io::Read;
use std::ops::Deref;
use std::time::{Duration, SystemTime};

use cpclib_bndbuild::BndBuilder;
use cpclib_bndbuild::rules::Rule;
use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use eframe::egui::{self, RichText};
use eframe::epaint::Color32;
use eframe::epaint::ahash::HashMap;
use egui_file::{self, FileDialog};
use itertools::Itertools;

use crate::egui::{Button, Key, KeyboardShortcut, Modifiers};

static CTRL_O: KeyboardShortcut = KeyboardShortcut {
    modifiers: Modifiers::COMMAND,
    logical_key: Key::O
};
static CTRL_S: KeyboardShortcut = KeyboardShortcut {
    modifiers: Modifiers::COMMAND,
    logical_key: Key::S
};
static CTRL_Q: KeyboardShortcut = KeyboardShortcut {
    modifiers: Modifiers::COMMAND,
    logical_key: Key::Q
};
static CTRL_R: KeyboardShortcut = KeyboardShortcut {
    modifiers: Modifiers::COMMAND,
    logical_key: Key::R
};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct BndBuildApp {
    /// The provided filename by the user
    filename: Option<Utf8PathBuf>,

    /// Recently opened files
    recent_files: Vec<Utf8PathBuf>,

    /// Watched target
    #[serde(skip)]
    watched: Option<Utf8PathBuf>,
    #[serde(skip)]
    watch_logs: String,

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
    requested_target: Option<Utf8PathBuf>,
    /// Target hovered to highlight dependencies
    hovered_target: Option<Utf8PathBuf>,

    /// stdout redirection
    #[serde(skip)]
    gags: (gag::BufferRedirect, gag::BufferRedirect),

    #[serde(skip)]
    request_reload: bool,

    #[serde(skip)]
    request_open: bool,

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
            is_dirty: false,
            builder_and_layers: None,
            file_error: None,
            build_error: None,
            open_file_dialog: None,
            requested_target: None,
            hovered_target: None,
            logs: String::default(),
            request_reload: false,
            request_open: false,
            job: None,
            watched: None,
            last_tick: SystemTime::now(),
            gags: (
                gag::BufferRedirect::stdout().unwrap(),
                gag::BufferRedirect::stderr().unwrap() //
            ),
            recent_files: Vec::new(),
            watch_logs: Default::default()
        }
    }
}

/// Store the list of targets level per level
struct Layers<'builder>(Vec<Vec<&'builder Utf8Path>>);
/// Cache up to date information to not recompute it 60 times per seconds
struct UpToDate<'builder>(HashMap<&'builder Rule, bool>);
/// Store the list of dependecies
struct DependencyOf(HashMap<Utf8PathBuf, HashSet<Utf8PathBuf>>);

impl Deref for DependencyOf {
    type Target = HashMap<Utf8PathBuf, HashSet<Utf8PathBuf>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'builder> Deref for Layers<'builder> {
    type Target = Vec<Vec<&'builder Utf8Path>>;

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
    up_to_date: UpToDate<'builder>,
    depends_on: DependencyOf
}

impl<'builder> From<&'builder BndBuilder> for UpToDate<'builder> {
    fn from(builder: &'builder BndBuilder) -> Self {
        let mut map = HashMap::default();
        for rule in builder.rules().iter() {
            map.insert(rule, rule.is_up_to_date::<Utf8PathBuf>(None));
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
                .rev()
                .collect_vec()
        )
    }
}

impl<'builder> From<&'builder BndBuilder> for DependencyOf {
    fn from(builder: &'builder BndBuilder) -> Self {
        let mut dep_of: HashMap<Utf8PathBuf, HashSet<Utf8PathBuf>> = Default::default();
        let targets: Vec<&'builder Utf8Path> = builder.targets();
        for task in targets.iter() {
            let deps = builder.get_layered_dependencies_for(task);
            let deps = deps.into_iter().flatten();
            for dep in deps {
                // println!("{} highlight {}", dep.display(), task.display());
                dep_of
                    .entry(dep.to_path_buf())
                    .or_default()
                    .insert(task.to_path_buf());
            }
        }

        DependencyOf(dep_of)
    }
}

impl<'builder> From<&'builder BndBuilder> for BuilderCache<'builder> {
    fn from(builder: &'builder BndBuilder) -> Self {
        BuilderCache {
            layers: builder.into(),
            up_to_date: builder.into(),
            depends_on: builder.into()
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

impl BuilderAndCache {
    pub fn update(&mut self) {
        self.with_dependent_mut(|builder, prev_cache| {
            let cache: BuilderCache = BuilderCache::from(builder);
            *prev_cache = cache;
        });
    }
}

impl BndBuildApp {
    pub fn new<P: AsRef<Utf8Path>>(cc: &eframe::CreationContext<'_>, path: Option<P>) -> Self {
        let mut app = if let Some(storage) = cc.storage {
            let mut app: BndBuildApp =
                eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
            app.build_error = None;
            app.file_error = None;
            app.builder_and_layers = None;
            app.open_file_dialog = None;
            app.requested_target = None;
            app.hovered_target = None;
            if let Some(fname) = &app.filename {
                app.load(fname.clone());
            }
            app
        }
        else {
            Default::default()
        };
        if let Some(path) = path {
            app.load(path)
        };
        app
    }

    pub fn load<P: AsRef<Utf8Path>>(&mut self, path: P) {
        let path = path.as_ref();
        match cpclib_bndbuild::BndBuilder::from_path(path) {
            Ok((ref path, builder)) => {
                self.filename = Some(path.into());
                self.builder_and_layers = BuilderAndCache::from(builder).into();

                if let Some(position) = self.recent_files.iter().position(|elem| elem == path) {
                    self.recent_files.remove(position);
                }
                self.recent_files.push(path.to_path_buf());
            },
            Err(err) => {
                self.file_error = Some(err.to_string());
            }
        }
    }

    pub fn update_cache(&mut self) {
        if let Some(b) = self.builder_and_layers.as_mut() {
            b.update()
        }
    }
}

impl BndBuildApp {
    fn update_menu(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            if self.job.is_some() {
                ui.disable();
            }

            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui
                        .add(Button::new("Open").shortcut_text(ctx.format_shortcut(&CTRL_O)))
                        .clicked()
                    {
                        self.request_open = true;
                        ui.close_menu();
                    };

                    if !self.recent_files.is_empty() {
                        ui.menu_button("Open Recent", |ui| {
                            for fname in self.recent_files.clone().iter().rev() {
                                if ui
                                    .add(
                                        Button::new(fname.to_string())
                                            .wrap_mode(egui::TextWrapMode::Extend)
                                    )
                                    .clicked()
                                {
                                    self.load(fname);
                                    ui.close_menu();
                                    self.logs.clear();
                                }
                            }
                        });
                    }

                    if self.filename.is_some()
                        && ui
                            .add(Button::new("Reload").shortcut_text(ctx.format_shortcut(&CTRL_R)))
                            .clicked()
                    {
                        self.request_reload = true;
                        ui.close_menu();
                    }
                    ui.separator();

                    if ui
                        .add(Button::new("Quit").shortcut_text(ctx.format_shortcut(&CTRL_Q)))
                        .clicked()
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            })
        });
    }

    fn update_status_and_shortcuts(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            match &self.filename {
                Some(fname) => ui.label(fname.to_string()),
                None => ui.label("No file loaded")
            };

            ui.separator();

            if ui.input_mut(|i| i.consume_shortcut(&CTRL_Q)) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if ui.input_mut(|i| i.consume_shortcut(&CTRL_R)) {
                self.request_reload = true;
            }
            if ui.input_mut(|i| i.consume_shortcut(&CTRL_O)) {
                self.request_open = true;
            }
        });
    }

    fn update_log(&mut self, _ctx: &egui::Context, ui: &mut eframe::egui::Ui) {
        if let Some(error) = self.build_error.as_ref() {
            let txt = RichText::new(error).color(Color32::RED).strong();
            ui.label(txt);
        }

        ui.heading("Output")
            .on_hover_text("Read here the output of the launched commands");
        egui::ScrollArea::new([true, true])
            .max_height(f32::INFINITY)
            .max_width(f32::INFINITY)
            //  .min_height(f32::INFINITY)
            //   .min_width(f32::INFINITY)
            .stick_to_right(false)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.monospace(&self.logs);
            });
    }

    fn update_targets(&mut self, _ctx: &egui::Context, ui: &mut eframe::egui::Ui) {
        if let Some(bnl) = &self.builder_and_layers {
            let default = bnl.borrow_owner().default_target();
            let is_hovered = self.hovered_target.take(); // ensure nothing is hovered unless if a button is really hovered

            ui.vertical_centered(|ui| {
                ui.heading("Tasks")
                    .on_hover_text("Click on the task of interest to execute it.");
                let cache = bnl.borrow_dependent();
                for layer in cache.layers.iter() {
                    ui.horizontal_wrapped(|ui| {
                        //   ui.set_max_width(ui.available_width()/2.0);
                        for tgt in layer.iter() {
                            let rule = bnl.borrow_owner().get_rule(tgt);

                            let txt = tgt.to_string();
                            let txt = if let Some(watched) = self.watched.as_ref()
                                && watched == tgt
                            {
                                format!("{txt} [watched]")
                            }
                            else {
                                txt
                            };
                            let txt = RichText::new(txt).color(Color32::BLACK);
                            // set in bold the default target to see it
                            let txt = if let Some(default) = &default
                                && default == tgt
                            {
                                txt.strong().strong()
                            }
                            else {
                                txt
                            };
                            // set underline the dependencies of the target
                            let txt = if let Some(hovered_tgt) = &is_hovered {
                                if bnl
                                    .borrow_dependent()
                                    .depends_on
                                    .get(&tgt.to_path_buf())
                                    .map(|item| item.contains(hovered_tgt))
                                    .unwrap_or(false)
                                {
                                    txt.underline()
                                }
                                else {
                                    txt
                                }
                            }
                            else {
                                txt
                            };
                            // color depends on the kind of target
                            let color = if let Some(rule) = &rule {
                                if *cache.up_to_date.get(rule).unwrap() {
                                    Color32::LIGHT_BLUE
                                }
                                else {
                                    Color32::LIGHT_RED
                                }
                            }
                            else {
                                Color32::LIGHT_GREEN
                            };

                            // Create the button
                            let button = Button::new(txt).fill(color);

                            // finally add the button
                            let button = ui.add(button);
                            let button = if let Some(rule) = rule {
                                if let Some(help) = rule.help() {
                                    button.on_hover_text(help)
                                }
                                else {
                                    button
                                }
                            }
                            else {
                                button.on_hover_text("Probably a leaf file (right click to open)")
                            };
                            if button.clicked() {
                                self.requested_target = Some(tgt.into());
                                self.logs.clear();
                            }
                            if button.hovered() {
                                self.hovered_target = Some(tgt.into());
                            }
                            button.context_menu(|ui| {
                                if tgt.exists() && ui.button(format!("Open \"{tgt}\"")).clicked() {
                                    match open::that(tgt) {
                                        Ok(_) => {},
                                        Err(e) => {
                                            self.file_error = Some(e.to_string());
                                        }
                                    };
                                    ui.close_menu();
                                }

                                if let Some(watched) = self.watched.as_ref()
                                    && watched == tgt
                                {
                                    if ui.button("Unwatch").clicked() {
                                        self.watched.take();
                                        ui.close_menu();
                                    }
                                }
                                else if ui.button("Watch").clicked() {
                                    self.watched = Some(tgt.to_path_buf());
                                    ui.close_menu();
                                }
                            });
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
                });
                self.update_log(ctx, &mut columns[1]);
            });
        });
    }
}

const REFRESH_DURATION: Duration = Duration::from_millis(100);

impl eframe::App for BndBuildApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // catppuccin_egui::set_theme(&ctx, catppuccin_egui::MOCHA);

        if self.job.is_some() {
            ctx.set_cursor_icon(egui::CursorIcon::Progress);
        }

        self.update_menu(ctx, frame);
        self.update_inner(ctx, frame);
        self.update_status_and_shortcuts(ctx, frame);

        // Handle file opening
        if self.request_open {
            let mut dialog = egui_file::FileDialog::open_file(
                self.filename.clone().map(|p| p.into_std_path_buf())
            );
            dialog.open();
            self.open_file_dialog = dialog.into();
            self.file_error = None;
            self.request_open = false;
        }

        // Handle file opening
        let p = if let Some(dialog) = self.open_file_dialog.as_mut() {
            if dialog.show(ctx).selected() {
                if let Some(path) = dialog.path() {
                    if path.exists() {
                        Some(path.to_owned())
                    }
                    else {
                        self.file_error = format!("{} does not exists.", path.display()).into();
                        None
                    }
                }
                else {
                    None
                }
            }
            else {
                None
            }
        }
        else {
            None
        };
        if let Some(p) = p {
            self.load(Utf8PathBuf::try_from(p).unwrap());
        }

        // Handle reload
        if self.request_reload {
            self.request_reload = false;
            self.logs.clear();
            self.file_error.take();
            self.load(self.filename.clone().unwrap());
            ctx.request_repaint_after(REFRESH_DURATION); // ensure progress will be displayed
        }

        // Handle target
        if let Some(tgt) = self.requested_target.take() {
            if let Some(builder) = &self.builder_and_layers {
                let builder: &'static BuilderAndCache = unsafe { std::mem::transmute(builder) }; // cheat on lifetime as we know if will live all the time
                self.logs.clear();
                self.build_error.take();
                self.job = std::thread::spawn(|| builder.borrow_owner().execute(tgt)).into();
            }
            ctx.request_repaint_after(REFRESH_DURATION); // ensure progress will be displayed
        }

        // Handle task termination
        let force_repaint = if self
            .job
            .as_ref()
            .map(|job| job.is_finished())
            .unwrap_or(false)
        {
            let job = self.job.take().unwrap();
            match job.join() {
                Ok(res) => {
                    if let Some(e) = res.err() {
                        self.build_error = Some(e.to_string());
                    }
                },
                Err(err) => {
                    // self.build_error = Some(err. ().to_string());
                    panic!("{err:?}");
                }
            }

            self.update_cache();
            true
        }
        else {
            false
        };

        // Handle print
        const HZ: u128 = 1000 / 20;
        if force_repaint || self.last_tick.elapsed().unwrap().as_millis() >= HZ {
            self.gags.0.read_to_string(&mut self.logs).unwrap();
            self.gags.1.read_to_string(&mut self.logs).unwrap();
            self.last_tick = SystemTime::now();
        }
        if force_repaint {
            ctx.request_repaint_after(REFRESH_DURATION);
        }

        // force refresh when there is a runnong task
        if self.job.is_some() {
            ctx.request_repaint_after(REFRESH_DURATION);
        }
        else {
            // handle watch if needed
            if let Some(watched) = self.watched.as_ref()
                && self
                    .builder_and_layers
                    .as_ref()
                    .map(|bnl| bnl.borrow_owner().outdated(watched).unwrap_or(false))
                    .unwrap_or(false)
            {
                self.watch_logs
                    .push_str(&format!("{watched} needs to be rebuilt"));
                if self.requested_target.is_some() {
                    self.watch_logs.push_str(&format!(
                        "Build delayed in favor of {}",
                        self.requested_target.as_ref().unwrap()
                    ));
                }
                else {
                    self.requested_target = Some(watched.to_owned());
                }
            }
        }

        // handle
    }
}
