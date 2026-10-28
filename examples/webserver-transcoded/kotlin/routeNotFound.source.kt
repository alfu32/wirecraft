val known = listOf("/home/about", "/home/version", "/api/user")

while (routeNotFoundRequest.isNotEmpty()) {
    val item = routeNotFoundRequest.removeFirst()
    var status = 400
    var reason = "Bad Request"
    var path = ""
    val words = item.request.header.trim().split(Regex("\\s+"))
    if (words.size >= 2) {
        path = words[1].take(255).substringBefore('?').take(200)
        status = 404
        reason = "Not Found"
        if (path in known) {
            status = 405
            reason = "Method Not Allowed"
        }
    }
    val body = if (status == 400) "400 $reason\n" else "$status $reason: $path\n"
    val answer = item.response.copy(status = status.toDouble(), content_type = "text/plain; charset=utf-8", body = body)
    routeNotFoundResponse.addLast(item.copy(response = answer))
}
