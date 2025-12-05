use cpclib_basmdoc::DocumentationPage;

fn main() {
    let args: Vec<_> = std::env::args().collect();

    let input = &args[1];
    let output = &args[2];

    let doc = DocumentationPage::for_file(input);
    let md = doc.to_markdown();

    std::fs::write(output, md);
}
