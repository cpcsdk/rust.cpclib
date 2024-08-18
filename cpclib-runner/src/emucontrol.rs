use std::collections::HashSet;
use std::marker::PhantomData;
use std::path::absolute;
use std::time::Duration;

use bon::builder;
use clap::{ArgAction, Command, CommandFactory, Parser, Subcommand, ValueEnum};
use cpclib_common::itertools::Itertools;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use xcap::image::{open, ImageBuffer, Rgba};
use xcap::Window;

#[cfg(windows)]
use fs_extra;

use crate::delegated::{clear_base_cache_folder, DelegatedRunner};
use crate::runner::emulator::Emulator;
use crate::runner::runner::RunnerWithClap;
use crate::runner::Runner;

type Screenshot = ImageBuffer<Rgba<u8>, Vec<u8>>;

pub enum AmstradRom {
    Orgams
}

#[builder]
pub struct EmulatorConf {
    pub(crate) drive_a: Option<String>,
    pub(crate) drive_b: Option<String>,
    #[builder(default)]
    pub(crate) roms_configuration: HashSet<AmstradRom>
}

impl EmulatorConf {
    /// Generate the args for the corresponding emulator
    pub fn args_for_emu(&self, emu: &Emulator) -> Vec<String> {
        let mut args = Vec::default();

        if let Some(drive_a) = &self.drive_a {
            match emu {
                Emulator::Ace(_) => args.push(drive_a.to_string()),
                Emulator::Cpcec(_) => args.push(drive_a.to_string()),
                Emulator::Winape(_) => {
                    args.push(absolute(drive_a.to_string()).unwrap().display().to_string())
                },
            }
        }

        if let Some(drive_b) = &self.drive_b {
            match emu {
                Emulator::Ace(_) => todo!(),
                Emulator::Cpcec(_) => todo!(),
                Emulator::Winape(_) => todo!()
            }
        }

        if !self.roms_configuration.is_empty() {
            match emu {
                Emulator::Ace(_) => todo!(),
                Emulator::Cpcec(_) => todo!(),
                Emulator::Winape(_) => todo!()
            }
        }
        args
    }
}

