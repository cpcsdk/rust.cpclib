use std::borrow::BorrowMut;
use std::error::Error;
use std::io::Read;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::sync::Mutex;
use std::sync::Arc;

pub mod cache;

use cache::CachedBndBuilder;
use camino::{Utf8Path, Utf8PathBuf};
use cpclib_bndbuild::app::{BndBuilderCommand, BndBuilderCommandInner};
use cpclib_bndbuild::{commands_list, ALL_APPLICATIONS};
use cpclib_bndbuild::cpclib_common::itertools::Itertools;
use cpclib_bndbuild::event::BndBuilderObserved;
use serde::Serialize;
use tauri::menu::{AboutMetadataBuilder, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_store::StoreExt;

const STORE_FNAME: &str = "store.json";
const STORE_RECENT_FILES_KEY: &str = "recent_files";
const MAX_RECENT_FILES_LISTED: usize = 10;


#[tauri::command]
fn empty_gags(app: AppHandle) {
    let state: State<'_, Mutex<BndbuildState>> = app.state();
    let lock = state.lock();
    if let Ok(mut lock) = lock {
        match lock.deref_mut() {
            BndbuildState::Loaded(loaded) => {
                let mut content = String::new();
                loaded.gags.0.read_to_string(&mut content);
                if !content.is_empty() {loaded.builder.emit_stdout(content);}

                let mut content = String::new();
                loaded.gags.1.read_to_string(&mut content);
                if !content.is_empty() {loaded.builder.emit_stderr(content);}
            },
            _ => {}
            }
    }
}


#[tauri::command]
fn clear_app(soft: Option<&str>, state: State<'_, Mutex<BndbuildState>>, app: AppHandle) -> Result<(), String>{

    let observers = {
        let state = state.lock().unwrap();
        if let Some(loaded) = state.loaded_state() {
            loaded.builder.observers()
        } else {
            use crate::cache::TauriBndBuilderObserver;
            use cpclib_bndbuild::event::ListOfBndBuilderObserverRc;
            use cpclib_bndbuild::event::BndBuilderObserverRc;
            
            Arc::new(vec![BndBuilderObserverRc::new(TauriBndBuilderObserver::new(&app))].into())
        }
    };

    BndBuilderCommand::new(
        BndBuilderCommandInner::Clear(soft.map(|s|s.to_owned())), 
         observers
    ).execute()
    .map_err(|e| e.to_string())

}

#[tauri::command]
fn load_build_file(fname: &Utf8Path, app: AppHandle) {
    let state: State<'_, Mutex<BndbuildState>> = app.state();
    let mut state = state.deref().lock().unwrap();
    *state = BndbuildState::Empty; // Ensure gags are destroyed
    *state = BndbuildState::load(fname, &app);

    match state.deref() {
        BndbuildState::Empty => {
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
}

#[tauri::command]
async fn execute_target(tgt: String, state: State<'_, Mutex<BndbuildState>>) -> Result<(), String> {
    // get the builder without keeping the lock
    let builder = {
        let state = state.lock().unwrap();
        let state = state.loaded_state().unwrap();
        Arc::clone(&state.builder.builder)
    };

    dbg!("Launch cmd execution");
    let res = builder.execute(tgt)
        .map_err(|e| e.to_string());
    dbg!("execution done", &res);
    res
}

// there is an infinite loop when updating the menu from the rust code (probably because it runs from the menu itself)
#[tauri::command]
fn update_menu(app: AppHandle) -> Result<(), String> {
    add_menu(&app).map_err(|e| e.to_string())
}
fn add_menu(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    let store = app.store(STORE_FNAME)?;

    let open = MenuItemBuilder::new("Open")
        .accelerator("CTRL+O")
        .build(app)?;
    let reload = MenuItemBuilder::new("Reload")
        .enabled(
            app.state::<Mutex<BndbuildState>>()
                .lock()
                .unwrap()
                .is_loaded()
        )
        .accelerator("CTRL+R")
        .build(app)?;

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
        .quit()
        .build()?;


    let (commands_list, clearable_list) = commands_list();


    let clear_all = MenuItemBuilder::new("All").build(app)?;
    let clearable = ALL_APPLICATIONS.iter()
        .filter(|item| item.1)
        .map(|(names, _)| {
            let current = &names[0];
            (current, MenuItemBuilder::new(current).build(app))
        })
        .map(|(name, menu)| {
            menu.map(|menu| (name, menu))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut clear_builder = SubmenuBuilder::new(app, "Clear")
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
            let cloned_app = app.clone();
            dbg!("Request load");
            app.dialog()
                .file()
                .add_filter("BNDbuild files", &["build", "bnd", "yml"])
                .pick_file(move |file_path| {
                    if let Some(file_path) = file_path {
                        if let Some(file_path) = file_path.as_path() {
                            let file_path = Utf8Path::from_path(file_path).unwrap().to_owned();
                            load_build_file(&file_path, cloned_app);
                        }
                    }
                });
        }
        else if event.id() == reload.id() {
            let cloned_app = app.clone();
            let file_path = app.state::<Mutex<BndbuildState>>().lock().unwrap().loaded_state().unwrap().fname.clone();
            load_build_file(&file_path, cloned_app);
        }
        else if let Some(pos) = recent_files_menu_item
            .iter()
            .position(|file_item| file_item.id() == event.id())
        {
            let cloned_app = app.clone();
            let fname = &recent_files_string[pos];
            let fname = Utf8PathBuf::from_str(&fname).unwrap();
            load_build_file(&fname, cloned_app); // TODO call asynchronously ?
   //         app.emit("load_build_file", fname); // does not semm to work
        }
        else if let Some(item) = clearable.iter().find(|item| item.1.id() == event.id()) {
            //app.emit("clear_app", item.0);
            clear_app(Some(item.0), app
            .state(), app.clone());
        }
        else if clear_all.id() == event.id() {
            clear_app(None, app.state(), app.clone());
        }
    });

    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            load_build_file,
            execute_target,
            update_menu,
            empty_gags
        ])
        .setup(|app| {
            // Handle application state
            app.manage(Mutex::new(BndbuildState::default()));

            // handle the menus
            add_menu(app.handle())?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Default)]
enum BndbuildState {
    #[default]
    Empty,
    Loaded(BndbuildStateLoaded),
    LoadError(BndbuildStateLoadError)
}

impl BndbuildState {
    pub fn loaded_state(&self) -> Option<&BndbuildStateLoaded> {
        match self {
            Self::Loaded(l) => Some(l),
            _ => None
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded_state().is_some()
    }
}

struct BndbuildStateLoaded {
    builder: CachedBndBuilder, 
    fname: Utf8PathBuf,
    watched: Option<Utf8PathBuf>,
    gags: (gag::BufferRedirect, gag::BufferRedirect) // TODO remove this as soon as no runner directly print over stdout/stderr
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
    pub fn load<P: Into<Utf8PathBuf>>(path: P, app: &AppHandle) -> Self {
        let fname = path.into();

        match cpclib_bndbuild::BndBuilder::from_path(&fname) {
            Ok((fname, builder)) => {
                match CachedBndBuilder::new(builder, app) {
                    Ok(builder) => {
                        // TODO add fname to the list of recent files
                        Self::Loaded(BndbuildStateLoaded {
                            builder,
                            fname,
                            watched: None,
                            gags: (
                                gag::BufferRedirect::stdout().unwrap(),
                                gag::BufferRedirect::stderr().unwrap()
                            )
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
