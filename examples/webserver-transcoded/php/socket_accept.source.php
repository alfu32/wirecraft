// Counters that a C body keeps in statics live in the run context.
$context['socket_accept'] ??= [
    'sequence' => 0,
    'resolved' => 0,
    'limit' => (int) getenv('WEBSERVER_MAX_REQUESTS'),
];
$state = &$context['socket_accept'];
// Both buffers of the awaitResponseLoop self-link hold the pending copies.
$pending = [&$awaitResponseLoop_in, &$awaitResponseLoop_out];

// 1. Resolve every response against its pending copy, rotating each loop buffer once.
while (count($response) > 0) {
    $item = array_shift($response);
    $found = false;
    foreach ($pending as &$side) {
        for ($remaining = count($side); $remaining > 0; $remaining--) {
            $waiting = array_shift($side);
            if ($found || $waiting->id !== $item->id) {
                $side[] = $waiting;
                continue;
            }
            $found = true;
            if ($libhttp->http_resolve($waiting, $item) !== 0) {
                fwrite(STDERR, "webserver: client closed the connection early\n");
            }
        }
    }
    unset($side);
    if (!$found) {
        throw new RuntimeException("webserver: response {$item->id} has no pending request");
    }
    $state['resolved']++;
    if ($state['limit'] > 0 && $state['resolved'] >= $state['limit']) {
        threadwork_runner()->shutdownRequest();
    }
}
// 2. Accept at most one connection; wait briefly only when nothing is in flight.
if (threadwork_runner()->isRunning()) {
    $idle = count($awaitResponseLoop_in) === 0 && count($awaitResponseLoop_out) === 0;
    if ($libhttp->http_wait($idle ? 25 : 0)) {
        [$accepted, $status] = $libhttp->http_accept(++$state['sequence']);
        if ($status < 0) {
            throw new RuntimeException('webserver: accepting a connection failed');
        }
        if ($accepted !== null) {
            // Packets are copied by value, as in C: each path gets its own request.
            $request[] = unserialize(serialize($accepted));
            $awaitResponseLoop_out[] = $accepted;
        }
    }
} elseif (count($awaitResponseLoop_in) === 0 && count($awaitResponseLoop_out) === 0) {
    $libhttp->http_close();
}
