const TEXT: &str = "WebServer 0.1.0 (Wirecraft Rust)\n";

while !routeHomeVersionRequest.is_empty() {
    let mut item = pop::<ReqRep>(routeHomeVersionRequest)
        .ok_or("routeHomeVersion: request buffer holds a value that is not a ReqRep")?;
    item.response.status = 200;
    item.response.content_type = "text/plain; charset=utf-8".into();
    item.response.body = TEXT.into();
    push(routeHomeVersionResponse, item);
}
