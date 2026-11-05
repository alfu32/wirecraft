use std::{
    fs,
    path::{Path, PathBuf},
};
fn visit(path: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            visit(&path, files);
        } else if path.extension().is_some_and(|e| e == "orch") {
            files.push(path);
        }
    }
}
fn main() {
    let root = Path::new("resources/archetypes");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = vec![];
    visit(root, &mut files);
    files.sort();
    let mut generated = String::from("pub const BUILTINS: &[(&str,&str)] = &[\n");
    for file in files {
        let id = file
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        generated.push_str(&format!(
            "({id:?}, include_str!({:?})),\n",
            file.canonicalize().unwrap()
        ));
    }
    generated.push_str("];\n");
    let output = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    fs::write(output.join("archetypes.rs"), generated).unwrap();
    // Named explicitly: examples/ also holds the user's own projects and generated output.
    let examples = Path::new("../../examples");
    let mut generated = String::from("pub const FILES: &[(&str,&str)] = &[\n");
    for name in [
        "webserver.orch",
        "webserver-component.orch",
        "webserver-go.orch",
        "webserver-kotlin.orch",
        "webserver-nodejs.orch",
        "webserver-php.orch",
        "webserver-rust.orch",
        "webserver-systemjs.orch",
    ] {
        let file = examples.join(name);
        println!("cargo:rerun-if-changed={}", file.display());
        generated.push_str(&format!(
            "({name:?}, include_str!({:?})),\n",
            file.canonicalize().unwrap()
        ));
    }
    generated.push_str("];\n");
    fs::write(output.join("examples.rs"), generated).unwrap();
}
