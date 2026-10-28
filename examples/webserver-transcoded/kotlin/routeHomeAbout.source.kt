val text = "Wirecraft WebServer example: a Kotlin dataflow HTTP server generated from a Wirecraft model.\n"

while (routeHomeAboutRequest.isNotEmpty()) {
    val item = routeHomeAboutRequest.removeFirst()
    val answer = item.response.copy(status = 200.0, content_type = "text/plain; charset=utf-8", body = text)
    routeHomeAboutResponse.addLast(item.copy(response = answer))
}
