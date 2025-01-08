use std::error::Error;
use std::io::Read;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::sync::Arc;

use cpclib_bndbuild::executor::execute;
use cpclib_bndbuild::task::InnerTask;
use tauri::async_runtime::Mutex;

pub mod cache;

use cache::CachedBndBuilder;
use camino::Utf8PathBuf;
use cpclib_bndbuild::app::{BndBuilderCommand, BndBuilderCommandInner};
use cpclib_bndbuild::cpclib_common::itertools::Itertools;
use cpclib_bndbuild::event::BndBuilderObserved;
use cpclib_bndbuild::{commands_list, ALL_APPLICATIONS};
use serde::Serialize;
use tauri::menu::{
    AboutMetadataBuilder, MenuBuilder, MenuEvent, MenuId, MenuItemBuilder, SubmenuBuilder
};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_store::StoreExt;

const STORE_FNAME: &str = "store.json";
const STORE_RECENT_FILES_KEY: &str = "recent_files";
const MAX_RECENT_FILES_LISTED: usize = 10;

const USE_GAGS: bool = true;

#[tauri::command]
async fn empty_gags(app: AppHandle) {
    let state: State<'_, Mutex<BndbuildState>> = app.state();
    let mut lock = state.lock().await;
    {
        match lock.deref_mut() {
            BndbuildState::Loaded(loaded) => {
                let mut content = String::new();
                let content1 = &mut content;
                loaded
                    .gags
                    .as_mut()
                    .map(|gag| gag.0.read_to_string(content1));
                if !content.is_empty() {
                    loaded.builder.emit_stdout(&content);
                }

                content.clear();
                let content1 = &mut content;
                loaded
                    .gags
                    .as_mut()
                    .map(|gag| gag.1.read_to_string(content1));
                if !content.is_empty() {
                    loaded.builder.emit_stderr(&content);
                }
            },
            _ => {}
        }
    }
}

#[tauri::command]
async fn clear_app(
    soft: Option<&str>,
    state: State<'_, Mutex<BndbuildState>>,
    app: AppHandle
) -> Result<(), String> {
    let observers = {
        let state = state.lock().await;
        if let Some(loaded) = state.loaded_state() {
            loaded.builder.observers()
        }
        else {
            use cpclib_bndbuild::event::BndBuilderObserverRc;

            use crate::cache::TauriBndBuilderObserver;

            Arc::new(
                vec![BndBuilderObserverRc::new(TauriBndBuilderObserver::new(
                    &app
                ))]
                .into()
            )
        }
    };

    BndBuilderCommand::new(
        BndBuilderCommandInner::Clear(soft.map(|s| s.to_owned())),
        observers
    )
    .execute()
    .map_err(|e| e.to_string())
}


#[tauri::command]
async fn select_cwd(dname: Utf8PathBuf, app: AppHandle) -> Result<(), String> {
    log::info!("Select a working directory{}", dname);
    let dname = &dname;

    // change the directory
    log::info!("before set_current_dir");
    dbg!(std::env::set_current_dir(dname)
        .map_err(|e| e.to_string()))?;
    log::info!("after set_current_dir");


    // Build the new state
    let state: State<'_, Mutex<BndbuildState>> = app.state();
    let mut state = state.deref().lock().await;
    let gags = match state.deref_mut() {
        BndbuildState::Loaded(state) => {
            state.gags.take()
        },

        _ => if USE_GAGS {
            Some((
                gag::BufferRedirect::stdout().unwrap(),
                gag::BufferRedirect::stderr().unwrap()
            ))
        }
        else {
            None
        }
    };

    *state = BndbuildState::Workdir(WorkdirState{gags});

    log::info!("left cwd");

    Ok(())


}

