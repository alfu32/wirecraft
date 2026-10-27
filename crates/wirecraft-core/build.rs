use std::{
    fs,
    path::{Path, PathBuf},
};
fn visit(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap() {
        let p = entry.unwrap().path();
        if p.is_dir() {
            visit(&p, files);
        } else {
            files.push(p);
        }
    }
}
fn main() {
    let root = Path::new("resources/compiler-templates");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = vec![];
    visit(root, &mut files);
    files.sort();
    let mut text = String::from("pub const ASSETS: &[(&str,&str)] = &[\n");
    for p in files {
        if p.extension()
            .is_some_and(|e| e == "peb" || e == "properties")
        {
            text.push_str(&format!(
                "({:?},include_str!({:?})),\n",
                p.strip_prefix(root).unwrap().to_string_lossy(),
                p.canonicalize().unwrap()
            ));
        }
    }
    text.push_str("];\n");
    fs::write(
        PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("templates.rs"),
        text,
    )
    .unwrap();
}
