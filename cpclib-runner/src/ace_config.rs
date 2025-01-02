use std::error::Error;

use cpclib_common::camino::{Utf8Path, Utf8PathBuf};
use ini::Ini;

pub const SANE_CONFIGURATION: &str = "
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
    PluginAlbireo2,
}

impl AsRef<str> for AceConfigFlag {
    fn as_ref(&self) -> &str {
        match self {
            AceConfigFlag::PluginNova => "PLUGIN_NOVA",
            AceConfigFlag::PluginAlbireo1 => "PLUGIN_ALBIREO1",
            AceConfigFlag::PluginAlbireo2 => "PLUGIN_ALBIREO2",
        }
    }
}

impl AceConfig {
    pub fn open<P: AsRef<Utf8Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let p = path.as_ref();
        let ini = Ini::load_from_file(p)?;
        Ok(AceConfig {
            ini,
            path: p.to_owned()
        })
    }

    pub fn open_or_default<P: AsRef<Utf8Path>>(path: P) -> Self {
        Self::open(&path)
            .unwrap_or_else(|_e|{ // XXX is it ok to ignore the error ?
                let ini = Self::default_ini();
                let path = path.as_ref().to_owned();
                AceConfig{ini, path}
            })
    }

    fn default_ini() -> Ini {
        Ini::load_from_str(SANE_CONFIGURATION).unwrap()
    }

    pub fn save_as<P: AsRef<Utf8Path>>(&self, path: P) -> std::io::Result<()> {
        self.ini.write_to_file(path.as_ref())
    }

    pub fn save(&self) -> std::io::Result<()> {
        self.save_as(&self.path)
    }

    pub fn folder(&self) -> &Utf8Path {
        &self.path
    }

    pub fn remove<Key: AsRef<str>>(&mut self, key: &Key) {
        self.ini.delete_from::<String>(None, key.as_ref());
    }

    pub fn set<Key: ToString, Value: ToString>(&mut self, key: Key, value: Value) {
        self.ini
            .set_to::<String>(None, key.to_string(), value.to_string())
    }


    pub fn enable(&mut self, flag: AceConfigFlag) {
        let key = flag.as_ref();
        self.set(key, "1");
    }

    pub fn disable(&mut self, flag: AceConfigFlag) {
        let key = flag.as_ref();
        self.set(key, "0");
    }


}
