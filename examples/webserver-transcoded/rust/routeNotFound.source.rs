const KNOWN: [&str; 3] = ["/home/about", "/home/version", "/api/user"];

while !routeNotFoundRequest.is_empty() {
    let mut item = pop::<ReqRep>(routeNotFoundRequest)
        .ok_or("routeNotFound: request buffer holds a value that is not a ReqRep")?;
    let mut reason = "Bad Request";
    let mut path = String::new();
    item.response.status = 400;
    let fields: Vec<&str> = item.request.header.split_whitespace().take(2).collect();
    if fields.len() == 2 {
        path = fields[1].chars().take(255).collect();
        if let Some(query) = path.find('?') {
            path.truncate(query);
        }
        path = path.chars().take(200).collect();
        item.response.status = 404;
        reason = "Not Found";
        if KNOWN.contains(&path.as_str()) {
            item.response.status = 405;
            reason = "Method Not Allowed";
        }
    }
    item.response.content_type = "text/plain; charset=utf-8".into();
    item.response.body = if item.response.status == 400 {
        format!("400 {reason}\n")
    } else {
        format!("{} {reason}: {path}\n", item.response.status)
    };
    push(routeNotFoundResponse, item);
}
