use std::collections::HashSet;
use std::marker::PhantomData;
use std::time::Duration;

use bon::builder;
use clap::{ArgAction, Command, CommandFactory, Parser, Subcommand, ValueEnum};
use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::itertools::Itertools;
use cpclib_common::parse_value;
use delegate;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
#[cfg(windows)]
use fs_extra;
use xcap::image::{open, ImageBuffer, Rgba};
use xcap::Window;

use crate::ace_config::AceConfig;
use crate::delegated::{clear_base_cache_folder, DelegatedRunner};
use crate::embedded::EmbeddedRoms;
use crate::event::EventObserver;
use crate::runner::emulator::{Emulator};
use crate::runner::runner::RunnerWithClap;
use crate::runner::Runner;

type Screenshot = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq)]
pub enum AmstradRom {
    Orgams,
    Unidos
}

/// Read a rasm debug file and convert it in winape sym string
pub fn rasm_debug_to_winape_sym(src: &Utf8Path) -> std::io::Result<String> {
    let content = std::fs::read_to_string(src)?;
    Ok(content
        .split(";")
        .filter(|code| code.starts_with("label") | code.starts_with("alias"))
        .map(|code| {
            let mut spliter = code.split_ascii_whitespace();
            spliter.next(); // consume alias or label

            let label = spliter.next().unwrap().replace('.', "_");
            let value = spliter.next().unwrap();
            let value = &mut value.as_bytes();
            let value = parse_value::<_, ()>(value).unwrap();

            format!("{label} #{value:.X}")
        })
        .join("\n"))
}

#[derive(Debug, bon::Builder)]
pub struct EmulatorConf {
    pub(crate) drive_a: Option<Utf8PathBuf>,
    pub(crate) drive_b: Option<Utf8PathBuf>,
    pub(crate) snapshot: Option<Utf8PathBuf>,

    #[builder(default)]
    pub(crate) roms_configuration: HashSet<AmstradRom>,

    #[builder(default)]
    pub(crate) debug_files: Vec<Utf8PathBuf>,

    pub(crate) auto_run: Option<String>,

    pub(crate) memory: Option<u32>
}

impl EmulatorConf {
    /// Generate the args for the corresponding emulator
    pub fn args_for_emu(&self, emu: &Emulator) -> Result<Vec<String>, String> {
        let mut args = Vec::default();

        if let Some(drive_a) = &self.drive_a {
            match emu {
                Emulator::Ace(_) => args.push(drive_a.to_string()),
                Emulator::Cpcec(_) => args.push(drive_a.to_string()),
                Emulator::SugarBoxV2(_) => args.push(drive_a.to_string()),
                Emulator::Winape(_) => args.push(emu.wine_compatible_fname(drive_a)?.to_string()),
                Emulator::Amspirit(_) => args.push(emu.wine_compatible_fname(drive_a)?.to_string())
            }
        }

        if let Some(drive_b) = &self.drive_b {
            match emu {
                Emulator::Ace(_) => return Err("Drive B not yet handled".to_owned()),
                Emulator::Cpcec(_) => return Err("Drive B not yet handled".to_owned()),
                Emulator::Winape(_) => return Err("Drive B not yet handled".to_owned()),
                Emulator::Amspirit(_) => return Err("Drive B not yet handled".to_owned()),
                Emulator::SugarBoxV2(_) => return Err("Drive B not yet handled".to_owned()),
            }
        }

        if let Some(sna) = &self.snapshot {
            match emu {
                Emulator::Ace(ace_version) => args.push(sna.to_string()),
                Emulator::Cpcec(cpcec_version) => args.push(sna.to_string()),
                Emulator::SugarBoxV2(_) => args.push(sna.to_string()),
                Emulator::Winape(winape_version) => {
                    let fname = emu.wine_compatible_fname(sna)?;
                    args.push(format!("/SN:{fname}"));
                },
                Emulator::Amspirit(v) => {
                    let fname = emu.wine_compatible_fname(sna)?;
                    args.push(format!("--file={}", fname));
                }
            }
        }

        // is it really usefull ? seems it is really done by playing with the conf files
        if !self.roms_configuration.is_empty() {
            match emu {
                Emulator::Ace(_) => todo!(),
                Emulator::Cpcec(_) => todo!(),
                Emulator::Winape(_) => todo!(),
                Emulator::Amspirit(_) => todo!(),
                Emulator::SugarBoxV2(_) => todo!(),
            }
        }

        if !self.debug_files.is_empty() {
            match emu {
                Emulator::Ace(_) => {
                    for fname in &self.debug_files {
                        args.push(fname.to_string());
                    }
                },
                Emulator::Winape(_) => {
                    eprintln!("Breapoints are currently ignored. TODO convert them in the appropriate format");
                    let mut sym_string = String::new();
                    for rasm_fname in &self.debug_files {
                        if !sym_string.is_empty() && !sym_string.ends_with('\n') {
                            sym_string.push('\n');
                        }
                        sym_string.push_str(&rasm_debug_to_winape_sym(rasm_fname).unwrap());
                    }

                    if !sym_string.is_empty() {
                        let tempfile = camino_tempfile::Builder::new()
                            .suffix(".winape.sym")
                            .tempfile().unwrap();
                        let path = tempfile.into_temp_path();
                        let path = path.keep().unwrap();
                        std::fs::write(&path, sym_string).unwrap();
                        let fname = emu.wine_compatible_fname(&path)?;
                        args.push(format!("/SYM:{fname}"));
                    }


                }
                _ => eprintln!("Debug files are currently ignored. TODO convert them in the appropriate format")
            }
        }

        if let Some(memory) = &self.memory {
            if let Emulator::Cpcec(_) = emu {
                let arg = match memory {
                    64 => "-k0",
                    128 => "-k1",
                    192 => "-k2",
                    320 => "-k3",
                    576 => "-k4",
                    1088 => "-k5",
                    2112 => "-k6",
                    _ => unimplemented!()
                };
                args.push(arg.to_owned());
            }
        }

        if let Some(run) = &self.auto_run {
            match emu {
                Emulator::Ace(_) => {
                    args.push("-autoRunFile".to_owned());
                    args.push(run.clone())
                },
                Emulator::Winape(_) => {
                    args.push(format!("/A:{run}"));
                },
                Emulator::Cpcec(_) => {
                    // is it automatic ?
                },
                Emulator::Amspirit(_) => {
                    args.push(format!("--run={run}"));
                },
                Emulator::SugarBoxV2(_) => unimplemented!()
            }
        }

        Ok(args)
    }
}

