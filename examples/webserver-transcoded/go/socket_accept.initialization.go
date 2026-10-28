port := 8080
if text := os.Getenv("WEBSERVER_PORT"); text != "" {
	port, _ = strconv.Atoi(text)
}
anyAddress := os.Getenv("WEBSERVER_BIND_ANY") == "1"
if err := libhttp.http_listen(port, anyAddress); err != nil {
	fmt.Fprintf(os.Stderr, "webserver: cannot listen on port %d\n", port)
	return err
}
host := "127.0.0.1"
if anyAddress {
	host = "0.0.0.0"
}
fmt.Printf("webserver: listening on http://%s:%d\n", host, port)
