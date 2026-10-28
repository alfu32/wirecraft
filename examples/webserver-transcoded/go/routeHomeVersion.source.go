const text = "WebServer 0.1.0 (Wirecraft Go)\n"
var item ReqRep

for len(*routeHomeVersionRequest) > 0 {
	if !pop(routeHomeVersionRequest, &item) {
		return errors.New("routeHomeVersion: request buffer holds a value that is not a ReqRep")
	}
	item.response.status = 200
	item.response.content_type = "text/plain; charset=utf-8"
	item.response.body = text
	push(routeHomeVersionResponse, item)
}