pub fn start_emulator(emu: &Emulator, conf: &EmulatorConf) -> Result<(), String> {
    let args = conf.args_for_emu(emu)?;
    let app = emu.configuration();

    let runner = DelegatedRunner::new(app, emu.get_command().into());
    runner.inner_run(&args, &())
}

pub fn get_emulator_window(emu: &Emulator) -> Window {
    let windows = Window::all().unwrap();
    let mut windows = windows
        .into_iter()
        .filter(|win| emu.window_name_corresponds(win.title()))
        .collect_vec();

    match windows.len() {
        0 => panic!("No window emulator found"),
        1 => windows.pop().unwrap(),
        _ => {
            eprintln!("There are several available windows. I pick one, but it may be wrong");
            windows.pop().unwrap()
        }
    }
}

trait UsedEmulator: Sized {
    fn screenshot(robot: &mut RobotImpl<Self>) -> ImageBuffer<Rgba<u8>, Vec<u8>>
    where Self: Sized {
        robot.window.capture_image().unwrap()
    }
}

struct AceUsedEmulator {}
struct CpcecUsedEmulator {}
struct WinapeUsedEmulator {}
struct AmspiritUsedEmulator {}
struct SugarBoxV2UsedEmulator {}

impl UsedEmulator for AceUsedEmulator {
    // here we delegate the creation of screenshot to Ace to avoid some issues i do not understand
    fn screenshot(robot: &mut RobotImpl<Self>) -> Screenshot {
        let folder = robot.emu.screenshots_folder();
        let before_screenshots: HashSet<_> = glob::glob(folder.join("*.png").as_str())
            .unwrap()
            .map(|p| p.unwrap().as_path().to_owned())
            .collect();

        // handlekey press
        robot.click_key(Key::F10);

        let mut file = None;
        while file.is_none() {
            RobotImpl::<AceUsedEmulator>::wait_a_bit();
            let after_screenshots: HashSet<_> = glob::glob(folder.join("*.png").as_str())
                .unwrap()
                .map(|p| p.unwrap().as_path().to_owned())
                .collect();
            let mut new_screenshots = after_screenshots
                .difference(&before_screenshots)
                .cloned()
                .collect_vec();
            if !new_screenshots.is_empty() {
                file = Some(new_screenshots.pop().unwrap());
            }
        }

        let file = file.as_ref().unwrap();
        let mut im = xcap::image::open(file);
        while im.is_err() {
            RobotImpl::<AceUsedEmulator>::wait_a_bit();
            im = xcap::image::open(file);
        }
        let im = im.unwrap().into_rgba8();
        std::fs::remove_file(file).unwrap();
        im
    }
}
impl UsedEmulator for CpcecUsedEmulator {}
impl UsedEmulator for WinapeUsedEmulator {}
impl UsedEmulator for SugarBoxV2UsedEmulator {}
impl UsedEmulator for AmspiritUsedEmulator {}

