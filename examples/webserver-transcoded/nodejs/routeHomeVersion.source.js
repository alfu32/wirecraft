const text = "WebServer 0.1.0 (Wirecraft Node.js)\n";

while (routeHomeVersionRequest.length > 0) {
  const item = routeHomeVersionRequest.shift();
  item.response = { ...item.response, status: 200, content_type: "text/plain; charset=utf-8", body: text };
  routeHomeVersionResponse.push(item);
}