#[tauri::command]
async fn load_build_file(fname: Utf8PathBuf, app: AppHandle) -> Result<(), ()> {
    log::info!("load_build_file {}", fname);

    let fname = &fname;

    let state: State<'_, Mutex<BndbuildState>> = app.state();
    let mut state = state.deref().lock().await;
    *state = BndbuildState::Empty; // Ensure gags are destroyed
    *state = BndbuildState::load(fname, &app).await;

    match state.deref() {
        BndbuildState::Empty | BndbuildState::Workdir(_)=> {
            unreachable!()
        },
        BndbuildState::Loaded(bndbuild_state_loaded) => {
            #[derive(Clone, Serialize)]
            #[serde(rename_all = "camelCase")]
            struct MsgLoad<'a> {
                fname: String,
                svg: &'a str
            }
            let state = state.loaded_state().unwrap();

            // update the list of previous files
            {
                // retreive previous values
                let store = app.store(STORE_FNAME).unwrap();
                let mut recent_list = if let Some(files) = store.get(STORE_RECENT_FILES_KEY) {
                    files
                        .as_array()
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|v| v.as_str().unwrap().to_owned())
                        .unique() // fix save bug
                        .collect_vec()
                }
                else {
                    Vec::new()
                };

                // remove existing one if any
                if let Some(pos) = recent_list.iter().position(|item| item == fname) {
                    recent_list.remove(pos);
                }

                // ensure it is the first one
                recent_list.insert(0, fname.to_string());

                // keep size limited
                if recent_list.len() > MAX_RECENT_FILES_LISTED {
                    recent_list.truncate(MAX_RECENT_FILES_LISTED);
                }
                store.set(STORE_RECENT_FILES_KEY, recent_list);
                let _ = store.save(); // we ignore the error
            }

            app.emit(
                "file-loaded",
                MsgLoad {
                    fname: fname.to_string(),
                    svg: state.svg()
                }
            )
            .unwrap();
        },
        BndbuildState::LoadError(error) => {
            app.emit(
                "event-stderr",
                format!("Unable to load {}. {}", error.fname.as_str(), error.error)
            )
            .unwrap();
        }
    }

    Ok(())
}

#[tauri::command]
async fn open_contextual_menu_for_target(
    tgt: String,
    window: tauri::Window,
    state: State<'_, Mutex<BndbuildState>>
) -> Result<(), ()> {
    let app_handler = window.app_handle();

    let build_item = MenuItemBuilder::new(format!("Build {tgt}"))
        .id(MenuId::new("CtxBuild"))
        .build(app_handler)
        .unwrap();
    let watch_item = MenuItemBuilder::new(format!("Watch {tgt}"))
        .id(MenuId::new("CtxWatch"))
        .build(app_handler)
        .unwrap();
    let open_item = MenuItemBuilder::new(format!("Open {tgt}"))
        .id(MenuId::new("CtxOpen"))
        .build(app_handler)
        .unwrap();
    let menu = MenuBuilder::new(app_handler)
        .items(&[&build_item, &watch_item, &open_item])
        .build()
        .unwrap();

    window.popup_menu(&menu).unwrap();

    let mut state = state.lock().await;
    let state = state.loaded_state_mut().unwrap();
    state.context_target = Some(Utf8PathBuf::from_str(&tgt).unwrap());
    Ok(())
}

async fn handle_context_menu_event(
    event: &MenuEvent,
    app: &AppHandle,
    state: State<'_, Mutex<BndbuildState>>
) {
    let tgt = {
        let mut state = state.lock().await;
        let state = state.loaded_state_mut(); // because of context menu for debug facilities
        state.map(|s| s.context_target.take())
    };

    if let Some(Some(tgt)) = tgt {
        if event.id() == &MenuId::new("CtxBuild") {
            app.emit("request-execute_target", tgt).unwrap();
        }
        else if event.id() == &MenuId::new("CtxWatch") {
            app.emit("request-watch_target", tgt).unwrap();
        }
        else if event.id() == &MenuId::new("CtxOpen") {
            app.emit("request-open_target", tgt).unwrap();
        }
    }
}

#[tauri::command]
async fn execute_target(tgt: String, state: State<'_, Mutex<BndbuildState>>) -> Result<(), String> {
    log::info!("execute_target {}", tgt);
    // get the builder without keeping the lock
    let builder = {
        let state = state.lock().await;
        let state = state.loaded_state().unwrap();
        Arc::clone(&state.builder.builder)
    };

    let res = builder.execute(tgt).map_err(|e| e.to_string());
    res
}

#[tauri::command]
async fn execute_manual_task(task: String, state: State<'_, Mutex<BndbuildState>>) -> Result<(), String> {
    log::info!("execute_manual_task {}", &task);

    let task = InnerTask::from_str(&task)?;


    let builder = {
        let state = state.lock().await;
        let state = state.loaded_state().unwrap();
        Arc::clone(&state.builder.builder)
    };

    // XXX For an unknown reason STDOUt is not transferred
    let observers = builder.observers();
    let res = execute(&task, &observers);

    log::info!("execute_manual_task {} done", &task);
    res

}

