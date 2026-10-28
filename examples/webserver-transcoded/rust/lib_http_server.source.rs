// HTTP/1.1 helpers for the WebServer network. Lent to consumers as `libhttp`.
const HTTP_MAX_HEADER: usize = 8192;
const HTTP_MAX_BODY: usize = 65536;

/// The standard library has no accept timeout, so a listening thread accepts and hands
/// connections over `accepted`; http_wait waits on that channel. Also: the connection
/// http_wait received (handed to the next http_accept) and the open connections, by the
/// number a ReqRep carries in `connection`.
struct HttpServer {
    accepted: Option<Receiver<TcpStream>>,
    address: Option<SocketAddr>,
    waiting: Option<TcpStream>,
    connections: BTreeMap<i64, TcpStream>,
    next_connection: i64,
}

static HTTP_SERVER: Mutex<HttpServer> = Mutex::new(HttpServer {
    accepted: None,
    address: None,
    waiting: None,
    connections: BTreeMap::new(),
    next_connection: 0,
});

/// Tells the listening thread to drop the listening socket at its next connection.
static HTTP_CLOSING: AtomicBool = AtomicBool::new(false);

fn http_server() -> MutexGuard<'static, HttpServer> {
    HTTP_SERVER.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn http_status_text(status: i64) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        413 => "Content Too Large",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        _ => "Unknown",
    }
}

/// Non-negative integer value of a header field (case-insensitive name), or -1.
fn http_header_number(header: &str, name: &str) -> i64 {
    for line in header.split("\r\n").skip(1) {
        if line.is_empty() {
            break;
        }
        let Some((field, value)) = line.split_once(':') else {
            continue;
        };
        if !field.eq_ignore_ascii_case(name) {
            continue;
        }
        let value = value.trim_start_matches([' ', '\t']);
        let digits = value.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return -1;
        }
        // Like strtol, an out-of-range value saturates instead of failing.
        return value[..digits].parse().unwrap_or(i64::MAX);
    }
    -1
}

/// Answers a connection that never became a request, then closes it.
fn http_reject(mut connection: TcpStream, status: i64) {
    let text = http_status_text(status);
    let reply = format!(
        "HTTP/1.1 {status} {text}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\n\
         Connection: close\r\nServer: Wirecraft-WebServer/0.1\r\n\r\n{status} {text}\n",
        text.len() + 5
    );
    let _ = connection.write_all(reply.as_bytes());
    // Closing with unread request bytes would reset the connection and lose the reply:
    // finish sending, then discard what the client still sends (bounded by the 2 s timeout).
    if connection.shutdown(Shutdown::Write).is_ok() {
        let mut discard = [0u8; 4096];
        let mut drained = 0;
        while drained < HTTP_MAX_BODY {
            match connection.read(&mut discard) {
                Ok(0) | Err(_) => break,
                Ok(count) => drained += count,
            }
        }
    }
}

fn http_listen(port: i64, any_address: bool) -> std::io::Result<()> {
    if !(1..=65535).contains(&port) {
        return Err(std::io::Error::new(ErrorKind::InvalidInput, "port out of range"));
    }
    let host = if any_address { "0.0.0.0" } else { "127.0.0.1" };
    let listener = TcpListener::bind((host, port as u16))?;
    let (sender, accepted) = channel();
    std::thread::spawn(move || {
        for connection in listener.incoming() {
            if HTTP_CLOSING.load(Ordering::SeqCst) {
                break;
            }
            if let Ok(connection) = connection {
                if sender.send(connection).is_err() {
                    break;
                }
            }
        }
    });
    let mut server = http_server();
    server.accepted = Some(accepted);
    server.address = Some(SocketAddr::from(([127, 0, 0, 1], port as u16)));
    Ok(())
}

/// Waits up to `timeout_ms` for a connection; true when http_accept has one to read.
fn http_wait(timeout_ms: u64) -> bool {
    let mut server = http_server();
    if server.waiting.is_some() {
        return true;
    }
    let Some(accepted) = &server.accepted else {
        return false;
    };
    let connection = if timeout_ms == 0 {
        accepted.try_recv().ok()
    } else {
        accepted.recv_timeout(Duration::from_millis(timeout_ms)).ok()
    };
    let ready = connection.is_some();
    server.waiting = connection;
    ready
}

