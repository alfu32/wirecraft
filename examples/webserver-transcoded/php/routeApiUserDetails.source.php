$text = json_encode(['id' => 1, 'name' => 'Ada Lovelace', 'email' => 'ada@example.invalid', 'roles' => ['admin', 'author']]) . "\n";

while (count($routeApiUserDetailsRequest) > 0) {
    $item = array_shift($routeApiUserDetailsRequest);
    $item->response->status = 200;
    $item->response->content_type = 'application/json';
    $item->response->body = $text;
    $routeApiUserDetailsResponse[] = $item;
}
