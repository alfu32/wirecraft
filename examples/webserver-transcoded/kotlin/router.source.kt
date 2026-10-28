while (request.isNotEmpty()) {
    val item = request.removeFirst()
    var route = routeNotFoundRequest
    // The request line's first two words, as sscanf("%7s %255s") reads them.
    val words = item.request.header.trim().split(Regex("\\s+"))
    if (words.size >= 2 && (words[0] == "GET" || words[0] == "HEAD")) {
        route = when (words[1].take(255).substringBefore('?')) {
            "/home/about" -> routeHomeAboutRequest
            "/home/version" -> routeHomeVersionRequest
            "/api/user" -> routeApiUserDetailsRequest
            else -> routeNotFoundRequest
        }
    }
    route.addLast(item)
}
