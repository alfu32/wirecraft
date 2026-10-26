use base64::Engine;
use std::path::Path;
use wirecraft_core::{
    Error, Result,
    generation::{GeneratedProject, safe_relative_path},
    store,
};
/// Write a fully staged project. Never merge into a nonempty user directory.
pub fn write(project: &GeneratedProject, output: &Path) -> Result<()> {
    if output.exists() && (!output.is_dir() || std::fs::read_dir(output)?.next().is_some()) {
        return Err(Error::new(
            "already_exists",
            "Generated output must be a new or empty directory",
        ));
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let stage = parent.join(format!(".wirecraft-generate-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&stage)?;
    let result = (|| -> Result<()> {
        for file in &project.files {
            safe_relative_path(&file.path)?;
            if file.path == "wirecraft-source-map.json" {
                return Err(Error::new(
                    "invalid_input",
                    "Generated filename is reserved for source mappings",
                ));
            }
            let data = if let Some(binary) = &file.binary_content {
                base64::engine::general_purpose::STANDARD
                    .decode(binary)
                    .map_err(|e| Error::new("invalid_input", e.to_string()))?
            } else {
                file.content.as_bytes().to_vec()
            };
            store::atomic_write(&stage.join(&file.path), &data)?;
        }
        store::atomic_write(
            &stage.join("wirecraft-source-map.json"),
            &serde_json::to_vec_pretty(project)?,
        )?;
        if output.exists() {
            std::fs::remove_dir(output)?;
        }
        std::fs::rename(&stage, output)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(stage);
    }
    result
}
