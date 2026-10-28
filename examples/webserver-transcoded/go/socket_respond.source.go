answered := []*ThreadworkBuffer{routeHomeAboutResponse, routeHomeVersionResponse, routeApiUserDetailsResponse, routeNotFoundResponse}
var item ReqRep

for _, buffer := range answered {
	for len(*buffer) > 0 {
		if !pop(buffer, &item) {
			return errors.New("socket_respond: response buffer holds a value that is not a ReqRep")
		}
		item.response.header = libhttp.http_render_header(item.response)
		push(response, item)
	}
}
