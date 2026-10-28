let port: i64 = match std::env::var("WEBSERVER_PORT") {
    Ok(text) if !text.is_empty() => text.parse().unwrap_or(0),
    _ => 8080,
};
let any_address = std::env::var("WEBSERVER_BIND_ANY").as_deref() == Ok("1");
if let Err(error) = libhttp.http_listen(port, any_address) {
    eprintln!("webserver: cannot listen on port {port}");
    return Err(error.into());
}
let host = if any_address { "0.0.0.0" } else { "127.0.0.1" };
println!("webserver: listening on http://{host}:{port}");
