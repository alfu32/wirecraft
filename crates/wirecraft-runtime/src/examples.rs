//! Example projects and the home screen's overview of recent files.
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};
use wirecraft_core::{Error, Result, store, thumbnail};

include!(concat!(env!("OUT_DIR"), "/examples.rs"));

pub struct Example {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    /// The project, then the files it refers to.
    pub files: &'static [&'static str],
}
pub const EXAMPLES: &[Example] = &[
    Example {
        id: "webserver",
        title: "Web Server, C17",
        description: "A C17 program that uses the Web Server component",
        files: &["webserver.orch", "webserver-component.orch"],
    },
    Example {
        id: "webserver-component",
        title: "Web Server Component",
        description: "A reusable C17 design that publishes its ports through Exteriors",
        files: &["webserver-component.orch"],
    },
    Example {
        id: "webserver-rust",
        title: "Web Server, Rust",
        description: "The Web Server design generating a Rust project",
        files: &["webserver-rust.orch"],
    },
    Example {
        id: "webserver-go",
        title: "Web Server, Go",
        description: "The Web Server design generating a Go project",
        files: &["webserver-go.orch"],
    },
    Example {
        id: "webserver-kotlin",
        title: "Web Server, Kotlin",
        description: "The Web Server design generating a Kotlin project for the JVM",
        files: &["webserver-kotlin.orch"],
    },
    Example {
        id: "webserver-nodejs",
        title: "Web Server, Node.js",
        description: "The Web Server design generating a Node.js project",
        files: &["webserver-nodejs.orch"],
    },
    Example {
        id: "webserver-php",
        title: "Web Server, PHP",
        description: "The Web Server design generating a PHP project",
        files: &["webserver-php.orch"],
    },
    Example {
        id: "webserver-systemjs",
        title: "Web Server, systemJS",
        description: "The Web Server design generating a systemJS project, run on Bun",
        files: &["webserver-systemjs.orch"],
    },
];
fn text(name: &str) -> Option<&'static str> {
    FILES.iter().find(|(file, _)| *file == name).map(|f| f.1)
}
/// Writes an example under `directory` and returns its project. Each version has its own
/// copies, and a copy that exists is kept: saving over an example must not lose that work
/// the next time the example opens.
pub fn materialize(id: &str, directory: &Path) -> Result<PathBuf> {
    let example = EXAMPLES
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| Error::new("invalid_input", format!("Unknown example '{id}'")))?;
    let directory = directory.join(env!("CARGO_PKG_VERSION"));
    fs::create_dir_all(&directory)?;
    for name in example.files {
        let path = directory.join(name);
        if !path.exists() {
            let content = text(name)
                .ok_or_else(|| Error::new("internal", format!("Example '{name}' not embedded")))?;
            store::atomic_write(&path, content.as_bytes())?;
        }
    }
    Ok(directory.join(example.files[0]))
}
/// Recent projects as they are on disk now, and the examples. A file that is gone or no
/// longer a project is still listed, so that the user can see what happened to it.
pub fn overview(recent: &[PathBuf]) -> Value {
    let now = SystemTime::now();
    let recent = recent
        .iter()
        .map(|path| {
            let modified = fs::metadata(path).and_then(|m| m.modified()).ok();
            json!({
                "path": path,
                "name": path.file_name().map(|n| n.to_string_lossy()),
                "exists": modified.is_some(),
                "secondsAgo": modified.and_then(|m| now.duration_since(m).ok()).map(|d| d.as_secs()),
                "thumbnail": store::load(path).ok().map(|d| thumbnail::of(&d)),
            })
        })
        .collect::<Vec<_>>();
    let examples = EXAMPLES
        .iter()
        .map(|e| {
            json!({
                "id": e.id,
                "title": e.title,
                "description": e.description,
                "thumbnail": text(e.files[0])
                    .and_then(|t| store::decode(t).ok())
                    .map(|d| thumbnail::of(&d)),
            })
        })
        .collect::<Vec<_>>();
    json!({"recent": recent, "examples": examples})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_example_is_embedded_and_opens_from_its_copy() {
        let dir = tempfile::tempdir().unwrap();
        for example in EXAMPLES {
            let path = materialize(example.id, dir.path()).unwrap();
            let document = store::load(&path).unwrap();
            document.ensure_valid().unwrap();
            assert!(!thumbnail::of(&document).boxes.is_empty(), "{}", example.id);
        }
        // Every embedded file is some example's project.
        for (name, _) in FILES {
            assert!(EXAMPLES.iter().any(|e| e.files[0] == *name), "{name}");
        }
        // The C17 program finds the component it refers to beside it.
        let program = materialize("webserver", dir.path()).unwrap();
        assert!(program.with_file_name("webserver-component.orch").exists());
        assert!(materialize("../secrets", dir.path()).is_err());
    }
    #[test]
    fn opening_an_example_again_keeps_what_was_saved_over_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = materialize("webserver-rust", dir.path()).unwrap();
        let mut document = store::load(&path).unwrap();
        let root = document.root_node_id.clone();
        document.patch(&root, &json!({"name":"edited"})).unwrap();
        store::save(&document, &path).unwrap();
        let again = materialize("webserver-rust", dir.path()).unwrap();
        assert_eq!(again, path);
        assert_eq!(store::load(&again).unwrap().project_name(), "edited");
    }
    #[test]
    fn the_overview_lists_a_missing_or_corrupt_recent_file_without_a_picture() {
        let dir = tempfile::tempdir().unwrap();
        let corrupt = dir.path().join("corrupt.orch");
        fs::write(&corrupt, "{bad").unwrap();
        let good = materialize("webserver-go", dir.path()).unwrap();
        let before = fs::read(&good).unwrap();
        let value = overview(&[good.clone(), corrupt.clone(), dir.path().join("gone.orch")]);
        let recent = value["recent"].as_array().unwrap();
        assert_eq!(recent[0]["exists"], true);
        assert!(recent[0]["thumbnail"]["boxes"].is_array());
        assert_eq!(recent[1]["exists"], true);
        assert!(recent[1]["thumbnail"].is_null());
        assert_eq!(recent[2]["exists"], false);
        assert!(recent[2]["secondsAgo"].is_null());
        assert_eq!(value["examples"].as_array().unwrap().len(), EXAMPLES.len());
        assert_eq!(fs::read(&good).unwrap(), before, "reading only");
    }
}
