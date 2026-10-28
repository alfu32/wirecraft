// Counters that a C body keeps in statics live in the run context.
sequence, _ := context.Imports["socket_accept.sequence"].(uint64)
resolvedTotal, _ := context.Imports["socket_accept.resolved"].(uint64)
limit, known := context.Imports["socket_accept.limit"].(int64)
if !known {
	limit, _ = strconv.ParseInt(os.Getenv("WEBSERVER_MAX_REQUESTS"), 10, 64)
	context.Imports["socket_accept.limit"] = limit
}
// Both buffers of the awaitResponseLoop self-link hold the pending copies.
pending := [2]*ThreadworkBuffer{awaitResponseLoop_in, awaitResponseLoop_out}
var item ReqRep

// 1. Resolve every response against its pending copy, rotating each loop buffer once.
for len(*response) > 0 {
	found := false
	if !pop(response, &item) {
		return errors.New("webserver: response buffer holds a value that is not a ReqRep")
	}
	for _, side := range pending {
		for remaining := len(*side); remaining > 0; remaining-- {
			var waiting ReqRep
			if !pop(side, &waiting) {
				return errors.New("webserver: pending buffer holds a value that is not a ReqRep")
			}
			if found || waiting.id != item.id {
				push(side, waiting)
				continue
			}
			found = true
			if libhttp.http_resolve(waiting, item) != 0 {
				fmt.Fprintln(os.Stderr, "webserver: client closed the connection early")
			}
		}
	}
	if !found {
		return fmt.Errorf("webserver: response %s has no pending request", item.id)
	}
	resolvedTotal++
	if limit > 0 && resolvedTotal >= uint64(limit) {
		threadworkRunner.ShutdownRequest()
	}
}
// 2. Accept at most one connection; wait briefly only when nothing is in flight.
if threadworkRunner.IsRunning() {
	timeout := 0
	if len(*awaitResponseLoop_in) == 0 && len(*awaitResponseLoop_out) == 0 {
		timeout = 25
	}
	if libhttp.http_wait(timeout) {
		sequence++
		accepted, status := libhttp.http_accept(sequence)
		if status < 0 {
			return errors.New("webserver: accepting a connection failed")
		}
		if status > 0 {
			push(request, accepted)
			push(awaitResponseLoop_out, accepted)
		}
	}
} else if len(*awaitResponseLoop_in) == 0 && len(*awaitResponseLoop_out) == 0 {
	libhttp.http_close()
}
context.Imports["socket_accept.sequence"] = sequence
context.Imports["socket_accept.resolved"] = resolvedTotal
