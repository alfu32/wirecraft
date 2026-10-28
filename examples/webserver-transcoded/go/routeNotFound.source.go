known := []string{"/home/about", "/home/version", "/api/user"}
var item ReqRep

for len(*routeNotFoundRequest) > 0 {
	reason := "Bad Request"
	path := ""
	if !pop(routeNotFoundRequest, &item) {
		return errors.New("routeNotFound: request buffer holds a value that is not a ReqRep")
	}
	item.response.status = 400
	if fields := strings.Fields(item.request.header); len(fields) >= 2 {
		path = fields[1][:min(len(fields[1]), 255)]
		if query := strings.IndexByte(path, '?'); query >= 0 {
			path = path[:query]
		}
		path = path[:min(len(path), 200)]
		item.response.status = 404
		reason = "Not Found"
		for _, route := range known {
			if path == route {
				item.response.status = 405
				reason = "Method Not Allowed"
			}
		}
	}
	item.response.content_type = "text/plain; charset=utf-8"
	if item.response.status == 400 {
		item.response.body = fmt.Sprintf("400 %s\n", reason)
	} else {
		item.response.body = fmt.Sprintf("%d %s: %s\n", int(item.response.status), reason, path)
	}
	push(routeNotFoundResponse, item)
}
