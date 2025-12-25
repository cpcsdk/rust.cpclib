use std::error::Error;

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use cpclib_common::itertools::Itertools;
use ini::Ini;

use crate::emucontrol::Crtc;

const SANE_CONFIGURATION: &str = "
SCREEN=0
CRTFILTER=0
OVERSCAN=0
VIDEOEXTRA=0
COVER=0
CRTC=0
DOUBLESIZE=0
FASTWIN=0
EXTRASOUND=0
";

pub struct AceConfig {
    path: Utf8PathBuf,
    ini: Ini
}

#[derive(Copy, Clone, Debug)]
pub enum AceConfigFlag {
    PluginNova,
    PluginAlbireo1,
    PluginAlbireo2
}

impl AsRef<str> for AceConfigFlag {
    fn as_ref(&self) -> &str {
        match self {
            AceConfigFlag::PluginNova => "PLUGIN_NOVA",
            AceConfigFlag::PluginAlbireo1 => "PLUGIN_ALBIREO1",
            AceConfigFlag::PluginAlbireo2 => "PLUGIN_ALBIREO2"
        }
    }
}

impl AceConfig {
    // On ly machine default file is in /home/romain/.config/ACE-DL_futuristics/config.cfg
    pub fn open<P: AsRef<Utf8Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let p = path.as_ref();
        let ini = Ini::load_from_file(p)?;
        Ok(AceConfig {
            ini,
            path: p.to_owned()
        })
    }

    pub fn open_or_default<P: AsRef<Utf8Path>>(path: P) -> Self {
        Self::open(&path).unwrap_or_else(|_e| {
            // XXX is it ok to ignore the error ?
            let ini = Self::default_ini();
            let path = path.as_ref().to_owned();
            AceConfig { ini, path }
        })
    }

    fn default_ini() -> Ini {
        Ini::load_from_str(SANE_CONFIGURATION).unwrap()
    }

    pub fn save_as<P: AsRef<Utf8Path>>(&self, path: P) -> std::io::Result<()> {
        self.ini.write_to_file(path.as_ref())
    }

    pub fn save(&self) -> std::io::Result<()> {
        let folder = self.path.parent().unwrap();
        if !folder.exists() {
            std::fs::create_dir_all(folder)?;
        }
        self.save_as(&self.path)
    }

    pub fn folder(&self) -> &Utf8Path {
        &self.path
    }

    pub fn sanitize(&mut self) -> &mut Self {
        let default = SANE_CONFIGURATION;
        for (key, value) in default
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .map(|l| l.split("=").collect_tuple().unwrap())
        {
            self.set(key, value);
        }

        self
    }

    pub fn remove<Key: AsRef<str> + ?Sized>(&mut self, key: &Key) -> &mut Self {
        self.ini.delete_from::<String>(None, key.as_ref());
        self
    }

    pub fn set<Key: ToString, Value: ToString>(&mut self, key: Key, value: Value) -> &mut Self {
        self.ini
            .set_to::<String>(None, key.to_string(), value.to_string());
        self
    }

    pub fn set_bool<Key: ToString>(&mut self, key: Key, value: bool) -> &mut Self {
        self.set(key, if value { "1" } else { "0" })
    }

    pub fn enable(&mut self, flag: AceConfigFlag) -> &mut Self {
        let key = flag.as_ref();
        self.set(key, "1");
        self
    }

    pub fn disable(&mut self, flag: AceConfigFlag) -> &mut Self {
        let key = flag.as_ref();
        self.set(key, "0");
        self
    }

    pub fn remove_cartridge(&mut self) -> &Self {
        self.remove("CARTRIDGE")
    }

    pub fn select_crtc(&mut self, crtc: Crtc) -> &Self {
        self.set("CRTC", crtc)
    }
}
