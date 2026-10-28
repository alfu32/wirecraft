const TEXT: &str = "Wirecraft WebServer example: a Rust dataflow HTTP server generated from a Wirecraft model.\n";

while !routeHomeAboutRequest.is_empty() {
    let mut item = pop::<ReqRep>(routeHomeAboutRequest)
        .ok_or("routeHomeAbout: request buffer holds a value that is not a ReqRep")?;
    item.response.status = 200;
    item.response.content_type = "text/plain; charset=utf-8".into();
    item.response.body = TEXT.into();
    push(routeHomeAboutResponse, item);
}
