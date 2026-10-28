// HTTP/1.1 helpers for the WebServer network. Lent to consumers as `libhttp`.
final class HttpServerState
{
    /** @var resource|null Listening socket shared by the http_* functions; null when closed. */
    public static $listener = null;
    /** @var array<int, resource> Open connections, by the number a ReqRep carries in `connection`. */
    public static array $connections = [];
    public static int $nextConnection = 0;
}

function http_status_text(int $status): string
{
    return match ($status) {
        200 => 'OK',
        400 => 'Bad Request',
        404 => 'Not Found',
        405 => 'Method Not Allowed',
        408 => 'Request Timeout',
        413 => 'Content Too Large',
        431 => 'Request Header Fields Too Large',
        500 => 'Internal Server Error',
        default => 'Unknown',
    };
}

// Non-negative integer value of a header field (case-insensitive name), or -1.
function http_header_number(string $header, string $name): int
{
    foreach (array_slice(explode("\r\n", $header), 1) as $line) {
        if ($line === '') {
            break;
        }
        $colon = strpos($line, ':');
        if ($colon !== strlen($name) || strcasecmp(substr($line, 0, $colon), $name) !== 0) {
            continue;
        }
        // Like strtol, an out-of-range value saturates instead of failing.
        return preg_match('/^[ \t]*([0-9]+)/', substr($line, $colon + 1), $digits) === 1
            ? (int) min((float) $digits[1], PHP_INT_MAX)
            : -1;
    }
    return -1;
}

// Writes every byte; false when the connection failed.
function http_write_all($connection, string $data): bool
{
    while ($data !== '') {
        $written = @fwrite($connection, $data);
        if ($written === false || $written === 0) {
            return false;
        }
        $data = substr($data, $written);
    }
    return true;
}

// Answers a connection that never became a request, then closes it.
function http_reject($connection, int $status): void
{
    $text = http_status_text($status);
    http_write_all($connection, sprintf(
        "HTTP/1.1 %d %s\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: %d\r\n"
            . "Connection: close\r\nServer: Wirecraft-WebServer/0.1\r\n\r\n%d %s\n",
        $status, $text, strlen($text) + 5, $status, $text
    ));
    // Closing with unread request bytes would reset the connection and lose the reply:
    // finish sending, then discard what the client still sends (bounded by the 2 s timeout).
    if (stream_socket_shutdown($connection, STREAM_SHUT_WR)) {
        for ($drained = 0; $drained < HTTP_MAX_BODY;) {
            $chunk = fread($connection, 4096);
            if ($chunk === false || $chunk === '') {
                break;
            }
            $drained += strlen($chunk);
        }
    }
    fclose($connection);
}

function http_listen(int $port, bool $anyAddress): int
{
    if ($port <= 0 || $port > 65535) {
        return -1;
    }
    $host = $anyAddress ? '0.0.0.0' : '127.0.0.1';
    $context = stream_context_create(['socket' => ['backlog' => 64]]);
    $listener = @stream_socket_server("tcp://$host:$port", $errno, $error,
        STREAM_SERVER_BIND | STREAM_SERVER_LISTEN, $context);
    if ($listener === false) {
        return -1;
    }
    HttpServerState::$listener = $listener;
    return 0;
}

// True when a connection is waiting, after at most timeoutMs.
function http_wait(int $timeoutMs): bool
{
    if (HttpServerState::$listener === null) {
        return false;
    }
    $read = [HttpServerState::$listener];
    $write = $except = null;
    return @stream_select($read, $write, $except, 0, $timeoutMs * 1000) > 0;
}

function http_close(): void
{
    if (HttpServerState::$listener !== null) {
        fclose(HttpServerState::$listener);
        HttpServerState::$listener = null;
    }
}

// Reads one request: the request and 1, or null and 0 when the connection was answered
// here instead (408, 400, 413, 431) or there was none after all.
function http_accept(int $sequence): array
{
    $connection = @stream_socket_accept(HttpServerState::$listener, 0);
    if ($connection === false) {
        return [null, 0];
    }
    stream_set_timeout($connection, 2);
    $data = '';
    $end = false;
    while ($end === false) {
        if (strlen($data) >= HTTP_MAX_HEADER) {
            http_reject($connection, 431);
            return [null, 0];
        }
        $chunk = fread($connection, min(4096, HTTP_MAX_HEADER - strlen($data)));
        if ($chunk === false || $chunk === '') {
            $timedOut = stream_get_meta_data($connection)['timed_out'];
            http_reject($connection, $data === '' && $timedOut ? 408 : 400);
            return [null, 0];
        }
        $data .= $chunk;
        $end = strpos($data, "\r\n\r\n");
    }
    $headerLength = $end + 4;
    $header = substr($data, 0, $headerLength);
    $bodyLength = max(http_header_number($header, 'Content-Length'), 0);
    if ($bodyLength > HTTP_MAX_BODY) {
        http_reject($connection, 413);
        return [null, 0];
    }
    while (strlen($data) < $headerLength + $bodyLength) {
        $chunk = fread($connection, min(4096, $headerLength + $bodyLength - strlen($data)));
        if ($chunk === false || $chunk === '') {
            http_reject($connection, 400);
            return [null, 0];
        }
        $data .= $chunk;
    }
    $number = ++HttpServerState::$nextConnection;
    HttpServerState::$connections[$number] = $connection;
    return [
        new ReqRep(
            "req-$sequence",
            $number,
            new Request($header, substr($data, $headerLength, $bodyLength)),
            new Response(0, '', '', ''),
        ),
        1,
    ];
}

function http_render_header(Response $response): string
{
    $status = (int) $response->status;
    return sprintf(
        "HTTP/1.1 %d %s\r\nContent-Type: %s\r\nContent-Length: %d\r\nConnection: close\r\n"
            . "Server: Wirecraft-WebServer/0.1\r\n%s\r\n",
        $status,
        http_status_text($status),
        $response->content_type !== '' ? $response->content_type : 'text/plain; charset=utf-8',
        strlen($response->body),
        $status === 405 ? "Allow: GET, HEAD\r\n" : ''
    );
}

// Writes the response to the pending request's connection, logs it and closes it; 0 on success.
function http_resolve(ReqRep $pending, ReqRep $resolved): int
{
    $number = (int) $pending->connection;
    $connection = HttpServerState::$connections[$number] ?? null;
    unset(HttpServerState::$connections[$number]);
    $head = str_starts_with($pending->request->header, 'HEAD ');
    $body = $resolved->response->body;
    $status = $connection !== null
        && http_write_all($connection, $resolved->response->header)
        && ($head || $body === '' || http_write_all($connection, $body)) ? 0 : -1;
    $words = preg_split('/\s+/', trim($pending->request->header));
    printf("%s %s %s %d %d\n", $pending->id, $words[0] ?? '', $words[1] ?? '',
        (int) $resolved->response->status, strlen($body));
    if ($connection !== null) {
        fclose($connection);
    }
    return $status;
}
