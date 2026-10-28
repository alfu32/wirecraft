let answered = [
    routeHomeAboutResponse,
    routeHomeVersionResponse,
    routeApiUserDetailsResponse,
    routeNotFoundResponse,
];

for buffer in answered {
    while !buffer.is_empty() {
        let mut item = pop::<ReqRep>(buffer)
            .ok_or("socket_respond: response buffer holds a value that is not a ReqRep")?;
        item.response.header = libhttp.http_render_header(&item.response);
        push(response, item);
    }
}
