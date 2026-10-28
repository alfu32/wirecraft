const text = "Wirecraft WebServer example: a Go dataflow HTTP server generated from a Wirecraft model.\n"
var item ReqRep

for len(*routeHomeAboutRequest) > 0 {
	if !pop(routeHomeAboutRequest, &item) {
		return errors.New("routeHomeAbout: request buffer holds a value that is not a ReqRep")
	}
	item.response.status = 200
	item.response.content_type = "text/plain; charset=utf-8"
	item.response.body = text
	push(routeHomeAboutResponse, item)
}