struct RobotImpl<E: UsedEmulator> {
    pub(crate) window: Window,
    pub(crate) enigo: Enigo,
    pub(crate) emu: Emulator,
    _emu: PhantomData<E>
}

pub enum Robot {
    Ace(RobotImpl<AceUsedEmulator>),
    Cpcec(RobotImpl<CpcecUsedEmulator>),
    Winape(RobotImpl<WinapeUsedEmulator>),
    Amspirit(RobotImpl<AmspiritUsedEmulator>),
    SugarboxV2(RobotImpl<SugarBoxV2UsedEmulator>),
}

impl From<RobotImpl<AceUsedEmulator>> for Robot {
    fn from(value: RobotImpl<AceUsedEmulator>) -> Self {
        Self::Ace(value)
    }
}

impl From<RobotImpl<CpcecUsedEmulator>> for Robot {
    fn from(value: RobotImpl<CpcecUsedEmulator>) -> Self {
        Self::Cpcec(value)
    }
}

impl From<RobotImpl<WinapeUsedEmulator>> for Robot {
    fn from(value: RobotImpl<WinapeUsedEmulator>) -> Self {
        Self::Winape(value)
    }
}

impl From<RobotImpl<AmspiritUsedEmulator>> for Robot {
    fn from(value: RobotImpl<AmspiritUsedEmulator>) -> Self {
        Self::Amspirit(value)
    }
}

impl From<RobotImpl<SugarBoxV2UsedEmulator>> for Robot {
    fn from(value: RobotImpl<SugarBoxV2UsedEmulator>) -> Self {
        Self::SugarboxV2(value)
    }
}


impl<E: UsedEmulator> From<(Window, Enigo, &Emulator)> for RobotImpl<E> {
    fn from(value: (Window, Enigo, &Emulator)) -> Self {
        Self {
            window: value.0,
            enigo: value.1,
            emu: value.2.clone(),
            _emu: PhantomData::<E>
        }
    }
}

impl Robot {
    delegate::delegate! {
        to match self {
            Robot::Ace(r) => r,
            Robot::Cpcec(r) => r,
            Robot::Winape(r) => r,
            Robot::Amspirit(r) => r,
            Robot::SugarboxV2(r) => r,
        } {
            fn handle_orgams(
                &mut self,
                drivea: Option<&str>,
                albireo: Option<&str>,
                src: &str,
                dst: Option<&str>,
                jump: bool,
                edit: bool
            ) -> Result<(), String>;
            fn type_text(&mut self, s: &str);
            fn close(&mut self);
        }

    }

    pub fn new(emu: &Emulator, window: Window, enigo: Enigo) -> Self {
        match emu {
            Emulator::Ace(_) => RobotImpl::<AceUsedEmulator>::from((window, enigo, emu)).into(),
            Emulator::Cpcec(_) => RobotImpl::<CpcecUsedEmulator>::from((window, enigo, emu)).into(),
            Emulator::Winape(_) => RobotImpl::<WinapeUsedEmulator>::from((window, enigo, emu)).into(),
            Emulator::Amspirit(_) => RobotImpl::<AmspiritUsedEmulator>::from((window, enigo, emu)).into(),
            Emulator::SugarBoxV2(_) => RobotImpl::<SugarBoxV2UsedEmulator>::from((window, enigo, emu)).into(),
        }
    }

    pub fn handle_raw_text<S: AsRef<str>>(&mut self, text: S) {
        let text = text.as_ref();
        let text = text.replace(r"\n", "\n");
        let text = &text;

        self.type_text(text);
    }
}

impl<E: UsedEmulator> RobotImpl<E> {
    pub fn screenshot(&mut self) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        E::screenshot(self)
    }
}

impl<E: UsedEmulator> RobotImpl<E> {
    pub fn close(&mut self) {
        self.enigo.key(Key::Alt, Direction::Press).unwrap();
        self.enigo.key(Key::F4, Direction::Click).unwrap();
        self.enigo.key(Key::Alt, Direction::Release).unwrap();
    }
}

impl<E: UsedEmulator> RobotImpl<E> {
    fn type_text(&mut self, txt: &str) {
        for c in txt.chars() {
            self.click_char(dbg!(c));
        }
    }

    fn click_keys(&mut self, keys: &[Key]) {
        for key in keys {
            self.click_key(*key);
        }
    }

