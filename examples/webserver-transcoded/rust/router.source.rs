while !request.is_empty() {
    let item = pop::<ReqRep>(request)
        .ok_or("router: request buffer holds a value that is not a ReqRep")?;
    let mut route = routeNotFoundRequest;
    // The request line's first two words, as sscanf("%7s %255s") reads them.
    let fields: Vec<&str> = item.request.header.split_whitespace().take(2).collect();
    if fields.len() == 2 && (fields[0] == "GET" || fields[0] == "HEAD") {
        let path: String = fields[1].chars().take(255).collect();
        route = match path.split('?').next().unwrap_or_default() {
            "/home/about" => routeHomeAboutRequest,
            "/home/version" => routeHomeVersionRequest,
            "/api/user" => routeApiUserDetailsRequest,
            _ => routeNotFoundRequest,
        };
    }
    push(route, item);
}
