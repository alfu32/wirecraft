foreach ([&$routeHomeAboutResponse, &$routeHomeVersionResponse, &$routeApiUserDetailsResponse, &$routeNotFoundResponse] as &$answered) {
    while (count($answered) > 0) {
        $item = array_shift($answered);
        $item->response->header = $libhttp->http_render_header($item->response);
        $response[] = $item;
    }
}
unset($answered);
