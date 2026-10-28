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
    let mut text = String::from("pub const ASSETS: &[(&str,&str)] = &[\n");
    // Baseline templates keep their relative names; the Wirecraft overlay and the
    // Wirecraft-authored sets (no Threadworks baseline) are prefixed.
    for (root, prefix) in [
        ("resources/compiler-templates", ""),
        ("resources/compiler-templates-wirecraft", "wirecraft/"),
        ("resources/compiler-templates-native", "native/"),
    ] {
        embed(Path::new(root), prefix, &mut text);
    }
    text.push_str("];\n");
    fs::write(
        PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("templates.rs"),
        text,
    )
    .unwrap();
}
fn embed(root: &Path, prefix: &str, text: &mut String) {
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = vec![];
    visit(root, &mut files);
    files.sort();
    for p in files {
        if p.extension()
            .is_some_and(|e| e == "peb" || e == "properties")
        {
            text.push_str(&format!(
                "({:?},include_str!({:?})),\n",
                format!(
                    "{prefix}{}",
                    p.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/")
                ),
                p.canonicalize().unwrap()
            ));
        }
    }
}