    fn click_char(&mut self, c: char) {
        let key = match c {
            '\n' => Key::Return,
            _ => Key::Unicode(c)
        };
        self.click_key(key)
    }

    fn press_control_char(&mut self, c: char) {
        self.enigo
            .key(Key::Control, enigo::Direction::Press)
            .unwrap();
        Self::wait_a_bit();
        self.enigo
            .key(Key::Unicode(c), enigo::Direction::Press)
            .unwrap();
        Self::wait_a_bit();
        self.enigo
            .key(Key::Unicode(c), enigo::Direction::Release)
            .unwrap();
        Self::wait_a_bit();
        self.enigo
            .key(Key::Control, enigo::Direction::Release)
            .unwrap();
    }

    #[cfg(target_os = "linux")]
    fn click_key(&mut self, key: Key) {
        self.enigo.key(key, enigo::Direction::Press).unwrap();
        Self::wait_a_bit();
        Self::wait_a_bit();
        self.enigo.key(key, enigo::Direction::Release).unwrap();
        Self::wait_a_bit();
        Self::wait_a_bit();
    }

    #[cfg(windows)]
    fn click_key(&mut self, key: Key) {
        dbg!(&key);

        #[cfg(windows)]
        match key {
            // https://boostrobotics.eu/windows-key-codes/
            Key::Unicode(v) if v.is_ascii_digit() => {
                if false {
                    let nb = v as u32 - '0' as u32;

                    self.enigo
                        .key(Key::RShift, enigo::Direction::Press)
                        .unwrap();
                    Self::wait_a_bit();
                    Self::wait_a_bit();

                    let lut = ['à', '&', 'é', '"', '\'', '(', '-', 'è', '_', 'ç'][nb as usize];
                    dbg!(nb, lut);
                    let key = Key::Unicode(lut);

                    self.enigo.key(key, enigo::Direction::Press).unwrap();
                    Self::wait_a_bit();
                    Self::wait_a_bit();
                    self.enigo.key(key, enigo::Direction::Release).unwrap();

                    self.enigo
                        .key(Key::RShift, enigo::Direction::Release)
                        .unwrap();
                    Self::wait_a_bit();
                }

                self.enigo.text(dbg!(&format!("{v}"))).unwrap();
            },
            _ => {
                dbg!(key);
                self.enigo.key(key, enigo::Direction::Press).unwrap();
                Self::wait_a_bit();
                self.enigo.key(key, enigo::Direction::Release).unwrap();
                Self::wait_a_bit();
            }
        };
    }
}

impl<E: UsedEmulator> RobotImpl<E> {
    pub fn unidos_select_drive(&mut self, drivea: Option<&str>, albireo: Option<&str>) {
        if drivea.is_some() {
            self.type_text("load\"dfa:");
            self.click_key(Key::Return);
        }
        else if albireo.is_some() {
            self.type_text("load\"sd:");
            self.click_key(Key::Return);
        }
        else {
            panic!("No storage selected");
        }
    }
}

impl<E: UsedEmulator> RobotImpl<E> {
    pub fn handle_orgams(
        &mut self,
        drivea: Option<&str>,
        albireo: Option<&str>,
        src: &str,
        dst: Option<&str>,
        jump: bool,
        edit: bool
    ) -> Result<(), String> {
        // we assume that we do not need to select the window as well launched it. it is already selected

        self.unidos_select_drive(drivea, albireo);

        let load_res = if src.ends_with('o') || src.ends_with('O') {
            // here we directly load an orgams file
            self.orgams_load(src)
        }
        else {
            // here we need to import
            self.orgams_import(src)
        }
        .map_err(|screen| (format!("Error while loading {}", src), screen));

        let next_res = if let Ok(()) = load_res {
            // No need to do more when we want to edit a file
            if edit {
                return Ok(());
            }

            self.orgams_assemble(src)
                .map_err(|screen| ("Error while assembling".to_string(), screen))
                .and_then(|_| {
                    if jump {
                        self.orgams_jump()
                            .map_err(|screen| ("Error while jumping".to_string(), screen))
                    }
                    else {
                        self.orgams_save(dst)
                            .map_err(|screen| ("Error while saving".to_string(), screen))
                    }
                })
        }
        else {
            load_res
        };

        next_res.map_err(|(msg, screen)| {
            let path = {
                let tempfile = camino_tempfile::Builder::new()
                    .prefix("bnd_stuff")
                    .suffix(".png")
                    .tempfile()
                    .unwrap();
                let (_f, path) = tempfile.keep().unwrap();
                path
            };
            screen.save(&path).unwrap();
            open(&path).unwrap();
            format!("An error occurred.\n{msg}\nLook at {}.", path)
        })
    }