fn http_close() {
    let mut server = http_server();
    server.waiting = None;
    server.accepted = None;
    // A connection of our own wakes the listening thread, which then closes the socket.
    if let Some(address) = server.address.take() {
        HTTP_CLOSING.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect_timeout(&address, Duration::from_secs(1));
    }
}

/// Reads one request from the waiting connection; None when there is none or when the
/// connection was answered here instead (408, 400, 413, 431).
fn http_accept(sequence: u64) -> Option<ReqRep> {
    let mut connection = http_server().waiting.take()?;
    let _ = connection.set_read_timeout(Some(Duration::from_secs(2)));
    let mut buffer = Vec::with_capacity(HTTP_MAX_HEADER);
    let mut chunk = [0u8; 4096];
    let end = loop {
        if buffer.len() >= HTTP_MAX_HEADER {
            http_reject(connection, 431);
            return None;
        }
        let room = chunk.len().min(HTTP_MAX_HEADER - buffer.len());
        match connection.read(&mut chunk[..room]) {
            Ok(count) if count > 0 => {
                buffer.extend_from_slice(&chunk[..count]);
                if let Some(end) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                    break end;
                }
            }
            result => {
                let timeout = matches!(&result, Err(error)
                    if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut));
                http_reject(connection, if buffer.is_empty() && timeout { 408 } else { 400 });
                return None;
            }
        }
    };
    let header_length = end + 4;
    let header = String::from_utf8_lossy(&buffer[..header_length]).into_owned();
    let body_length = http_header_number(&header, "Content-Length").max(0) as usize;
    if body_length > HTTP_MAX_BODY {
        http_reject(connection, 413);
        return None;
    }
    while buffer.len() < header_length + body_length {
        let room = chunk.len().min(header_length + body_length - buffer.len());
        match connection.read(&mut chunk[..room]) {
            Ok(count) if count > 0 => buffer.extend_from_slice(&chunk[..count]),
            _ => {
                http_reject(connection, 400);
                return None;
            }
        }
    }
    let body = String::from_utf8_lossy(&buffer[header_length..header_length + body_length]);
    let mut server = http_server();
    server.next_connection += 1;
    let number = server.next_connection;
    server.connections.insert(number, connection);
    Some(ReqRep {
        id: format!("req-{sequence}"),
        connection: number,
        request: Request {
            header,
            body: body.into_owned(),
        },
        ..Default::default()
    })
}

fn http_render_header(response: &Response) -> String {
    let content_type = if response.content_type.is_empty() {
        "text/plain; charset=utf-8"
    } else {
        &response.content_type
    };
    let allow = if response.status == 405 { "Allow: GET, HEAD\r\n" } else { "" };
    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\
         Server: Wirecraft-WebServer/0.1\r\n{allow}\r\n",
        response.status,
        http_status_text(response.status),
        response.body.len()
    )
}

/// Writes the response to the pending request's connection, logs it and closes it; 0 on success.
fn http_resolve(pending: &ReqRep, resolved: &ReqRep) -> i32 {
    let connection = http_server().connections.remove(&pending.connection);
    let head = pending.request.header.starts_with("HEAD ");
    let status = match connection {
        None => -1,
        Some(mut connection) => {
            let mut written = connection.write_all(resolved.response.header.as_bytes());
            if written.is_ok() && !head && !resolved.response.body.is_empty() {
                written = connection.write_all(resolved.response.body.as_bytes());
            }
            // Dropping the stream closes the connection.
            if written.is_ok() { 0 } else { -1 }
        }
    };
    let mut fields = pending.request.header.split_whitespace();
    let (method, path) = match (fields.next(), fields.next()) {
        (Some(method), Some(path)) => (method, path),
        _ => ("", ""),
    };
    println!(
        "{} {method} {path} {} {}",
        pending.id,
        resolved.response.status,
        resolved.response.body.len()
    );
    status
}
