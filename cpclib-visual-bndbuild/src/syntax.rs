use std::collections::HashSet;

use egui_code_editor::Syntax;

pub fn syntax() -> Syntax {
    Syntax {
        language: "bndbuild",
        case_sensitive: true,
        comment: "#",
        comment_multiline: [r"/*", r"*/"],
        keywords: HashSet::from([
            "tgt",
            "target",
    //        "build",
            "dep",
            "dependency",
            "requires",
            "cmd",
            "command",
            "launch",
            "run"
        ]),
        types: HashSet::from([
            "basm",
            "assemble",
            "echo",
            "print",
            "rm",
            "del",
            "img2cpc",
            "imgconverter",
            "xfer",
            "cpcwifi",
            "m4",
            "extern"
        ]),
        special: HashSet::from([":", "-"])
    }
}
