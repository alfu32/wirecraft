// Counters that a C body keeps in statics live in the run context.
@Suppress("UNCHECKED_CAST")
val state = context.imports.getOrPut("socket_accept") {
    mutableMapOf(
        "sequence" to 0L,
        "resolved" to 0L,
        "limit" to (System.getenv("WEBSERVER_MAX_REQUESTS")?.trim()?.toLongOrNull() ?: 0L),
    )
} as MutableMap<String, Long>
// Both buffers of the awaitResponseLoop self-link hold the pending copies.
val pending = arrayOf(awaitResponseLoop_in, awaitResponseLoop_out)

// 1. Resolve every response against its pending copy, rotating each loop buffer once.
while (response.isNotEmpty()) {
    val item = response.removeFirst()
    var found = false
    for (side in pending) {
        repeat(side.size) {
            val waiting = side.removeFirst()
            if (found || waiting.id != item.id) {
                side.addLast(waiting)
            } else {
                found = true
                if (libhttp.http_resolve(waiting, item) != 0) {
                    System.err.println("webserver: client closed the connection early")
                }
            }
        }
    }
    if (!found) throw IllegalStateException("webserver: response ${item.id} has no pending request")
    val resolved = state.getValue("resolved") + 1
    state["resolved"] = resolved
    val limit = state.getValue("limit")
    if (limit > 0 && resolved >= limit) threadworkRunner.shutdownRequest()
}
// 2. Accept at most one connection; wait briefly only when nothing is in flight.
if (threadworkRunner.isRunning()) {
    val idle = awaitResponseLoop_in.isEmpty() && awaitResponseLoop_out.isEmpty()
    if (libhttp.http_wait(if (idle) 25 else 0)) {
        val sequence = state.getValue("sequence") + 1
        state["sequence"] = sequence
        val (accepted, status) = libhttp.http_accept(sequence)
        if (status < 0) throw IllegalStateException("webserver: accepting a connection failed")
        if (accepted != null) {
            request.addLast(accepted)
            awaitResponseLoop_out.addLast(accepted)
        }
    }
} else if (awaitResponseLoop_in.isEmpty() && awaitResponseLoop_out.isEmpty()) {
    libhttp.http_close()
}
