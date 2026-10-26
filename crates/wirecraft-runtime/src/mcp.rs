//! Thin protocol mapping. All tool operations call the shared application service.
use crate::{backend::Application, service::Request};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
const GUIDE: &str = include_str!("../resources/agent-guide.md");
fn definitions(which: &str) -> Value {
    serde_json::from_str(match which {
        "tools" => include_str!("../resources/tools.json"),
        "resources" => include_str!("../resources/resources.json"),
        _ => include_str!("../resources/prompts.json"),
    })
    .unwrap()
}
pub async fn handle(app: &Application, request: Value) -> Option<Value> {
    let id = request.get("id").cloned();
    if !request.is_object() || request["jsonrpc"] != "2.0" || !request["method"].is_string() {
        return Some(error(
            id.unwrap_or(Value::Null),
            -32600,
            "Invalid JSON-RPC request",
        ));
    }
    let method = request["method"].as_str().unwrap();
    let params = request.get("params").cloned().unwrap_or(json!({}));
    let result = match method {
        "initialize" => {
            let requested = params["protocolVersion"].as_str().unwrap_or("");
            let version = if [
                "2026-07-28",
                "2025-11-25",
                "2025-06-18",
                "2025-03-26",
                "2024-11-05",
            ]
            .contains(&requested)
            {
                requested
            } else {
                "2025-11-25"
            };
            Ok(
                json!({"protocolVersion":version,"serverInfo":{"name":"wirecraft","version":env!("CARGO_PKG_VERSION")},"capabilities":{"tools":{},"resources":{"listChanged":false},"prompts":{"listChanged":false}},"instructions":GUIDE}),
            )
        }
        "server/discover" => Ok(
            json!({"resultType":"complete","supportedVersions":["2026-07-28"],"capabilities":{"tools":{},"resources":{"listChanged":false},"prompts":{"listChanged":false}},"instructions":GUIDE,"ttlMs":0,"cacheScope":"private"}),
        ),
        "notifications/initialized" => return None,
        "ping" => Ok(json!({})),
        "tools/list" => Ok(definitions("tools")),
        "resources/list" => Ok(definitions("resources")),
        "prompts/list" => Ok(definitions("prompts")),
        "tools/call" => {
            let name = params["name"].as_str().unwrap_or("");
            let known = definitions("tools")["tools"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t["name"] == name);
            let operation = if known {
                let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
                if name == "wirecraft.workbench" {
                    let operation = arguments["operation"].as_str().unwrap_or("");
                    let command = match operation {
                        "preview_archetype" => "archetypes.preview",
                        "preview_export" => "export.preview",
                        "export" => "export.write",
                        "prepare_ai_prompt" => "ai.prompt",
                        "settings" => "settings.get",
                        "replace_settings" => "settings.replace",
                        "mcp_status" => "mcp.status",
                        "set_mcp_enabled" => "mcp.set_enabled",
                        "list_printers" => "printers.list",
                        "print" => "print.submit",
                        _ => "unsupported.workbench",
                    };
                    app.execute(Request::new(
                        command,
                        arguments.get("arguments").cloned().unwrap_or(json!({})),
                    ))
                    .await
                } else if name == "wirecraft.language_server" || name == "wirecraft.build" {
                    let prefix = if name == "wirecraft.build" {
                        "jobs"
                    } else {
                        "lsp"
                    };
                    let operation = arguments["operation"].as_str().unwrap_or("");
                    app.execute(Request::new(
                        &format!("{prefix}.{operation}"),
                        arguments.get("arguments").cloned().unwrap_or(json!({})),
                    ))
                    .await
                } else {
                    let command = match name {
                        "wirecraft.open_project" => "file.open",
                        "wirecraft.generate_files" => "generate.files",
                        "wirecraft.compile_c17" => "compile.c17",
                        "wirecraft.analyze_c17" => "editor.analyze_generated",
                        "wirecraft.save_archetype" => "archetypes.save",
                        "wirecraft.save_project" => "file.save",
                        "wirecraft.undo" => "edit.undo",
                        "wirecraft.redo" => "edit.redo",
                        "wirecraft.copy_entities" => "entities.copy",
                        "wirecraft.paste_entities" => "entities.paste",
                        "wirecraft.list_archetypes" => "archetypes.list",
                        "wirecraft.import_archetype" => "archetypes.import",
                        "wirecraft.analyze_network" => "analysis.report",
                        "wirecraft.export_diagram" => "export.diagram",
                        "wirecraft.component_fiches" => "export.markdown",
                        other => other,
                    };
                    app.execute(Request::new(command, arguments)).await
                }
            } else {
                Err(wirecraft_core::Error::new(
                    "not_found",
                    format!("Unknown MCP tool '{name}'"),
                ))
            };
            Ok(match operation {
                Ok(data) => {
                    json!({"content":[{"type":"text","text":data.to_string()}],"structuredContent":data,"isError":false})
                }
                Err(e) => {
                    json!({"content":[{"type":"text","text":e.message}],"structuredContent":{"error":e},"isError":true})
                }
            })
        }
        "resources/read" => {
            let uri = params["uri"].as_str().unwrap_or("");
            let text = match uri {
                "threadwork://instructions" | "threadwork://agent-guide" => Ok(GUIDE.into()),
                "threadwork://design" => app
                    .execute(Request::new("threadwork.get_design", json!({})))
                    .await
                    .map(|v| v["document"].to_string()),
                "threadwork://technologies" => app
                    .execute(Request::new("threadwork.list_technologies", json!({})))
                    .await
                    .map(|v| v.to_string()),
                "threadwork://commands" => app
                    .execute(Request::new("threadwork.get_command_catalog", json!({})))
                    .await
                    .map(|v| v.to_string()),
                _ => Err(wirecraft_core::Error::new(
                    "not_found",
                    "Unknown resource URI",
                )),
            };
            text.map(|text|json!({"contents":[{"uri":uri,"mimeType":if uri.ends_with("agent-guide"){"text/markdown"}else if uri.ends_with("instructions"){"text/plain"}else{"application/json"},"text":text}]}))
        }
        "prompts/get" => match params["name"].as_str().unwrap_or("") {
            "threadwork.inspect_design" => Ok(
                json!({"description":"Inspect the open design","messages":[{"role":"user","content":{"type":"text","text":"Use threadwork.get_design, threadwork.validate_design, and threadwork.analyze_network before proposing changes."}}]}),
            ),
            "threadwork.edit_entity" => Ok(
                json!({"description":"Inspect and edit an entity","messages":[{"role":"user","content":{"type":"text","text":"Use threadwork.get_entity first, then apply the smallest threadwork.update_entity patch needed."}}]}),
            ),
            _ => Err(wirecraft_core::Error::new("not_found", "Unknown prompt")),
        },
        _ => return id.map(|id| error(id, -32601, "Unknown MCP method")),
    };
    id.map(|id|match result{Ok(mut value)=>{value["resultType"]=json!("complete");value["_meta"]=json!({"io.modelcontextprotocol/serverInfo":{"name":"wirecraft","version":env!("CARGO_PKG_VERSION")}});json!({"jsonrpc":"2.0","id":id,"result":value})},Err(e)=>error(id,-32602,&e.message)})
}
fn error(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
pub async fn stdio(app: &Application) -> anyhow::Result<()> {
    let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
    let mut stdout = tokio::io::stdout();
    while let Some(line) = lines.next_line().await? {
        if line.len() > 16 * 1024 * 1024 {
            anyhow::bail!("MCP message exceeds 16 MiB");
        }
        let response = match serde_json::from_str(&line) {
            Ok(request) => handle(app, request).await,
            Err(_) => Some(error(Value::Null, -32700, "Parse error")),
        };
        if let Some(response) = response {
            stdout.write_all(response.to_string().as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
    }
    app.shutdown().await;
    Ok(())
}
