use cpclib_bndbuild::rules::Rules;

fn main() {
    // Hardcoded executable for samourai

    let yaml_rules = "
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

    let rules: Rules = serde_yaml::from_str(yaml_rules).expect("Unable to read yaml");

    let deps = rules.to_deps().unwrap();
    println!("Show version of commands");
    let r = deps.execute("version");

    match r {
        Ok(_o) => {},
        Err(e) => {
            println!("{}", e);
            panic!();
        }
    }
}
