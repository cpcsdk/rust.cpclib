
use std::collections::BTreeSet;

use egui_code_editor::Syntax;

pub fn syntax() -> Syntax {
    Syntax {
        language: "bndbuild",
        case_sensitive: true,
        comment: "#",
        comment_multiline: [r"/*", r"*/"],
        keywords: BTreeSet::from([
            "tgt",
            "target",
            "targets",
            //        "build",
            "dep",
            "dependency",
            "dependencies",
            "requires",
            "cmd",
            "command",
            "launch",
            "run",
            "help",
            "constraint"
        ]),
        types: BTreeSet::from([
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
            "extern",
            "Os",
            "Linux",
            "Windows",
            "MacOsx"
        ]),
        special: BTreeSet::from([":", "-"])
    }
}
