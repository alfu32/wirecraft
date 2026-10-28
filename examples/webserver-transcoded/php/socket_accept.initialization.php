$portText = getenv('WEBSERVER_PORT');
// Like atoi, text that is not a number yields port 0, which fails to listen.
$port = $portText !== false ? (int) $portText : 8080;
$anyAddress = getenv('WEBSERVER_BIND_ANY') === '1';

if ($libhttp->http_listen($port, $anyAddress) !== 0) {
    fwrite(STDERR, "webserver: cannot listen on port $port\n");
    throw new RuntimeException("webserver: cannot listen on port $port");
}
printf("webserver: listening on http://%s:%d\n", $anyAddress ? '0.0.0.0' : '127.0.0.1', $port);
