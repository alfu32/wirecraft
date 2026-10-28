$text = "WebServer 0.1.0 (Wirecraft PHP)\n";

while (count($routeHomeVersionRequest) > 0) {
    $item = array_shift($routeHomeVersionRequest);
    $item->response->status = 200;
    $item->response->content_type = 'text/plain; charset=utf-8';
    $item->response->body = $text;
    $routeHomeVersionResponse[] = $item;
}
