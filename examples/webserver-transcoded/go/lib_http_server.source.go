// HTTP/1.1 helpers for the WebServer network. Lent to consumers as `libhttp`.
const httpMaxHeader = 8192
const httpMaxBody = 65536

// Listening socket shared by the http_* functions; nil when closed.
var httpListener *net.TCPListener

// A connection that http_wait accepted, handed to the next http_accept.
var httpWaiting net.Conn

// Open connections, by the number a ReqRep carries in `connection`.
var httpConnections = map[int]net.Conn{}
var httpNextConnection = 0

func http_status_text(status int) string {
	switch status {
	case 200:
		return "OK"
	case 400:
		return "Bad Request"
	case 404:
		return "Not Found"
	case 405:
		return "Method Not Allowed"
	case 408:
		return "Request Timeout"
	case 413:
		return "Content Too Large"
	case 431:
		return "Request Header Fields Too Large"
	case 500:
		return "Internal Server Error"
	default:
		return "Unknown"
	}
}

// Non-negative integer value of a header field (case-insensitive name), or -1.
func http_header_number(header string, name string) int {
	lines := strings.Split(header, "\r\n")
	for _, line := range lines[1:] {
		if line == "" {
			break
		}
		colon := strings.IndexByte(line, ':')
		if colon != len(name) || !strings.EqualFold(line[:colon], name) {
			continue
		}
		value := strings.TrimLeft(line[colon+1:], " \t")
		digits := 0
		for digits < len(value) && value[digits] >= '0' && value[digits] <= '9' {
			digits++
		}
		number, err := strconv.Atoi(value[:digits])
		if digits == 0 || err != nil {
			return -1
		}
		return number
	}
	return -1
}

// Answers a connection that never became a request, then closes it.
func http_reject(connection net.Conn, status int) {
	text := http_status_text(status)
	reply := fmt.Sprintf("HTTP/1.1 %d %s\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: %d\r\n"+
		"Connection: close\r\nServer: Wirecraft-WebServer/0.1\r\n\r\n%d %s\n",
		status, text, len(text)+5, status, text)
	_, _ = connection.Write([]byte(reply))
	// Closing with unread request bytes would reset the connection and lose the reply:
	// finish sending, then discard what the client still sends (bounded by the 2 s deadline).
	if tcp, ok := connection.(*net.TCPConn); ok && tcp.CloseWrite() == nil {
		discard := make([]byte, 4096)
		for drained := 0; drained < httpMaxBody; {
			count, err := connection.Read(discard)
			drained += count
			if err != nil {
				break
			}
		}
	}
	_ = connection.Close()
}

func http_listen(port int, anyAddress bool) error {
	if port <= 0 || port > 65535 {
		return errors.New("port out of range")
	}
	host := "127.0.0.1"
	if anyAddress {
		host = "0.0.0.0"
	}
	address, err := net.ResolveTCPAddr("tcp4", net.JoinHostPort(host, strconv.Itoa(port)))
	if err != nil {
		return err
	}
	httpListener, err = net.ListenTCP("tcp4", address)
	return err
}

// Waits up to timeoutMs for a connection; true when http_accept has one to read.
func http_wait(timeoutMs int) bool {
	if httpListener == nil {
		return false
	}
	if httpWaiting != nil {
		return true
	}
	// A deadline already in the past fails before looking at the queue; wait at least 1 ms.
	_ = httpListener.SetDeadline(time.Now().Add(time.Duration(max(timeoutMs, 1)) * time.Millisecond))
	connection, err := httpListener.Accept()
	if err != nil {
		return false
	}
	httpWaiting = connection
	return true
}

func http_close() {
	if httpWaiting != nil {
		_ = httpWaiting.Close()
		httpWaiting = nil
	}
	if httpListener != nil {
		_ = httpListener.Close()
		httpListener = nil
	}
}

// Reads one request from the waiting connection: 1 and the request, 0 when the
// connection was answered here instead (408, 400, 413, 431), -1 on failure.
func http_accept(sequence uint64) (ReqRep, int) {
	connection := httpWaiting
	httpWaiting = nil
	if connection == nil {
		return ReqRep{}, 0
	}
	_ = connection.SetReadDeadline(time.Now().Add(2 * time.Second))
	buffer := make([]byte, 0, httpMaxHeader)
	chunk := make([]byte, 4096)
	end := -1
	for end < 0 {
		if len(buffer) >= httpMaxHeader {
			http_reject(connection, 431)
			return ReqRep{}, 0
		}
		count, err := connection.Read(chunk[:min(len(chunk), httpMaxHeader-len(buffer))])
		buffer = append(buffer, chunk[:count]...)
		if end = bytes.Index(buffer, []byte("\r\n\r\n")); end >= 0 {
			break
		}
		if err != nil {
			status := 400
			var timeout net.Error
			if len(buffer) == 0 && errors.As(err, &timeout) && timeout.Timeout() {
				status = 408
			}
			http_reject(connection, status)
			return ReqRep{}, 0
		}
	}
	headerLength := end + 4
	header := string(buffer[:headerLength])
	bodyLength := max(http_header_number(header, "Content-Length"), 0)
	if bodyLength > httpMaxBody {
		http_reject(connection, 413)
		return ReqRep{}, 0
	}
	for len(buffer) < headerLength+bodyLength {
		count, err := connection.Read(chunk[:min(len(chunk), headerLength+bodyLength-len(buffer))])
		buffer = append(buffer, chunk[:count]...)
		if err != nil && len(buffer) < headerLength+bodyLength {
			http_reject(connection, 400)
			return ReqRep{}, 0
		}
	}
	httpNextConnection++
	httpConnections[httpNextConnection] = connection
	return ReqRep{
		id:         fmt.Sprintf("req-%d", sequence),
		connection: float64(httpNextConnection),
		request:    Request{header: header, body: string(buffer[headerLength : headerLength+bodyLength])},
	}, 1
}

func http_render_header(response Response) string {
	contentType := response.content_type
	if contentType == "" {
		contentType = "text/plain; charset=utf-8"
	}
	allow := ""
	if response.status == 405 {
		allow = "Allow: GET, HEAD\r\n"
	}
	return fmt.Sprintf("HTTP/1.1 %d %s\r\nContent-Type: %s\r\nContent-Length: %d\r\nConnection: close\r\n"+
		"Server: Wirecraft-WebServer/0.1\r\n%s\r\n",
		int(response.status), http_status_text(int(response.status)), contentType, len(response.body), allow)
}

// Writes the response to the pending request's connection, logs it and closes it; 0 on success.
func http_resolve(pending ReqRep, resolved ReqRep) int {
	connection := httpConnections[int(pending.connection)]
	delete(httpConnections, int(pending.connection))
	status := 0
	head := strings.HasPrefix(pending.request.header, "HEAD ")
	if connection == nil {
		status = -1
	} else if _, err := connection.Write([]byte(resolved.response.header)); err != nil {
		status = -1
	} else if !head && resolved.response.body != "" {
		if _, err := connection.Write([]byte(resolved.response.body)); err != nil {
			status = -1
		}
	}
	method, path := "", ""
	if fields := strings.Fields(pending.request.header); len(fields) >= 2 {
		method, path = fields[0], fields[1]
	}
	fmt.Printf("%s %s %s %d %d\n", pending.id, method, path, int(resolved.response.status), len(resolved.response.body))
	if connection != nil {
		_ = connection.Close()
	}
	return status
}
