$known = ['/home/about', '/home/version', '/api/user'];

while (count($routeNotFoundRequest) > 0) {
    $item = array_shift($routeNotFoundRequest);
    $status = 400;
    $reason = 'Bad Request';
    $path = '';
    $words = preg_split('/\s+/', trim($item->request->header));
    if (count($words) >= 2) {
        $path = substr(explode('?', substr($words[1], 0, 255))[0], 0, 200);
        $status = 404;
        $reason = 'Not Found';
        if (in_array($path, $known, true)) {
            $status = 405;
            $reason = 'Method Not Allowed';
        }
    }
    $item->response->status = $status;
    $item->response->content_type = 'text/plain; charset=utf-8';
    $item->response->body = $status === 400 ? "400 $reason\n" : "$status $reason: $path\n";
    $routeNotFoundResponse[] = $item;
}
