const TEXT: &str = concat!(
    r#"{"id":1,"name":"Ada Lovelace","email":"ada@example.invalid","roles":["admin","author"]}"#,
    "\n"
);

while !routeApiUserDetailsRequest.is_empty() {
    let mut item = pop::<ReqRep>(routeApiUserDetailsRequest)
        .ok_or("routeApiUserDetails: request buffer holds a value that is not a ReqRep")?;
    item.response.status = 200;
    item.response.content_type = "application/json".into();
    item.response.body = TEXT.into();
    push(routeApiUserDetailsResponse, item);
}
