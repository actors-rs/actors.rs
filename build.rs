extern crate skeptic;
use skeptic::*;

fn main() {
    // generates doc tests for `README.md`.
    // let mut mdbook_files = markdown_files_of_directory("mkdocs/docs/");
    // mdbook_files.push("README.md".into());
    // generate_doc_tests(&mdbook_files);
    generate_doc_tests(&[
        "README.md",
        "mkdocs/docs/actors.md",
        "mkdocs/docs/hierarchy.md",
        "mkdocs/docs/messaging.md",
        "mkdocs/docs/supervision.md",
    ]);
}
