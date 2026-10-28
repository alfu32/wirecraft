while (request.length > 0) {
  const item = request.shift();
  let route = routeNotFoundRequest;
  // The request line's first two words, as sscanf("%7s %255s") reads them.
  const [method, target] = item.request.header.trim().split(/\s+/);
  if (target !== undefined && (method === "GET" || method === "HEAD")) {
    switch (target.slice(0, 255).split("?")[0]) {
      case "/home/about": route = routeHomeAboutRequest; break;
      case "/home/version": route = routeHomeVersionRequest; break;
      case "/api/user": route = routeApiUserDetailsRequest; break;
    }
  }
  route.push(item);
}
