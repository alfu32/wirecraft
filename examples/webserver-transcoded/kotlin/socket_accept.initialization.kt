// Like atoi, text that is not a number yields port 0, which fails to listen.
val port = System.getenv("WEBSERVER_PORT")?.let { it.trim().toIntOrNull() ?: 0 } ?: 8080
val anyAddress = System.getenv("WEBSERVER_BIND_ANY") == "1"
if (libhttp.http_listen(port, anyAddress) != 0) {
    System.err.println("webserver: cannot listen on port $port")
    throw IllegalStateException("webserver: cannot listen on port $port")
}
println("webserver: listening on http://${if (anyAddress) "0.0.0.0" else "127.0.0.1"}:$port")
System.out.flush()
