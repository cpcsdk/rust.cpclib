use cpclib_bndbuild::BndBuilder;
use cpclib_bndbuild::event::{BndBuilderObserved, BndBuilderObserverRc};

const YAML_RULES: &str = "
- targets: version
  dependencies: basm img2cpc

- targets: basm
  commands: basm  --version

- targets: img2cpc
  commands: img2cpc --version

- targets: echo
  commands: echo --version

- targets: rm
  commands: -rm --version
";

fn main() {
    let observer = BndBuilderObserverRc::new_default();

    let mut builder =
        BndBuilder::from_string(YAML_RULES.into(), None, false).expect("Unable to read yaml");
    builder.add_observer(observer);

    println!("Show version of commands");
    let r = builder.execute("version");

    match r {
        Ok(_o) => {},
        Err(e) => {
            println!("{}", e);
            panic!();
        }
    }
}
