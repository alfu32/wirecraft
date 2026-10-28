var item ReqRep

for len(*request) > 0 {
	route := routeNotFoundRequest
	if !pop(request, &item) {
		return errors.New("router: request buffer holds a value that is not a ReqRep")
	}
	// The request line's first two words, as sscanf("%7s %255s") reads them.
	if fields := strings.Fields(item.request.header); len(fields) >= 2 && (fields[0] == "GET" || fields[0] == "HEAD") {
		path := fields[1][:min(len(fields[1]), 255)]
		if query := strings.IndexByte(path, '?'); query >= 0 {
			path = path[:query]
		}
		switch path {
		case "/home/about":
			route = routeHomeAboutRequest
		case "/home/version":
			route = routeHomeVersionRequest
		case "/api/user":
			route = routeApiUserDetailsRequest
		}
	}
	push(route, item)
}
