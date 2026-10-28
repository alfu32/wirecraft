for (const answered of [routeHomeAboutResponse, routeHomeVersionResponse, routeApiUserDetailsResponse, routeNotFoundResponse]) {
  while (answered.length > 0) {
    const item = answered.shift();
    item.response.header = libhttp.http_render_header(item.response);
    response.push(item);
  }
}
