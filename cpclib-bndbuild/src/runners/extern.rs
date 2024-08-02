use cpclib_common::itertools::Itertools;

use super::Runner;
use crate::task::EXTERN_CMDS;

#[derive(Default)]
pub struct ExternRunner {}
impl ExternRunner {}
impl Runner for ExternRunner {
    fn inner_run<S: AsRef<str>>(&self, itr: &[S]) -> Result<(), String> {
        let itr = itr.iter().map(|s| s.as_ref()).collect_vec();

        // WARNING
        // Deactivated because if makes fail normal progam on Linux
        // however, it was maybe mandatory for Windows
        // let app = std::fs::canonicalize(&itr[0])
        //     .map_err(|e| format!("Wrong executable {}.{}", &itr[0], e.to_string()))?;
        let app = &itr[0];

        let cwd = std::env::current_dir().map_err(|e| {
            format!(
                "Unable to get the current working directory {}.",
                e.to_string()
            )
        })?;
        let cwd = std::fs::canonicalize(cwd).map_err(|e| {
            format!(
                "Unable to get the current working directory {}.",
                e.to_string()
            )
        })?;

        let mut cmd = std::process::Command::new(app);
        cmd.current_dir(cwd);
        for arg in &itr[1..] {
            cmd.arg(dbg!(arg));
        }
        let mut handle = cmd
            .spawn()
            .map_err(|e| format!("Error while launching {}. {}", &itr[0], e.to_string()))?;

        let status = handle
            .wait()
            .map_err(|e| format!("Error while executing {}. {}", &itr[0], e.to_string()))?;

        if !status.success() {
            return Err("Error while launching the command.".to_owned());
        }
        Ok(())
    }

    fn get_command(&self) -> &str {
        &EXTERN_CMDS[0]
    }
}
