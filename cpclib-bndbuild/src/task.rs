use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Task {
    Basm(StandardTask),
    Rm(StandardTask),
    Echo(StandardTask),
    ImgConverter(StandardTask),
    Xfer(StandardTask),
    Dsk(StandardTask),
    Extern(StandardTask)
}

impl<'de> Deserialize<'de> for Task {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        struct Line;
        impl<'de> Visitor<'de> for Line {
            type Value = Task;

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where E: serde::de::Error {
                let (code, next) = v.split_once(" ").ok_or(Error::custom("Wrong format"))?;
                let (code, ignore) = if code.starts_with("-") {
                    (&code[1..], true)
                }
                else {
                    (code, false)
                };
                let std = StandardTask {
                    args: next.to_owned(),
                    ignore_error: ignore
                };

                match code {
                    "basm" | "assemble" => Ok(Task::Basm(std)),
                    "echo" | "print" => Ok(Task::Echo(std)),
                    "rm" | "del" => Ok(Task::Rm(std)),
                    "img2cpc" | "imgconverter" => Ok(Task::ImgConverter(std)),
                    "xfer" | "cpcwifi" | "m4" => Ok(Task::Xfer(std)),
                    "extern" => Ok(Task::Extern(std)),
                    "dsk" | "disc" => Ok(Task::Dsk(std)),
                    _ => Err(Error::custom(format!("{code} is invalid")))
                }
            }

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("Expecting a command")
            }
        }

        deserializer.deserialize_str(Line)
    }
}

impl Task {
    pub fn new_dsk(args: &str) -> Self {
        Self::Dsk(StandardTask::new(args))
    }
    pub fn new_basm(args: &str) -> Self {
        Self::Basm(StandardTask::new(args))
    }

    pub fn new_rm(args: &str) -> Self {
        Self::Rm(StandardTask::new(args))
    }

    pub fn new_echo(args: &str) -> Self {
        Self::Echo(StandardTask::new(args))
    }

    pub fn new_imgconverter(args: &str) -> Self {
        Self::ImgConverter(StandardTask::new(args))
    }

    pub fn args(&self) -> &str {
        match self {
            Task::Basm(t)
            | Task::Rm(t)
            | Task::Echo(t)
            | Task::ImgConverter(t)
            | Task::Xfer(t)
            | Task::Extern(t)
            | Task::Dsk(t) => &t.args
        }
    }

    pub fn ignore_errors(&self) -> bool {
        match self {
            Task::Basm(t)
            | Task::Rm(t)
            | Task::Echo(t)
            | Task::ImgConverter(t)
            | Task::Xfer(t)
            | Task::Extern(t) 
            | Task::Dsk(t) => t.ignore_error
        }
    }

    pub fn set_ignore_errors(mut self, ignore: bool) -> Self {
        match self {
            Task::Basm(ref mut t)
            | Task::Rm(ref mut t)
            | Task::Echo(ref mut t)
            | Task::Xfer(ref mut t)
            | Task::ImgConverter(ref mut t)
            | Task::Extern(ref mut t)
            | Task::Dsk(ref mut t)  => t.ignore_error = ignore
        }

        self
    }

    // TODO deeply check the arguments of the commands because here we may be wrong ...
    pub fn is_phony(&self) -> bool {
        match self {
            Task::Basm(_) => false, // wrong when displaying stuff
            Task::Rm(_) => false,
            Task::Echo(_) => true,
            Task::Xfer(_) => true, // wrong when downloading files
            Task::ImgConverter(_) => false,
            Task::Extern(_) => false,
            Task::Dsk(_) => false, // wrong for winape
        }
    }
}

#[derive(Deserialize, Clone, PartialEq, Debug, Eq, Hash)]
pub struct StandardTask {
    args: String,
    ignore_error: bool
}

impl StandardTask {
    pub fn new(args: &str) -> Self {
        Self {
            args: args.to_string(),
            ignore_error: false
        }
    }
}

#[cfg(test)]
mod test {
    use super::Task;
    use crate::task::StandardTask;

    #[test]
    fn test_deserialize_task() {
        let yaml = "basm toto.asm -o toto.o";
        let task: Task = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            task,
            Task::Basm(StandardTask {
                args: "toto.asm -o toto.o".to_owned(),
                ignore_error: false
            })
        );

        let yaml = "-basm toto.asm -o toto.o";
        let task: Task = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            task,
            Task::Basm(StandardTask {
                args: "toto.asm -o toto.o".to_owned(),
                ignore_error: true
            })
        );
    }
}
