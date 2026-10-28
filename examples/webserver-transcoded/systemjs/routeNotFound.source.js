const known = ["/home/about", "/home/version", "/api/user"];

while (routeNotFoundRequest.length > 0) {
  const item = routeNotFoundRequest.shift();
  let status = 400;
  let reason = "Bad Request";
  let path = "";
  const [, target] = item.request.header.trim().split(/\s+/);
  if (target !== undefined) {
    path = target.slice(0, 255).split("?")[0].slice(0, 200);
    status = 404;
    reason = "Not Found";
    if (known.includes(path)) {
      status = 405;
      reason = "Method Not Allowed";
    }
  }
  const body = status === 400 ? `400 ${reason}\n` : `${status} ${reason}: ${path}\n`;
  item.response = { ...item.response, status, content_type: "text/plain; charset=utf-8", body };
  routeNotFoundResponse.push(item);
}
