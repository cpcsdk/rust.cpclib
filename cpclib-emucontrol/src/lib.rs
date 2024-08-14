use std::collections::HashSet;
use std::marker::PhantomData;
use std::time::Duration;

use bon::builder;
use cpclib_bndbuild::delegated::DelegatedRunner;
use cpclib_bndbuild::runners::emulator::Emulator;
use cpclib_bndbuild::runners::Runner;
use cpclib_common::itertools::Itertools;
use enigo::{Direction, Enigo, Key, Keyboard};
use xcap::image::{open, GenericImageView, ImageBuffer, Rgba};
use xcap::Window;

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
                Emulator::Cpcec(_) => todo!(),
                Emulator::Winape(_) => todo!()
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
    fn screenshot(robot: &mut RobotImpl<Self>) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
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

    pub fn handle_orgams(
        &mut self,
        src: &str,
        dst: Option<&str>,
        jump: bool
    ) -> Result<(), String> {
        match self {
            Robot::Ace(r) => r.handle_orgams(src, dst, jump),
            Robot::Cpcec(r) => r.handle_orgams(src, dst, jump),
            Robot::Winape(r) => r.handle_orgams(src, dst, jump)
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
            self.click_char(c);
        }
    }

    fn click_keys(&mut self, keys: &[Key]) {
        for key in keys {
            self.click_key(*key);
        }
    }

    fn click_char(&mut self, c: char) {
        self.click_key(Key::Unicode(c))
    }

        #[cfg(target_os="linux")]
        fn click_key(&mut self, key: Key) {




        self.enigo.key(key, enigo::Direction::Press).unwrap();
        Self::wait_a_bit();
        self.enigo.key(key, enigo::Direction::Release).unwrap();
        Self::wait_a_bit();
    }


        #[cfg(windows)]
        fn click_key(&mut self, key: Key) {
        dbg!(&key);

        #[cfg(windows)]
        let key = match key {
            // https://boostrobotics.eu/windows-key-codes/
            Key::Unicode(v) if v >= '0' && v <= '9' => {
                let nb = (v as u32 - '0' as u32);

                self.enigo.key(Key::LShift, enigo::Direction::Press).unwrap();
                Self::wait_a_bit();
                Self::wait_a_bit();

                let lut = ['à', '&', 'é', '"', '\'', '(', '-', 'è', '_', 'ç'][nb as usize];
                dbg!(nb, lut);
                let key = Key::Unicode(lut);

                self.enigo.key(key, enigo::Direction::Press).unwrap();
                Self::wait_a_bit();
                Self::wait_a_bit();
                self.enigo.key(key, enigo::Direction::Release).unwrap();
                Self::wait_a_bit();
                Self::wait_a_bit();

                self.enigo.key(Key::LShift, enigo::Direction::Release).unwrap();
                Self::wait_a_bit();
            }
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
    pub fn handle_orgams(
        &mut self,
        src: &str,
        dst: Option<&str>,
        jump: bool
    ) -> Result<(), String> {
        // we assume that we do not need to select the window as well launched it. it is already selected

        let res = self.orgams_assemble(src);
        match res {
            Ok(_) => {
                if jump {
                    self.orgams_jump()
                }
                else {
                    self.orgams_save(dst)
                }
            },
            Err(screen) => {
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
                Err(format!("An error occured. Look at {}", path.to_string()))
            }
        }
    }

    fn orgams_jump(&mut self) -> Result<(), String> {
        self.click_char('j');
        Ok(())
    }

    fn orgams_save(&mut self, dst: Option<&str>) -> Result<(), String> {
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
                println!("  error.");
                return Err("Error while saving {dst}".to_owned());
            }
        }
    }

    fn orgams_assemble(&mut self, src: &str) -> Result<(), ImageBuffer<Rgba<u8>, Vec<u8>>> {
        // Open orgams
        println!("> Launch orgams and open file {src} from drive a");
        // French setup ?
        #[cfg(target_os="linux")]
        let mut keys = vec![Key::Unicode('ù')];
        #[cfg(windows)]
        let mut keys = vec![Key::Unicode('ù') /*Key::Other(165)*/];
        keys.extend_from_slice(&[Key::Unicode('o'), Key::Unicode(','), Key::Unicode('"')]);
        for c in src.chars() {
            keys.push(Key::Unicode(c));
        }
        keys.extend_from_slice(&[Key::Unicode('"'), Key::Return]);
        self.click_keys(&keys);
        self.wait_orgams_loading();
        println!("  done.");

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

    fn wait_orgams_loading(&mut self) {
        #[derive(PartialEq)]
        enum State {
            Basic,
            Loading,
            Loaded
        };

        // we check a specific pixel that goes from blue to black then purple
        let coord_of_interest = (8, 48); // 2 to work on Amstrad plus and old
        let mut state = State::Basic;
        while state != State::Loaded {
            let screen = E::screenshot(self);
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

            std::thread::sleep(Duration::from_millis(1000 / 10));
        }
    }
}