pub fn start_emulator(emu: &Emulator, conf: &EmulatorConf) -> Result<(), String> {
    let args = conf.args_for_emu(emu);
    let app = emu.configuration();

    let runner = DelegatedRunner::new(app, emu.get_command().into());
    runner.inner_run(&args)
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

trait UsedEmulator {
    fn screenshot(robot: &mut RobotImpl<Self>) -> ImageBuffer<Rgba<u8>, Vec<u8>>
    where Self: Sized;
}

struct AceUsedEmulator {}
struct CpcecUsedEmulator {}
struct WinapeUsedEmulator {}

impl UsedEmulator for AceUsedEmulator {
    // here we delegate the creation of screenshot to Ace to avoid some issues i do not understand
    fn screenshot(robot: &mut RobotImpl<Self>) -> Screenshot {
        let folder = robot.emu.screenshots_folder();
        let before_screenshots: HashSet<_> = glob::glob(folder.join("*.png").as_str())
            .unwrap()
            .into_iter()
            .map(|p| p.unwrap().as_path().to_owned())
            .collect();

        // handlekey press
        robot.click_key(Key::F10);

        let mut file = None;
        while file.is_none() {
            RobotImpl::<AceUsedEmulator>::wait_a_bit();
            let after_screenshots: HashSet<_> = glob::glob(folder.join("*.png").as_str())
                .unwrap()
                .into_iter()
                .map(|p| p.unwrap().as_path().to_owned())
                .collect();
            let mut new_screenshots = after_screenshots
                .difference(&before_screenshots)
                .cloned()
                .collect_vec();
            if new_screenshots.len() > 0 {
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
impl UsedEmulator for CpcecUsedEmulator {
    fn screenshot(robot: &mut RobotImpl<Self>) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        robot.window.capture_image().unwrap()
    }
}
impl UsedEmulator for WinapeUsedEmulator {
    fn screenshot(robot: &mut RobotImpl<Self>) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        robot.window.capture_image().unwrap()
    }
}

struct RobotImpl<E: UsedEmulator> {
    pub(crate) window: Window,
    pub(crate) enigo: Enigo,
    pub(crate) emu: Emulator,
    _emu: PhantomData<E>
}

pub enum Robot {
    Ace(RobotImpl<AceUsedEmulator>),
    Cpcec(RobotImpl<CpcecUsedEmulator>),
    Winape(RobotImpl<WinapeUsedEmulator>)
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
    pub fn new(emu: &Emulator, window: Window, enigo: Enigo) -> Self {
        match emu {
            Emulator::Ace(_) => RobotImpl::<AceUsedEmulator>::from((window, enigo, emu)).into(),
            Emulator::Cpcec(_) => RobotImpl::<CpcecUsedEmulator>::from((window, enigo, emu)).into(),
            Emulator::Winape(_) => {
                RobotImpl::<WinapeUsedEmulator>::from((window, enigo, emu)).into()
            },
        }
    }

    pub fn handle_raw_text<S: AsRef<str>>(&mut self, text: S) {
        let text = text.as_ref();
        let text = text.replace(r"\n", "\n");
        let text = &text;

        match self {
            Robot::Ace(r) => r.type_text(text),
            Robot::Cpcec(r) => r.type_text(text),
            Robot::Winape(r) => r.type_text(text)
        }
    }

    pub fn handle_orgams(
        &mut self,
        drivea: Option<&str>,
        albireo: Option<&str>,
        src: &str,
        dst: Option<&str>,
        jump: bool
    ) -> Result<(), String> {
        match self {
            Robot::Ace(r) => r.handle_orgams(drivea, albireo, src, dst, jump),
            Robot::Cpcec(r) => r.handle_orgams(drivea, albireo, src, dst, jump),
            Robot::Winape(r) => r.handle_orgams(drivea, albireo, src, dst, jump)
        }
    }

    pub fn close(&mut self) {
        match self {
            Robot::Ace(r) => r.close(),
            Robot::Cpcec(r) => r.close(),
            Robot::Winape(r) => r.close()
        }
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
        let key = match key {
            // https://boostrobotics.eu/windows-key-codes/
            Key::Unicode(v) if v >= '0' && v <= '9' => {
                if false {
                    let nb = (v as u32 - '0' as u32);

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
        jump: bool
    ) -> Result<(), String> {
        // we assume that we do not need to select the window as well launched it. it is already selected

        self.unidos_select_drive(drivea, albireo);

        self.orgams_load(src)
            .map_err(|screen| ("Error while loading".to_string(), screen))
            .and_then(|_| {
                self.orgams_assemble(src)
                    .map_err(|screen| ("Error while assembling".to_string(), screen))
            })
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
            .map_err(|(msg, screen)| {
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
                format!("An error occurred.\n{msg}\nLook at {}.", path.to_string())
            })
    }

    fn orgams_jump(&mut self) -> Result<(), Screenshot> {
        self.click_char('j');
        Ok(())
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
        self.enigo
            .key(Key::Control, enigo::Direction::Press)
            .unwrap();
        Self::wait_a_bit();
        self.enigo
            .key(Key::Unicode('1'), enigo::Direction::Press)
            .unwrap();
        Self::wait_a_bit();
        self.enigo
            .key(Key::Unicode('1'), enigo::Direction::Release)
            .unwrap();
        Self::wait_a_bit();
        self.enigo
            .key(Key::Control, enigo::Direction::Release)
            .unwrap();

        self.wait_orgams_assembling();
        println!("  done.");

        let result: ImageBuffer<Rgba<u8>, Vec<u8>> = self.window.capture_image().unwrap();

        if result
            .pixels()
            .into_iter()
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
pub struct Cli {
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

    #[arg(short, long, default_value = "ace", alias = "emu")]
    emulator: Emu,

    #[arg(short, long, action = ArgAction::SetTrue, help = "Keep the emulator open after the interaction")]
    keepemulator: bool,

    #[arg(short, long, action = ArgAction::SetTrue, help = "Clear the cache folder")]
    clear_cache: bool,

    #[command(subcommand)]
    command: Commands
}

#[derive(ValueEnum, Clone, Debug)]
pub enum Emu {
    Ace,
    Winape,
    Cpcec
}

#[derive(Subcommand, Clone, Debug)]
pub enum Commands {
    Orgams {
        /// lists test values
        #[arg(short, long, help = "Filename to assemble")]
        src: String,

        #[arg(
            short,
            long,
            help = "Filename to save. By default use the one provided by orgams"
        )]
        dst: Option<String>,

        #[arg(short, long, action = ArgAction::SetTrue, conflicts_with="dst", help="Jump on the program instead of saving it")]
        jump: bool
    },

    Run {
        #[arg(short, long, help = "Simple text to type")]
        text: Option<String>
    }
}


pub const EMUCTRL_CMD: &str = "cpc";

pub struct EmuControlledRunner {
    command: Command
}

impl Default for EmuControlledRunner {
    fn default() -> Self {
        Self { command: Cli::command() }
    }
}

impl Runner for EmuControlledRunner {
    fn inner_run<S: AsRef<str>>(&self, itr: &[S]) -> Result<(), String> {
        let mut itr = itr.iter().map(|s| s.as_ref()).collect_vec();
        itr.insert(0, "cpc");
        let cli = Cli::parse_from(itr);

        handle_arguments(cli)
    }

    fn get_command(&self) -> &str {
        EMUCTRL_CMD
    }
}


impl RunnerWithClap for EmuControlledRunner {
    fn get_clap_command(&self) -> &clap::Command {
        &self.command
    }
}


pub fn handle_arguments(mut cli: Cli) -> Result<(), String> {

    if cli.clear_cache {
        clear_base_cache_folder()
            .map_err(|e| format!("Unable to clear the cache folder. {}", e.to_string()))?;
    }


    let builder = EmulatorConf::builder()
        .maybe_drive_a(cli.drive_a.clone())
        .maybe_drive_b(cli.drive_b.clone());
    let conf = builder.build();

    let emu = match cli.emulator {
        Emu::Ace => Emulator::Ace(Default::default()),
        Emu::Winape => Emulator::Winape(Default::default()),
        Emu::Cpcec => Emulator::Cpcec(Default::default())
    };


    { // ensure emulator is isntalled
        let conf = emu.configuration();
        if ! conf.is_cached() {
            conf.install()?;
        }
    }

    // setup emulator
    // TODO do it conditionally
    // copy the non standard roms
    let needed_roms = [
        "unidos.rom",
        "unitools.rom",
        "albireo.rom",
        "nova.rom",
        "parados12.fixedanyslot.fixedname.quiet.rom"
    ];
    for rom in needed_roms {
        let dst = emu.roms_folder().join(rom);

        if !dst.exists() {
            let src = std::path::Path::new("roms").join(rom);
            std::fs::copy(src, dst).unwrap();
        }
    }

    #[cfg(unix)]
    let albireo_backup_and_original = if let Some(albireo) = &cli.albireo {
        let emu_folder = emu.albireo_folder();
        let backup_folder = emu_folder
            .parent()
            .unwrap()
            .join(emu_folder.file_name().unwrap().to_owned() + ".bak");

        if backup_folder.exists() {
            std::fs::remove_dir_all(&backup_folder).unwrap();
        }

        std::fs::rename(&emu_folder, &backup_folder).unwrap();

        std::os::unix::fs::symlink(
            std::path::absolute(&albireo).unwrap(),
            std::path::absolute(&emu_folder).unwrap()
        )
        .unwrap();

        Some((backup_folder, emu_folder))
    }
    else {
        None
    };

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
        std::thread::sleep(Duration::from_secs(8));
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
        Commands::Orgams { src, dst, jump } => {
            if jump && !cli.keepemulator {
                robot.close();
                Err("You must request to keep the emulator open with -k".to_string())
            } else {

                println!("!!! Current limitation: Ace must be configure as\n - Amstrad old\n - with a French keyboard\n - a French firmware\n - Unidos with nova and albireo\n - and must have enough memory. !!! No idea yet how to overcome that without modifying ace");

                robot.handle_orgams(
                    cli.drive_a.as_ref().map(|s| s.as_str()),
                    cli.albireo.as_ref().map(|s| s.as_str()),
                    &src,
                    dst.as_ref().map(|s| s.as_str()),
                    jump
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

    #[cfg(unix)]
    if let Some((backup_folder, emu_folder)) = albireo_backup_and_original {
        if cli.keepemulator {
            eprintln!("Albireo folder not cleaned");
        } else {
            std::fs::rename(&backup_folder, &emu_folder).unwrap();
        }
    }

    #[cfg(windows)]
    if let Some(albireo) = &cli.albireo {
        if cli.keepemulator {
            eprintln!("Albireo folder not cleaned");
        } else {
        let option = fs_extra::dir::CopyOptions::new()
            .copy_inside(true)
            .overwrite(true)
            .skip_exist(false)
            .content_only(true);
        let emu_folder = emu.albireo_folder();
        std::fs::remove_dir_all(&albireo).unwrap();
        fs_extra::dir::copy(&emu_folder, albireo, &option).unwrap();
        }
    }

    res
}