    fn orgams_jump(&mut self) -> Result<(), Screenshot> {
        self.click_char('j');
        Ok(())
    }

    fn orgams_wait_import(&mut self) -> Result<(), Screenshot> {
        self.orgams_wait_save()
    }

    fn orgams_wait_save(&mut self) -> Result<(), Screenshot> {
        loop {
            let screen = self.screenshot();
            let coord_of_interest = (0, 48);
            let pix_of_interest = screen.get_pixel(coord_of_interest.0, coord_of_interest.1);

            if !(pix_of_interest == &Rgba([1, 1, 1, 255])
                || pix_of_interest == &Rgba([1, 2, 1, 255]))
            {
                println!("  done.");
                return Ok(()); // success
            }

            let coord_of_interest = (56, 508); // XXX Plus (63, 508)
            let pix_of_interest = screen.get_pixel(coord_of_interest.0, coord_of_interest.1);
            if pix_of_interest == &Rgba([247, 247, 247, 255])
                || pix_of_interest == &Rgba([255, 243, 249, 255])
            {
                return Err(screen);
            }
        }
    }

    fn orgams_save(&mut self, dst: Option<&str>) -> Result<(), Screenshot> {
        println!("> Save result");
        // handle saving
        self.click_key(Key::Unicode('b'));
        std::thread::sleep(Duration::from_millis(2000));
        if let Some(dst) = dst {
            self.type_text(dst);
            self.click_key(Key::Return);
        }
        else {
            self.click_key(Key::Return);
            std::thread::sleep(Duration::from_millis(1000));
            self.click_key(Key::Return);
        }
        println!("  Filename provided.");

        // wait save is done
        std::thread::sleep(Duration::from_millis(3000 / 2)); // we consider it takes at minimum to assemble a file
        self.orgams_wait_save()
    }

    fn orgams_import(&mut self, src: &str) -> Result<(), ImageBuffer<Rgba<u8>, Vec<u8>>> {
        self.type_text("ùo");
        self.click_key(Key::Return);

        std::thread::sleep(Duration::from_secs(1)); // we wait one second for orgams loading

        self.press_control_char('i');
        std::thread::sleep(Duration::from_secs(1)); // we wait one second for orgams loading

        self.type_text(src);
        self.click_key(Key::Return);

        self.orgams_wait_import()
    }

    fn orgams_load(&mut self, src: &str) -> Result<(), ImageBuffer<Rgba<u8>, Vec<u8>>> {
        // Open orgams
        println!("> Launch orgams and open file {src} from drive a");

        // French setup ?
        let mut keys = vec![Key::Unicode('ù')];
        keys.extend_from_slice(&[Key::Unicode('o'), Key::Unicode(','), Key::Unicode('"')]);
        for c in src.chars() {
            keys.push(Key::Unicode(c));
        }
        keys.extend_from_slice(&[Key::Unicode('"'), Key::Return]);
        self.click_keys(&keys);

        let res = self.wait_orgams_loading();
        println!("  done.");

        res
    }

    fn orgams_assemble(&mut self, src: &str) -> Result<(), ImageBuffer<Rgba<u8>, Vec<u8>>> {
        println!("> Assemble {src}");
        self.press_control_char('1');

        self.wait_orgams_assembling();
        println!("  done.");

        let result: ImageBuffer<Rgba<u8>, Vec<u8>> = self.window.capture_image().unwrap();

        if result
            .pixels()
            .any(|p| p == &Rgba([99, 247, 99, 255]) || p == &Rgba([113, 243, 107, 255]))
        {
            Ok(())
        }
        else {
            Err(result)
        }
    }

    pub fn wait_a_bit() {
        std::thread::sleep(Duration::from_millis(1000 / 20));
    }

    fn wait_orgams_assembling(&mut self) {
        let coord_of_interest = (0, 200);
        let mut finished = false;
        while !finished {
            std::thread::sleep(Duration::from_millis(1000 / 10));
            let screen = E::screenshot(self);
            let pix_of_interest = screen.get_pixel(coord_of_interest.0, coord_of_interest.1);

            finished = !(pix_of_interest == &Rgba([1, 1, 1, 255])
                || pix_of_interest == &Rgba([1, 2, 1, 255]));
        }
    }

