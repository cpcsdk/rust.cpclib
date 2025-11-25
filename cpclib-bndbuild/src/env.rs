use camino::Utf8Path;
use cpclib_runner::runner::tracker::at3::At3Version;
use minijinja::{Environment, Error, ErrorKind};

pub fn create_template_env<P: AsRef<Utf8Path>, S1: AsRef<str>, S2: AsRef<str>>(
        _working_directory: Option<P>,
        definitions: &[(S1, S2)]) -> Environment<'_> {

    let mut env = Environment::new();
    fn error(error: String) -> Result<String, Error> {
        Err(Error::new(ErrorKind::InvalidOperation, error))
    }
    #[cfg(target_os = "linux")]
    fn basm_escape_path(path: String) -> Result<String, Error> {
        Ok(path)
    } 

    #[cfg(target_os = "windows")]
    fn basm_escape_path(path: String) -> Result<String, Error> {
        Ok(path.replace("\\", "\\\\\\\\"))
    }

    pub fn path_loader<'x, P: AsRef<std::path::Path> + 'x>(
        dir: P
    ) -> impl for<'a> Fn(&'a str) -> Result<Option<String>, Error> + Send + Sync + 'static
    {
        let dir = dir.as_ref().to_path_buf();
        move |name| {
            let path = dir.join(name); // TODO add a safety ??
            match std::fs::read_to_string(path) {
                Ok(result) => Ok(Some(result)),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(err) => {
                    Err(
                        Error::new(ErrorKind::InvalidOperation, "could not read template")
                            .with_source(err)
                    )
                },
            }
        }
    }

    env.set_loader(path_loader(std::env::current_dir().unwrap()));
    env.add_function("fail", error);
    env.add_function("basm_escape_path", basm_escape_path);
    env.add_filter("basm_escape_path", basm_escape_path);

    // Automatic feeding of FAP related environement variables
    #[cfg(feature = "fap")]
    {
        use cpclib_runner::runner::ay::fap::FAPVersion;

        let fap = FAPVersion::default(); // TODO allow to handle various versions
        env.add_global("FAP_PLAY_PATH", fap.fap_play_path::<()>().as_str());
        env.add_global("FAP_INIT_PATH", fap.fap_init_path::<()>().as_str());
    }

    // Feed AT3 variables
    {
        let at = At3Version::default();
        env.add_global("AKG_PLAYER_PATH", at.akg_path::<()>().as_str());
        env.add_global("AKM_PLAYER_PATH", at.akm_path::<()>().as_str());
        env.add_global("AKY_PLAYER_PATH", at.aky_path::<()>().as_str());
        env.add_global(
            "AKY_STABLE_PLAYER_PATH",
            at.aky_stable_path::<()>().as_str()
        );
    }

    // Feed user related variables
    for (key, value) in definitions {
        let key = key.as_ref();
        let value = value.as_ref();
        env.add_global(key, value);
    }

    env

}