while (count($request) > 0) {
    $item = array_shift($request);
    $route = &$routeNotFoundRequest;
    // The request line's first two words, as sscanf("%7s %255s") reads them.
    $words = preg_split('/\s+/', trim($item->request->header));
    if (count($words) >= 2 && ($words[0] === 'GET' || $words[0] === 'HEAD')) {
        switch (explode('?', substr($words[1], 0, 255))[0]) {
            case '/home/about':
                $route = &$routeHomeAboutRequest;
                break;
            case '/home/version':
                $route = &$routeHomeVersionRequest;
                break;
            case '/api/user':
                $route = &$routeApiUserDetailsRequest;
                break;
        }
    }
    $route[] = $item;
    unset($route);
}
