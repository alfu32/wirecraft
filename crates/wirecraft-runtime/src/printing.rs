//! CUPS is optional. Listing is read-only; submission is an explicit supervised job.
use crate::{backend::Application, service::Request};
use serde_json::{Value, json};
use wirecraft_core::{Error, Result};
pub async fn list() -> Result<Value> {
    let mut command = tokio::process::Command::new("lpstat");
    command.arg("-a").kill_on_drop(true);
    crate::process::terminate_with_parent(&mut command);
    let output = tokio::time::timeout(std::time::Duration::from_secs(5), command.output())
        .await
        .map_err(|_| Error::new("timeout", "Printer discovery timed out"))?
        .map_err(|e| Error::new("tool_unavailable", format!("CUPS lpstat: {e}")))?;
    if !output.status.success() {
        return Err(Error::new(
            "unavailable",
            format!(
                "CUPS printer discovery: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }
    let printers = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            line.split_whitespace()
                .next()
                .map(|name| json!({"name":name,"status":line}))
        })
        .collect::<Vec<_>>();
    Ok(
        json!({"printers":printers,"adapter":"cups","note":"Wirecraft always provides file export; CUPS queues require explicit Print submission."}),
    )
}
pub async fn submit(app: &Application, arguments: Value) -> Result<Value> {
    let printer = arguments["printer"]
        .as_str()
        .ok_or_else(|| Error::new("invalid_input", "Choose a printer"))?
        .to_string();
    let queues = list().await?;
    if !queues["printers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["name"] == printer)
    {
        return Err(Error::new(
            "not_found",
            "Printer queue is no longer available",
        ));
    }
    let service = app.service.clone();
    let (dir, path) = tokio::task::spawn_blocking(move || {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("wirecraft-print.pdf");
        let mut arguments = arguments;
        arguments["format"] = json!("pdf");
        arguments["path"] = json!(path);
        service
            .lock()
            .map_err(|_| Error::new("internal", "Project lock poisoned"))?
            .execute(Request::new("export.write", arguments))?;
        Ok::<_, Error>((dir, path))
    })
    .await
    .map_err(|e| Error::new("internal", e.to_string()))??;
    let job = app
        .jobs
        .start(
            "lp",
            &[
                "-d".into(),
                printer,
                "--".into(),
                path.to_string_lossy().into_owned(),
            ],
            dir.path(),
            std::time::Duration::from_secs(30),
        )
        .await?;
    let id = job["id"].as_str().unwrap().to_string();
    let jobs = app.jobs.clone();
    tokio::spawn(async move {
        // Keep the spool fixture alive until lp has consumed it. Jobs owns process cleanup.
        loop {
            if jobs.get(&id).await.is_ok_and(|v| v["status"] != "running") {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        drop(dir);
    });
    Ok(job)
}