// there is an infinite loop when updating the menu from the rust code (probably because it runs from the menu itself)
#[tauri::command]
async fn update_menu(app: AppHandle) -> Result<(), String> {
    add_menu(&app).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn reload_file(
    app: AppHandle,
    state: State<'_, Mutex<BndbuildState>>
) -> Result<(), String> {
    log::info!("request_reload");
    let file_path = state.lock().await.loaded_state().unwrap().fname.clone();
    app.emit("request-load_build_file", file_path)
        .inspect_err(|e| eprintln!("{e}"));

    Ok(())
}

/// In order to handle frontend side effect, the frontend executes nothing with menu interactions.
/// Instead en event is sent to the frontend, then the GUI go back once again to the backend if needed
async fn add_menu(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let store = app.store(STORE_FNAME)?;

    let open = MenuItemBuilder::new("Open")
        .accelerator("CTRL+O")
        .build(app)?;
    let reload = MenuItemBuilder::new("Reload")
        .enabled(app.state::<Mutex<BndbuildState>>().lock().await.is_loaded())
        .accelerator("CTRL+R")
        .build(app)?;
    let mut select_cwd = MenuItemBuilder::new("Select working directory").accelerator("CTRL+S").build(app)?;

    let mut submenu_recent = SubmenuBuilder::new(app, "Recent files");
    let recent_files_string = if let Some(files) = store.get(STORE_RECENT_FILES_KEY) {
        let files = files.as_array().unwrap();
        files
            .iter()
            .unique()
            .map(|file| file.as_str().unwrap().to_owned())
            .collect_vec()
    }
    else {
        Vec::new()
    };
    let recent_files_menu_item = recent_files_string
        .iter()
        .enumerate()
        .map(|(pos, file)| {
            debug_assert!(MAX_RECENT_FILES_LISTED <= 10, "Need to code this case");
            MenuItemBuilder::new(file.replace("_", "__"))
                .enabled(true)
                .accelerator(format!("CTRL+{pos}"))
                .build(app)
        })
        .collect::<Result<Vec<_>, _>>()?;

    for menu_item in recent_files_menu_item.iter() {
        submenu_recent = submenu_recent.item(menu_item);
    }
    if recent_files_menu_item.is_empty() {
        submenu_recent = submenu_recent.enabled(false);
    }
    let submenu_recent = submenu_recent.build()?;

    let submenu_file = SubmenuBuilder::new(app, "File")
        .item(&open)
        .item(&submenu_recent)
        .item(&reload)
        .separator()
        .item(&select_cwd)
        .quit()
        .build()?;

    let (commands_list, clearable_list) = commands_list();

    let clear_all = MenuItemBuilder::new("All").build(app)?;
    let clearable = ALL_APPLICATIONS
        .iter()
        .filter(|item| item.1)
        .map(|(names, _)| {
            let current = &names[0];
            (current, MenuItemBuilder::new(current).build(app))
        })
        .map(|(name, menu)| menu.map(|menu| (name, menu)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut clear_builder = SubmenuBuilder::new(app, "Clear")
        .separator()
        .item(&clear_all);
    for item in clearable.iter().map(|(_, menu)| menu) {
        clear_builder = clear_builder.item(item);
    }
    let submenu_clear = clear_builder.build()?;

    let submenu_tools = SubmenuBuilder::new(app, "Tools")
        .item(&submenu_clear)
        .build()?;

    let menu = MenuBuilder::new(app)
    .item(&submenu_file)
    .item(&submenu_tools)
    .about(Some(AboutMetadataBuilder::new()
        .authors(Some(["Krusty/Benediction <krusty.benediction@gmail.com>".to_owned()].to_vec()))
        .comments(Some("Builder application for Amstrad CPC and related z80 projects"))
        .name(Some("BNDBuild"))
        .website(Some("https://cpcsdk.github.io/rust.cpclib/bndbuild/"))
        .credits(Some("The project also rely on the following tools and their respective authors: rasm, sjamsplus, vasm, martine, convgeneric, winape, acedl, cpcec, arkos tracker 3"))
        .build()
    ))
    .build()?;

    app.set_menu(menu)?;
    // listen for menu item click events
    app.on_menu_event(move |app, event| {
        if event.id() == open.id() {
            app.emit("request-open", Option::<String>::None)
                .inspect_err(|e| eprintln!("{e}"))
                .unwrap();
        }
        else if event.id() == reload.id() {
            app.emit("request-reload", Option::<String>::None)
                .inspect_err(|e| eprintln!("{e}"))
                .unwrap();
        }
        else if event.id() == select_cwd.id() {
            app.emit("request-select_cwd", Option::<String>::None)
                .inspect_err(|e| eprintln!("{e}"))
                .unwrap();
        }
        else if let Some(pos) = recent_files_menu_item
            .iter()
            .position(|file_item| file_item.id() == event.id())
        {
            let fname = &recent_files_string[pos];
            let fname = Utf8PathBuf::from_str(&fname).unwrap();
            // load_build_file(&fname, cloned_app); // TODO call asynchronously ?
            let _ = app
                .emit("request-load_build_file", fname)
                .inspect_err(|e| eprintln!("{e}"));
        }
        else if let Some(item) = clearable.iter().find(|item| item.1.id() == event.id()) {
            // app.emit("clear_app", item.0);
            app.emit("request-clear", Some(item.0))
                .inspect_err(|e| eprintln!("{e}"))
                .unwrap();
        }
        else if clear_all.id() == event.id() {
            app.emit("request-clear", Option::<String>::None)
                .inspect_err(|e| eprintln!("{e}"))
                .unwrap();
        }
        else {
            handle_context_menu_event(&event, app, app.state());
        }
    });

    Ok(())
}

pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_log::Builder::new().build());
    // let builder = if true {
    // builder
    // .plugin(tauri_plugin_devtools::init())
    // .plugin(tauri_plugin_devtools_app::init())
    // } else {
    // builder
    // };
    builder
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Webview
                ))
                .build()
        )
        .invoke_handler(tauri::generate_handler![
            empty_gags,
            execute_manual_task,
            execute_target,
            load_build_file,
            open_contextual_menu_for_target,
            reload_file,
            select_cwd,
            update_menu,
        ])
        .setup(|app| {
            log::info!("Setup app");
            // Handle application state
            app.manage(Mutex::new(BndbuildState::default()));

            // handle the menus
            tauri::async_runtime::block_on(add_menu(app.handle()))?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Handle the state of the application
#[derive(Default)]
enum BndbuildState {
    /// Nothing has been specified yet
    #[default]
    Empty,
    /// A build script has been loaded
    Loaded(BndbuildStateLoaded),
    /// A build script failed to load
    LoadError(BndbuildStateLoadError),
    /// A working directory has been selected
    Workdir(WorkdirState)
}

impl BndbuildState {
    pub fn loaded_state(&self) -> Option<&BndbuildStateLoaded> {
        match self {
            Self::Loaded(l) => Some(l),
            _ => None
        }
    }

    pub fn loaded_state_mut(&mut self) -> Option<&mut BndbuildStateLoaded> {
        match self {
            Self::Loaded(l) => Some(l),
            _ => None
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded_state().is_some()
    }
}

struct WorkdirState {
    gags: Option<(gag::BufferRedirect, gag::BufferRedirect)> /* TODO remove this as soon as no runner directly print over stdout/stderr */
}

struct BndbuildStateLoaded {
    builder: CachedBndBuilder,
    fname: Utf8PathBuf,
    watched: Option<Vec<Utf8PathBuf>>,
    context_target: Option<Utf8PathBuf>,
    gags: Option<(gag::BufferRedirect, gag::BufferRedirect)> /* TODO remove this as soon as no runner directly print over stdout/stderr */
}

impl Deref for BndbuildStateLoaded {
    type Target = CachedBndBuilder;

    fn deref(&self) -> &Self::Target {
        &self.builder
    }
}

#[derive(Debug)]
struct BndbuildStateLoadError {
    fname: Utf8PathBuf,
    error: String
}

impl BndbuildState {
    pub async fn load<P: Into<Utf8PathBuf>>(path: P, app: &AppHandle) -> Self {
        let fname = path.into();

        match cpclib_bndbuild::BndBuilder::from_path(&fname) {
            Ok((fname, builder)) => {
                match CachedBndBuilder::new(builder, app).await {
                    Ok(builder) => {
                        // TODO add fname to the list of recent files
                        Self::Loaded(BndbuildStateLoaded {
                            builder,
                            fname,
                            watched: None,
                            context_target: None,
                            gags: if USE_GAGS {
                                Some((
                                    gag::BufferRedirect::stdout().unwrap(),
                                    gag::BufferRedirect::stderr().unwrap()
                                ))
                            }
                            else {
                                None
                            }
                        })
                    },
                    Err(error) => Self::LoadError(BndbuildStateLoadError { fname, error })
                }
            },
            Err(err) => {
                Self::LoadError(BndbuildStateLoadError {
                    fname,
                    error: err.to_string()
                })
            },
        }
    }
}