    fn wait_orgams_loading(&mut self) -> Result<(), Screenshot> {
        #[derive(PartialEq)]
        enum State {
            Basic,
            Loading,
            Loaded
        };

        // we check a specific pixel that goes from blue to black then purple
        let mut state = State::Basic;
        while state != State::Loaded {
            let screen = E::screenshot(self);
            let coord_of_interest = (8, 48); // 2 to work on Amstrad plus and old
            let pix_of_interest = screen.get_pixel(coord_of_interest.0, coord_of_interest.1);

            state = match state {
                State::Basic => {
                    if pix_of_interest == &Rgba([1, 1, 99, 255])
                        || pix_of_interest == &Rgba([1, 2, 107, 255])
                    {
                        State::Basic
                    }
                    else {
                        State::Loading
                    }
                },
                State::Loading => {
                    if pix_of_interest == &Rgba([1, 1, 1, 255])
                        || pix_of_interest == &Rgba([1, 2, 1, 255])
                    {
                        State::Loading
                    }
                    else {
                        State::Loaded
                    }
                },
                State::Loaded => State::Loaded
            };

            let coord_of_interest = (56, 508); // XXX Plus (63, 508)
            let pix_of_interest = screen.get_pixel(coord_of_interest.0, coord_of_interest.1);
            if pix_of_interest == &Rgba([247, 247, 247, 255])
                || pix_of_interest == &Rgba([255, 243, 249, 255])
            {
                return Err(screen);
            }

            std::thread::sleep(Duration::from_millis(1000 / 10));
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct EmuCli {
    #[arg(
        short = 'a',
        long = "drivea",
        value_name = "DISCA",
        help = "Disc A image"
    )]
    drive_a: Option<String>,

    #[arg(
        short = 'b',
        long = "driveb",
        value_name = "DISCB",
        help = "Disc B image"
    )]
    drive_b: Option<String>,

    #[arg(
        long = "albireo",
        value_name = "FOLDER",
        help = "Albireo content (only for ACE) - WARNING. It is destructive as it completely replaces the existing content"
    )]
    albireo: Option<String>,

    #[arg(
        long = "snapshot",
        value_name = "SNAPSHOT",
        help = "Specify the snapshot to launch"
    )]
    snapshot: Option<String>,

    #[arg(short, long, value_parser = clap::builder::PossibleValuesParser::new(&["64", "128", "192", "256", "320", "576", "1088", "2112"]))]
    memory: Option<String>,

    #[arg(short, long, default_value = "ace", alias = "emu")]
    emulator: Emu,

    #[arg(short, long, action = ArgAction::SetTrue, help = "Keep the emulator open after the interaction")]
    keepemulator: bool,

    #[arg(short, long, action = ArgAction::SetTrue, help = "Clear the cache folder")]
    clear_cache: bool,

    #[arg(short, long, action = ArgAction::Append, help = "rasm-compatible debug file (for ace ATM)")]
    debug: Vec<Utf8PathBuf>,

    #[arg(short='r', long, aliases = ["auto", "run", "autoRunFile"], action = ArgAction::Set, help = "The file to run" )]
    auto_run_file: Option<String>,

    #[arg(long, action=ArgAction::Append, help="List the ROMS to deactivate")]
    disable_rom: Vec<AmstradRom>,

    #[command(subcommand)]
    command: Commands
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum Emu {
    Ace,
    Winape,
    Cpcec,
    Amspirit,
    Sugarbox
}

use clap::Args;

#[derive(Args, Clone, Debug)]
pub struct OrgamsCli {
    /// lists test values
    #[arg(short, long, help = "Filename to assemble or edit")]
    src: String,

    #[arg(
        short,
        long,
        help = "Filename to save after assembling. By default use the one provided by orgams"
    )]
    dst: Option<String>,

    #[arg(
            short,
            long,
            action = ArgAction::SetTrue,
            requires = "dst",
            help = "Convert a Z80 source file into an ascii orgams file"
        )]
    basm2orgams: bool,

    #[arg(
            short,
            long,
            action = ArgAction::SetTrue,
            alias = "monogams",
            conflicts_with_all = ["dst", "jump"],
            help = "Launch the editor in an emulator"
        )]
    edit: bool,

    #[arg(
            short,
            long,
            action = ArgAction::SetTrue,
            conflicts_with_all = ["dst", "edit"],
            help="Jump on the program instead of saving it")]
    jump: bool
}

#[derive(Subcommand, Clone, Debug)]
pub enum Commands {
    Orgams(OrgamsCli),

    Run {
        #[arg(short, long, help = "Simple text to type")]
        text: Option<String>
    }
}

pub const EMUCTRL_CMD: &str = "cpc";

pub struct EmuControlledRunner<E: EventObserver> {
    command: Command,
    _phantom: PhantomData<E>
}

