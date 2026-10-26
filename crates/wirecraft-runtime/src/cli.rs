use crate::{
    backend::Application,
    mcp,
    service::{Request, Service},
};
use serde_json::json;
use std::path::PathBuf;
use wirecraft_core::{Document, store};
pub async fn run(mut args: impl Iterator<Item = String>) -> anyhow::Result<()> {
    let mode = args.next().unwrap_or_else(|| "help".into());
    if mode == "help" || mode == "--help" {
        println!(
            "Wirecraft {}\n\nwirecraft serve [--project FILE] [--port PORT] [--token TOKEN] [--recovery FILE]\nwirecraft mcp-stdio [--project FILE]\nwirecraft validate FILE\nwirecraft new FILE\n\nserve binds 127.0.0.1 only and prints a JSON readiness record.\nNo project file is saved unless requested. Recovery is opt-in.",
            env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    }
    if mode == "validate" {
        let path = args
            .next()
            .ok_or_else(|| anyhow::anyhow!("Missing project path"))?;
        let d = store::load(&PathBuf::from(path))?;
        println!(
            "{}",
            json!({"valid":true,"nodes":d.nodes.len(),"diagnostics":d.validate()})
        );
        return Ok(());
    }
    if mode == "new" {
        let path = PathBuf::from(
            args.next()
                .ok_or_else(|| anyhow::anyhow!("Missing project path"))?,
        );
        if path.exists() {
            anyhow::bail!("Refusing to overwrite existing project");
        }
        store::save(&Document::new("Untitled Wirecraft"), &path)?;
        return Ok(());
    }
    if mode != "serve" && mode != "mcp-stdio" {
        anyhow::bail!("Unknown mode '{mode}'; use --help");
    }
    let mut project = None;
    let mut port = 0u16;
    let mut token = uuid::Uuid::new_v4().to_string();
    let mut recovery = None;
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| anyhow::anyhow!("Missing value for {flag}"))?;
        match flag.as_str() {
            "--project" => project = Some(PathBuf::from(value)),
            "--port" => port = value.parse()?,
            "--token" => {
                if value.len() < 16 {
                    anyhow::bail!("Token must have at least 16 characters");
                }
                token = value
            }
            "--recovery" => recovery = Some(PathBuf::from(value)),
            _ => anyhow::bail!("Unknown option {flag}"),
        }
    }
    let service = if let Some(path) = &project {
        Service::new(store::load(path)?, project)
    } else if let Some(path) = recovery.as_ref().filter(|p| p.exists()) {
        Service::read_recovery(path)?
    } else {
        Service::new(Document::new("Untitled Wirecraft"), None)
    };
    if let Some(path) = &recovery {
        service.write_recovery(path)?;
    }
    let app = Application::new(service, token.clone(), recovery);
    if mode == "mcp-stdio" {
        mcp::stdio(&app).await?;
        return Ok(());
    }
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    let addr = listener.local_addr()?;
    println!(
        "{}",
        json!({"status":"ready","endpoint":format!("http://{addr}"),"mcp":format!("http://{addr}/mcp"),"token":token,"pid":std::process::id()})
    );
    let signal_app = app.clone();
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            if let Ok(mut term) =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            {
                tokio::select! {_ = tokio::signal::ctrl_c()=>{}, _ = term.recv()=>{}}
            } else {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
        #[cfg(not(unix))]
        let _ = tokio::signal::ctrl_c().await;
        let _ = signal_app
            .execute(Request::new("system.shutdown", json!({})))
            .await;
    });
    app.serve(listener).await?;
    Ok(())
}
