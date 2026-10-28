const text = "Wirecraft WebServer example: a Node.js dataflow HTTP server generated from a Wirecraft model.\n";

while (routeHomeAboutRequest.length > 0) {
  const item = routeHomeAboutRequest.shift();
  item.response = { ...item.response, status: 200, content_type: "text/plain; charset=utf-8", body: text };
  routeHomeAboutResponse.push(item);
}
