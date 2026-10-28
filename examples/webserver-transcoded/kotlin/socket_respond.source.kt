for (answered in arrayOf(routeHomeAboutResponse, routeHomeVersionResponse, routeApiUserDetailsResponse, routeNotFoundResponse)) {
    while (answered.isNotEmpty()) {
        val item = answered.removeFirst()
        val header = libhttp.http_render_header(item.response)
        response.addLast(item.copy(response = item.response.copy(header = header)))
    }
}