impl<E: EventObserver> Default for EmuControlledRunner<E> {
    fn default() -> Self {
        Self {
            command: EmuCli::command(),
            _phantom: Default::default()
        }
    }
}

impl<E: EventObserver> Runner for EmuControlledRunner<E> {
    type EventObserver = E;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &E) -> Result<(), String> {
        let mut itr = itr.iter().map(|s| s.as_ref()).collect_vec();
        itr.insert(0, "cpc");
        let cli = EmuCli::parse_from(itr);

        handle_arguments(cli, o)
    }

    fn get_command(&self) -> &str {
        EMUCTRL_CMD
    }
}

impl<E: EventObserver> RunnerWithClap for EmuControlledRunner<E> {
    fn get_clap_command(&self) -> &clap::Command {
        &self.command
    }
}

pub fn handle_arguments<E: EventObserver>(mut cli: EmuCli, o: &E) -> Result<(), String> {
    if cli.clear_cache {
        clear_base_cache_folder()
            .map_err(|e| format!("Unable to clear the cache folder. {}", e))?;
    }

    let builder = EmulatorConf::builder()
        .maybe_drive_a(cli.drive_a.clone().map(|a| a.into()))
        .maybe_drive_b(cli.drive_b.clone().map(|a| a.into()))
        .maybe_snapshot(cli.snapshot.clone().map(|a| a.into()))
        .debug_files(cli.debug.clone())
        .maybe_auto_run(cli.auto_run_file.clone())
        .maybe_memory(cli.memory.clone().map(|v| v.parse::<u32>().unwrap()));
    let conf = builder.build();

    let emu = match cli.emulator {
        Emu::Ace => Emulator::Ace(Default::default()),
        Emu::Winape => Emulator::Winape(Default::default()),
        Emu::Cpcec => Emulator::Cpcec(Default::default()),
        Emu::Amspirit => Emulator::Amspirit(Default::default()),
        Emu::Sugarbox => Emulator::SugarBoxV2(Default::default())
    };

    {
        // ensure emulator is installed to properly handle its setup
        let conf = emu.configuration();
        if !conf.is_cached() {
            conf.install(o)?;
        }
    }

    // setup emulator
    // todo standardize that to shorten code and avoid copy paste
    if cli.emulator == Emu::Ace {
        // copy the non standard roms and configure the emu (at least ace)
        let ace_conf_path = emu.ace_version().unwrap().config_file(); // todo get it programmatically
        let mut ace_conf = AceConfig::open(&ace_conf_path);

        if let Some(mem) = &cli.memory {
            ace_conf.set("RAM", mem);
        }
        else {
            ace_conf.set("RAM", 128);
        }

        // Ensure system is French. TODO handle that properly for foreign partners !
        ace_conf.set(
            "OS",
            emu.configuration::<E>()
                .cache_folder()
                .join("private/firmware/OS6128_FR.rom")
        );
        ace_conf.set("KTRANS", 1);
        ace_conf.set("KGTRANS", 1);

        let extra_roms: &[(AmstradRom, &[(&str, usize)])] = &[
            (
                AmstradRom::Unidos,
                &[
                    ("unidos.rom", 7),
                    ("nova.rom", 8),
                    ("albireo.rom", 9),
                    ("parados12.fixedanyslot.fixedname.quiet.rom", 10)
                ]
            ),
            (AmstradRom::Orgams, &[("Orgams_FF240128.e0f", 15)])
        ];
        // for fname in EmbeddedRoms::iter() {
        // println!("{fname}");
        // }
        for (kind, roms) in extra_roms {
            let remove = cli.disable_rom.contains(kind);

            // a minimum ammount of memory is required
            if !remove && kind == &AmstradRom::Orgams && cli.memory.is_none() {
                ace_conf.set("RAM", 576);
            }

            for (rom, slot) in roms.iter() {
                let dst = emu.roms_folder().join(rom);
                let exists = dst.exists();

                if !exists && !remove {
                    let src = format!("roms://{rom}");
                    println!("Install {} in {}", src, dst);
                    let data =
                        EmbeddedRoms::get(&src).unwrap_or_else(|| panic!("{src} not embedded"));
                    std::fs::write(&dst, data.data).unwrap();
                }
                else if exists && remove {
                    std::fs::remove_file(&dst).unwrap();
                }

                let key = format!("ROM{slot}");
                if remove {
                    ace_conf.remove(&key);
                }
                else {
                    ace_conf.set(key, dst.to_string());
                }
            }
        }
        ace_conf.save().unwrap();
    }

    let albireo_backup_and_original = {
        if emu.is_ace() {
            let emu_folder = emu.albireo_folder();
            let backup_folder = emu_folder
                .parent()
                .unwrap()
                .join(emu_folder.file_name().unwrap().to_owned() + ".bak");

            if backup_folder.exists() {
                std::fs::remove_dir_all(&backup_folder).unwrap();
            }

            if emu_folder.exists() {
                std::fs::rename(&emu_folder, &backup_folder).unwrap();
            }

            Some((backup_folder, emu_folder))
        }
        else {
            None
        }
    };

    if emu.is_ace() {
        let emu_folder = emu.albireo_folder();

        if let Some(albireo) = &cli.albireo {
            #[cfg(unix)]
            {
                let (backup_folder, emu_folder) = albireo_backup_and_original.as_ref().unwrap();

                std::os::unix::fs::symlink(
                    std::path::absolute(albireo).unwrap(),
                    std::path::absolute(emu_folder).unwrap()
                )
                .unwrap();
            }

            #[cfg(windows)]
            {
                let option = fs_extra::dir::CopyOptions::new()
                    .copy_inside(true)
                    .overwrite(true)
                    .skip_exist(false)
                    .content_only(true);
                fs_extra::dir::copy(albireo, &emu_folder, &option).unwrap();
            }
        }
    }

    // I had issues with symlinks on windows. no time to search why
    #[cfg(windows)]
    if let Some(albireo) = &cli.albireo {
        let option = fs_extra::dir::CopyOptions::new()
            .copy_inside(true)
            .overwrite(true)
            .skip_exist(false)
            .content_only(true);
        let emu_folder = emu.albireo_folder();
        if emu_folder.exists() {
            std::fs::remove_dir_all(&emu_folder).unwrap();
        }
        fs_extra::dir::copy(albireo, &emu_folder, &option).unwrap();
    }

    let t_emu = emu.clone();
    let emu_thread = std::thread::spawn(move || start_emulator(&t_emu, &conf).unwrap());

    if cli.albireo.is_some() {
        std::thread::sleep(Duration::from_secs(5));
    }
    else {
        std::thread::sleep(Duration::from_secs(3));
    }

    let window = get_emulator_window(&emu);
    let enigo = Enigo::new(&Settings::default()).unwrap();
    let mut robot = Robot::new(&emu, window, enigo);

    #[cfg(windows)]
    std::thread::sleep(Duration::from_millis(1000 * 3));

    let res = match cli.command {
        Commands::Orgams(OrgamsCli {
            src,
            dst,
            jump,
            edit,
            basm2orgams
        }) => {
            if basm2orgams {
                if let Some(albi) = &cli.albireo {
                    let src = Utf8Path::new(albi).join(src);
                    let dst = dst.as_ref().unwrap();
                    cpclib_asm::orgams::convert_from_to(src, dst).map_err(|e| e.to_string())
                }
                else {
                    unimplemented!()
                }
            }
            else if (jump || edit) && !cli.keepemulator {
                robot.close();
                Err("You must request to keep the emulator open with -k".to_string())
            }
            else {
                println!("!!! Current limitation: Ace must be configure as\n - Amstrad old\n - with a French keyboard\n - a French firmware\n - Unidos with nova and albireo\n - and must have enough memory. !!! No idea yet how to overcome that without modifying ace");
                robot.handle_orgams(
                    cli.drive_a.as_deref(),
                    cli.albireo.as_deref(),
                    &src,
                    dst.as_deref(),
                    jump,
                    edit
                )
            }
        },

        Commands::Run { text } => {
            cli.keepemulator = true;

            if let Some(text) = text {
                robot.handle_raw_text(text);
            }

            Ok(())
        }
    };

    if !cli.keepemulator {
        robot.close();
    }

    if let Some((backup_folder, emu_folder)) = albireo_backup_and_original {
        if cli.keepemulator {
            eprintln!(
                "Albireo folder not cleaned automatically. you'll have to do it if necessary"
            );
        }
        else {
            #[cfg(windows)]
            {
                // need to copy back modifications
                let option = fs_extra::dir::CopyOptions::new()
                    .copy_inside(true)
                    .overwrite(true)
                    .skip_exist(false)
                    .content_only(true);
                let albireo = cli.albireo.as_ref().unwrap();
                std::fs::remove_dir_all(albireo).unwrap();
                fs_extra::dir::copy(&emu_folder, albireo, &option).unwrap();

                // restore previous
                if backup_folder.exists() {
                    std::fs::rename(&backup_folder, &emu_folder).unwrap();
                }
            }
        }
    }

    res
}
