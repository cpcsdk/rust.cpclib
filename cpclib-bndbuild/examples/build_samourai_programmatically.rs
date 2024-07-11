use cpclib_bndbuild::rules::{Rule, Rules};
use cpclib_bndbuild::task::Task;

fn main() {
    // Hardcoded executable for samourai

    std::env::set_current_dir(r"C:\Users\giotr\perso\CPC\welcome-to-the-garbage\src\samourai")
        .unwrap();

    let rules = Rules::new(vec![ Rule::new(
        &["samourai.sna", "samourai.sym"],
        &["samourai.asm"],
        &[Task::new_basm("samourai.asm --progress --snapshot -o samourai.sna -Idata --sym samourai.sym")]),
        

    Rule::new(
        &["distclean"] as &[&str],
        &["clean"] as &[&str],
        &[Task::new_echo("Delete uneeded files"), Task::new_rm("samourai*.sna").set_ignore_errors(true)]
    ),

    Rule::new(
        &["clean"],
        &[],
        &[Task::new_rm("samourai.{lst,sym}").set_ignore_errors(true)]
    ),

    Rule::new(
        &["samourai_linked.sna", "linking.lst"],
        &["samourai.sna", "samourai.sym"],
        &[Task::new_basm("linking.asm --progress  --snapshot  -l samourai.sym  -o samourai_linked.sna  --lst linking.lst")]
    ),

    Rule::new(
        &["data/font_000.bin"],
        &["data/font.png"],
        &[Task::new_imgconverter( "data/font.png --mode 2  --pen0 26 --pen1 0  tile -w 1 -h 8  -o data/font.bin")]
    )


    ]);

    let deps = rules.to_deps().unwrap();
    println!("Try to show dependencies");
    deps.show_dependencies(&"samourai.sna".into());
    deps.execute("samourai.sna").unwrap();

    let deps = rules.to_deps().unwrap();
    deps.execute("distclean").unwrap();
}
