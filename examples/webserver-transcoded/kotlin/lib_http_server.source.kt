// HTTP/1.1 helpers for the WebServer network. Lent to consumers as `libhttp`.
private const val HTTP_MAX_HEADER = 8192
private const val HTTP_MAX_BODY = 65536

// Listening socket shared by the http_* functions; null when closed.
private var httpListener: ServerSocket? = null

// A connection that http_wait accepted, handed to the next http_accept.
private var httpWaiting: Socket? = null

// Open connections, by the number a ReqRep carries in `connection`.
private val httpConnections = mutableMapOf<Int, Socket>()
private var httpNextConnection = 0

fun http_status_text(status: Int): String = when (status) {
    200 -> "OK"
    400 -> "Bad Request"
    404 -> "Not Found"
    405 -> "Method Not Allowed"
    408 -> "Request Timeout"
    413 -> "Content Too Large"
    431 -> "Request Header Fields Too Large"
    500 -> "Internal Server Error"
    else -> "Unknown"
}

// Non-negative integer value of a header field (case-insensitive name), or -1.
private fun http_header_number(header: String, name: String): Int {
    for (line in header.split("\r\n").drop(1)) {
        if (line.isEmpty()) break
        val colon = line.indexOf(':')
        if (colon != name.length || !line.substring(0, colon).equals(name, ignoreCase = true)) continue
        val digits = line.substring(colon + 1).trimStart(' ', '\t').takeWhile { it in '0'..'9' }
        // Like strtol, an out-of-range value saturates instead of failing.
        return if (digits.isEmpty()) -1 else digits.toIntOrNull() ?: Int.MAX_VALUE
    }
    return -1
}

// Answers a connection that never became a request, then closes it.
private fun http_reject(connection: Socket, status: Int) {
    val text = http_status_text(status)
    val reply = "HTTP/1.1 $status $text\r\nContent-Type: text/plain; charset=utf-8\r\n" +
        "Content-Length: ${text.length + 5}\r\nConnection: close\r\nServer: Wirecraft-WebServer/0.1\r\n\r\n" +
        "$status $text\n"
    try {
        connection.getOutputStream().write(reply.toByteArray(Charsets.ISO_8859_1))
        // Closing with unread request bytes would reset the connection and lose the reply:
        // finish sending, then discard what the client still sends (bounded by the 2 s timeout).
        connection.shutdownOutput()
        val input = connection.getInputStream()
        val discard = ByteArray(4096)
        var drained = 0
        while (drained < HTTP_MAX_BODY) {
            val count = input.read(discard)
            if (count <= 0) break
            drained += count
        }
    } catch (error: IOException) {
        // The client went away; there is nothing left to answer.
    }
    connection.close()
}

fun http_listen(port: Int, anyAddress: Boolean): Int {
    if (port <= 0 || port > 65535) return -1
    return try {
        val listener = ServerSocket()
        listener.reuseAddress = true
        listener.bind(InetSocketAddress(if (anyAddress) "0.0.0.0" else "127.0.0.1", port), 64)
        httpListener = listener
        0
    } catch (error: IOException) {
        -1
    }
}

// Waits up to timeoutMs for a connection; true when http_accept has one to read.
fun http_wait(timeoutMs: Int): Boolean {
    val listener = httpListener ?: return false
    if (httpWaiting != null) return true
    // A zero timeout would wait forever; wait at least 1 ms.
    listener.soTimeout = maxOf(timeoutMs, 1)
    return try {
        httpWaiting = listener.accept()
        true
    } catch (error: IOException) {
        false
    }
}

fun http_close() {
    httpWaiting?.close()
    httpWaiting = null
    httpListener?.close()
    httpListener = null
}

// Reads one request from the waiting connection: the request and 1, or null and 0 when
// the connection was answered here instead (408, 400, 413, 431).
fun http_accept(sequence: Long): Pair<ReqRep?, Int> {
    val connection = httpWaiting ?: return Pair(null, 0)
    httpWaiting = null
    var data = ByteArray(0)
    val chunk = ByteArray(4096)
    try {
        connection.soTimeout = 2000
        val input = connection.getInputStream()
        var end = -1
        while (end < 0) {
            if (data.size >= HTTP_MAX_HEADER) {
                http_reject(connection, 431)
                return Pair(null, 0)
            }
            val count = input.read(chunk, 0, minOf(chunk.size, HTTP_MAX_HEADER - data.size))
            if (count < 0) {
                http_reject(connection, 400)
                return Pair(null, 0)
            }
            data += chunk.copyOf(count)
            end = String(data, Charsets.ISO_8859_1).indexOf("\r\n\r\n")
        }
        val headerLength = end + 4
        val header = String(data, 0, headerLength, Charsets.ISO_8859_1)
        val bodyLength = maxOf(http_header_number(header, "Content-Length"), 0)
        if (bodyLength > HTTP_MAX_BODY) {
            http_reject(connection, 413)
            return Pair(null, 0)
        }
        while (data.size < headerLength + bodyLength) {
            val count = input.read(chunk, 0, minOf(chunk.size, headerLength + bodyLength - data.size))
            if (count < 0) {
                http_reject(connection, 400)
                return Pair(null, 0)
            }
            data += chunk.copyOf(count)
        }
        httpNextConnection++
        httpConnections[httpNextConnection] = connection
        return Pair(
            ReqRep(
                id = "req-$sequence",
                connection = httpNextConnection.toDouble(),
                request = Request(header = header, body = String(data, headerLength, bodyLength, Charsets.ISO_8859_1)),
                response = Response(status = 0.0, content_type = "", header = "", body = ""),
            ),
            1,
        )
    } catch (error: SocketTimeoutException) {
        http_reject(connection, if (data.isEmpty()) 408 else 400)
    } catch (error: IOException) {
        http_reject(connection, 400)
    }
    return Pair(null, 0)
}

fun http_render_header(response: Response): String {
    val status = response.status.toInt()
    val type = response.content_type.ifEmpty { "text/plain; charset=utf-8" }
    val allow = if (status == 405) "Allow: GET, HEAD\r\n" else ""
    val length = response.body.toByteArray(Charsets.UTF_8).size
    return "HTTP/1.1 $status ${http_status_text(status)}\r\nContent-Type: $type\r\nContent-Length: $length\r\n" +
        "Connection: close\r\nServer: Wirecraft-WebServer/0.1\r\n$allow\r\n"
}

// Writes the response to the pending request's connection, logs it and closes it; 0 on success.
fun http_resolve(pending: ReqRep, resolved: ReqRep): Int {
    val connection = httpConnections.remove(pending.connection.toInt())
    val head = pending.request.header.startsWith("HEAD ")
    val body = resolved.response.body.toByteArray(Charsets.UTF_8)
    var status = 0
    try {
        val output = connection?.getOutputStream() ?: throw IOException("connection is closed")
        output.write(resolved.response.header.toByteArray(Charsets.ISO_8859_1))
        if (!head && body.isNotEmpty()) output.write(body)
        output.flush()
    } catch (error: IOException) {
        status = -1
    }
    val words = pending.request.header.trim().split(Regex("\\s+"))
    val method = if (words.size >= 2) words[0] else ""
    val path = if (words.size >= 2) words[1] else ""
    println("${pending.id} $method $path ${resolved.response.status.toInt()} ${body.size}")
    System.out.flush()
    connection?.close()
    return status
}
