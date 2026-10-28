//! The C17 WebServer example transcoded into each template-compiler technology
//! (scripts/transcode_webserver.py): every design is generated, built with the real
//! toolchain and driven over HTTP with the same exchange as the C17 example test.
//! Go and Node.js are required, as for the other compiler tests; Kotlin, PHP and
//! QuickJS run with `--include-ignored` (see `tool`).
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};
use wirecraft_core::compiler::{self, CompileOptions};

/// Executable for a toolchain, overridable like the other compiler tests.
fn tool(variable: &str, default: &str) -> String {
    std::env::var(variable).unwrap_or_else(|_| default.into())
}

/// Generates `examples/webserver-<language>.orch` into a new directory.
fn generate(language: &str, out: &Path) -> Vec<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../examples/webserver-{language}.orch"));
    let d = wirecraft_core::store::load(&path).unwrap();
    let project = compiler::generate(&d, &d.root_node_id, CompileOptions::default())
        .unwrap_or_else(|e| {
            panic!("{language}: {}", e.message);
        });
    // Every authored body maps to its generated lines, after any planner corrections.
    let mut mapped = 0;
    for file in &project.files {
        let lines: Vec<&str> = file.content.lines().collect();
        for map in &file.source_map {
            let n = &d.nodes[&map.node_id];
            let text = if map.section == "initialization" {
                &n.text.initialization
            } else {
                &n.text.source
            };
            let first = text
                .lines()
                .nth(map.source_start_line - 1)
                .unwrap_or("")
                .trim();
            assert_eq!(
                lines[map.generated_start_line - 1].trim(),
                first,
                "{language}: {} {} maps to the wrong line of {}",
                n.name,
                map.section,
                file.path
            );
            mapped += 1;
        }
    }
    assert!(mapped >= 10, "{language}: only {mapped} mapped sections");
    project
        .files
        .iter()
        .map(|file| {
            let target = out.join(&file.path);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(&target, &file.content).unwrap();
            target
        })
        .collect()
}

fn built(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Runs the server and checks routes, fallbacks, rejection, HEAD and the request limit.
fn exercise(mut server: Command, display: &str) {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut server = server
        .env("WEBSERVER_PORT", port.to_string())
        .env("WEBSERVER_MAX_REQUESTS", "5")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let exchange = |raw: &str| {
        for _ in 0..200 {
            if let Ok(mut stream) = std::net::TcpStream::connect(("127.0.0.1", port)) {
                stream
                    .set_read_timeout(Some(Duration::from_secs(10)))
                    .unwrap();
                stream.write_all(raw.as_bytes()).unwrap();
                let mut reply = String::new();
                stream.read_to_string(&mut reply).unwrap();
                return reply;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("{display}: server did not accept connections");
    };
    let length = |reply: &str| -> usize {
        reply
            .lines()
            .find_map(|l| l.strip_prefix("Content-Length: "))
            .unwrap_or_else(|| panic!("{display}: no Content-Length in {reply}"))
            .trim()
            .parse()
            .unwrap()
    };
    let body = |reply: &str| reply.split_once("\r\n\r\n").unwrap().1.to_string();
    let about = exchange("GET /home/about?x=1 HTTP/1.1\r\nHost: test\r\n\r\n");
    assert!(
        about.starts_with("HTTP/1.1 200 OK\r\n"),
        "{display}: {about}"
    );
    assert!(about.ends_with("Wirecraft model.\n"), "{display}: {about}");
    assert_eq!(length(&about), body(&about).len(), "{display}: {about}");
    let user = exchange("GET /api/user HTTP/1.1\r\n\r\n");
    assert!(
        user.contains("Content-Type: application/json\r\n") && user.contains("\"Ada Lovelace\""),
        "{display}: {user}"
    );
    let missing = exchange("GET /missing HTTP/1.1\r\n\r\n");
    assert!(
        missing.starts_with("HTTP/1.1 404") && missing.ends_with("404 Not Found: /missing\n"),
        "{display}: {missing}"
    );
    let post = exchange("POST /home/version HTTP/1.1\r\nContent-Length: 3\r\n\r\nabc");
    assert!(
        post.starts_with("HTTP/1.1 405") && post.contains("Allow: GET, HEAD\r\n"),
        "{display}: {post}"
    );
    // Rejected before entering the network, with unread bytes still queued.
    let oversized = exchange(&format!(
        "GET / HTTP/1.1\r\nX: {}\r\n\r\n",
        "a".repeat(9000)
    ));
    assert!(
        oversized.starts_with("HTTP/1.1 431"),
        "{display}: {oversized}"
    );
    let head = exchange("HEAD /home/version HTTP/1.1\r\n\r\n");
    let version = format!("WebServer 0.1.0 (Wirecraft {display})\n");
    assert_eq!(length(&head), version.len(), "{display}: {head}");
    assert!(head.ends_with("\r\n\r\n"), "{display}: {head}");
    // The fifth resolved request ends the run.
    let (tx, rx) = std::sync::mpsc::channel();
    let mut stdout = server.stdout.take().unwrap();
    let mut stderr = server.stderr.take().unwrap();
    std::thread::spawn(move || {
        let (mut out, mut err) = (String::new(), String::new());
        let _ = stdout.read_to_string(&mut out);
        let _ = stderr.read_to_string(&mut err);
        let _ = tx.send((out, err));
    });
    let (log, errors) = rx
        .recv_timeout(Duration::from_secs(30))
        .unwrap_or_else(|_| {
            let _ = server.kill();
            panic!("{display}: server did not stop after its request limit");
        });
    let status = server.wait().unwrap();
    assert!(status.success(), "{display}: {status}\n{log}\n{errors}");
    assert!(
        log.contains("webserver: listening on http://127.0.0.1:"),
        "{display}: {log}"
    );
    assert_eq!(
        log.lines().filter(|l| l.starts_with("req-")).count(),
        5,
        "{display}: {log}"
    );
    assert!(
        log.contains("req-4 POST /home/version 405 "),
        "{display}: {log}"
    );
}

#[test]
fn go_webserver() {
    let temp = tempfile::tempdir().unwrap();
    let files = generate("go", temp.path());
    let exe = temp.path().join("webserver");
    built(
        Command::new(tool("WIRECRAFT_TEST_GO", "go"))
            .arg("build")
            .arg("-o")
            .arg(&exe)
            .arg(&files[0])
            .current_dir(temp.path()),
    );
    exercise(Command::new(exe), "Go");
}
