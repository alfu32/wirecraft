const text = `{"id":1,"name":"Ada Lovelace","email":"ada@example.invalid","roles":["admin","author"]}` + "\n"
var item ReqRep

for len(*routeApiUserDetailsRequest) > 0 {
	if !pop(routeApiUserDetailsRequest, &item) {
		return errors.New("routeApiUserDetails: request buffer holds a value that is not a ReqRep")
	}
	item.response.status = 200
	item.response.content_type = "application/json"
	item.response.body = text
	push(routeApiUserDetailsResponse, item)
}
