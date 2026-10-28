const text = JSON.stringify({ id: 1, name: "Ada Lovelace", email: "ada@example.invalid", roles: ["admin", "author"] }) + "\n";

while (routeApiUserDetailsRequest.length > 0) {
  const item = routeApiUserDetailsRequest.shift();
  item.response = { ...item.response, status: 200, content_type: "application/json", body: text };
  routeApiUserDetailsResponse.push(item);
}
