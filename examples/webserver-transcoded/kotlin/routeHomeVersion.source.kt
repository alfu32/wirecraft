val text = "WebServer 0.1.0 (Wirecraft Kotlin)\n"

while (routeHomeVersionRequest.isNotEmpty()) {
    val item = routeHomeVersionRequest.removeFirst()
    val answer = item.response.copy(status = 200.0, content_type = "text/plain; charset=utf-8", body = text)
    routeHomeVersionResponse.addLast(item.copy(response = answer))
}
