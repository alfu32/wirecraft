const portText = process.env.WEBSERVER_PORT;
// Like atoi, text that is not a number yields port 0, which fails to listen.
const port = portText !== undefined ? parseInt(portText, 10) || 0 : 8080;
const anyAddress = process.env.WEBSERVER_BIND_ANY === "1";

if (libhttp.http_listen(port, anyAddress) !== 0) {
  console.error(`webserver: cannot listen on port ${port}`);
  throw new Error(`webserver: cannot listen on port ${port}`);
}
console.log(`webserver: listening on http://${anyAddress ? "0.0.0.0" : "127.0.0.1"}:${port}`);
