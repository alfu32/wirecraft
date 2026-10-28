$text = "Wirecraft WebServer example: a PHP dataflow HTTP server generated from a Wirecraft model.\n";

while (count($routeHomeAboutRequest) > 0) {
    $item = array_shift($routeHomeAboutRequest);
    $item->response->status = 200;
    $item->response->content_type = 'text/plain; charset=utf-8';
    $item->response->body = $text;
    $routeHomeAboutResponse[] = $item;
}
