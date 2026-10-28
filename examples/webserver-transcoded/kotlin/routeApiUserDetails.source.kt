val text = """{"id":1,"name":"Ada Lovelace","email":"ada@example.invalid","roles":["admin","author"]}""" + "\n"

while (routeApiUserDetailsRequest.isNotEmpty()) {
    val item = routeApiUserDetailsRequest.removeFirst()
    val answer = item.response.copy(status = 200.0, content_type = "application/json", body = text)
    routeApiUserDetailsResponse.addLast(item.copy(response = answer))
}
