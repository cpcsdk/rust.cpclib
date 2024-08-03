use cpclib_bndbuild::rules::Rules;
use serde_yaml;

fn main() {
    // Hardcoded executable for samourai

    std::env::set_current_dir(r"C:\Users\giotr\perso\CPC\welcome-to-the-garbage\src\samourai")
        .unwrap();

    let yaml_rules = "
# Builder configuration for the samourai project

- targets: samourai.sna samourai.sym
  dependencies: samourai.asm
  commands: basm samourai.asm --progress --snapshot -o samourai.sna -Idata --sym samourai.sym

- targets: distclean
  dependencies: clean
  commands:
   - echo Delete uneeded files
   - -rm samourai*.sna

- targets: clean
  commands: -rm samourai.{lst,sym}

- targets: samourai_linked.sna linking.lst
  dependencies: samourai.sna samourai.sym
  commands: basm linking.asm --progress  --snapshot  -l samourai.sym  -o samourai_linked.sna  --lst linking.lst

- targets: data/font_000.bin
  dependencies: data/font.png
  commands: imgconverter data/font.png --mode 2  --pen0 26 --pen1 0  tile -w 1 -h 8  -o data/font.bin

";

    let rules: Rules = serde_yaml::from_str(yaml_rules).expect("Unable to read yaml");

    let deps = rules.to_deps().unwrap();
    println!("Try to show dependencies");
    deps.show_dependencies("samourai.sna");
    deps.execute("samourai.sna").unwrap();

    let deps = rules.to_deps().unwrap();
    deps.execute("distclean").unwrap();
}
