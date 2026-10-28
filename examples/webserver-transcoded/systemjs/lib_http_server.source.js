// HTTP/1.1 helpers for the WebServer network. Lent to consumers as `libhttp`.
// Node.js sockets are event driven: connections are read completely between ticks,
// and http_accept hands out the ones that became requests, in arrival order.
const HTTP_MAX_HEADER = 8192;
const HTTP_MAX_BODY = 65536;

// Listening server shared by the http_* functions; null when closed.
let httpServer = null;
// Set when the server failed after http_listen returned; http_accept reports it.
let httpFailure = null;
// Complete requests waiting for http_accept: { socket, header, body }.
const httpReady = [];
// Open connections, by the number a ReqRep carries in `connection`.
const httpConnections = new Map();
let httpNextConnection = 0;

function http_status_text(status) {
  switch (status) {
    case 200: return "OK";
    case 400: return "Bad Request";
    case 404: return "Not Found";
    case 405: return "Method Not Allowed";
    case 408: return "Request Timeout";
    case 413: return "Content Too Large";
    case 431: return "Request Header Fields Too Large";
    case 500: return "Internal Server Error";
    default: return "Unknown";
  }
}

// Non-negative integer value of a header field (case-insensitive name), or -1.
function httpHeaderNumber(header, name) {
  for (const line of header.split("\r\n").slice(1)) {
    if (line === "") break;
    const colon = line.indexOf(":");
    if (colon !== name.length || line.slice(0, colon).toLowerCase() !== name.toLowerCase()) continue;
    const digits = /^[ \t]*([0-9]+)/.exec(line.slice(colon + 1));
    return digits ? Number(digits[1]) : -1;
  }
  return -1;
}

// Answers a connection that never became a request, then closes it.
function httpReject(socket, status) {
  const text = http_status_text(status);
  // end() finishes sending, then the socket keeps discarding what the client still sends,
  // so unread request bytes cannot reset the connection and lose the reply.
  socket.removeAllListeners("data");
  socket.removeAllListeners("timeout");
  let drained = 0;
  socket.on("data", (chunk) => {
    drained += chunk.length;
    if (drained >= HTTP_MAX_BODY) socket.destroy();
  });
  socket.on("timeout", () => socket.destroy());
  socket.end(
    `HTTP/1.1 ${status} ${text}\r\nContent-Type: text/plain; charset=utf-8\r\n` +
      `Content-Length: ${text.length + 5}\r\nConnection: close\r\nServer: Wirecraft-WebServer/0.1\r\n\r\n` +
      `${status} ${text}\n`,
    "latin1",
  );
}

// Reads one connection until it is a complete request or has been answered.
function httpRead(socket) {
  let data = Buffer.alloc(0);
  let headerLength = -1;
  let total = -1;
  socket.setTimeout(2000);
  socket.on("timeout", () => httpReject(socket, data.length === 0 ? 408 : 400));
  socket.on("end", () => {
    if (total < 0 || data.length < total) httpReject(socket, 400);
  });
  socket.on("error", () => socket.destroy());
  socket.on("data", (chunk) => {
    data = Buffer.concat([data, chunk]);
    if (headerLength < 0) {
      const end = data.indexOf("\r\n\r\n", 0, "latin1");
      if (end < 0 || end + 4 > HTTP_MAX_HEADER) {
        if (data.length >= HTTP_MAX_HEADER) httpReject(socket, 431);
        return;
      }
      headerLength = end + 4;
      const bodyLength = Math.max(httpHeaderNumber(data.toString("latin1", 0, headerLength), "Content-Length"), 0);
      if (bodyLength > HTTP_MAX_BODY) {
        httpReject(socket, 413);
        return;
      }
      total = headerLength + bodyLength;
    }
    if (data.length >= total) {
      socket.removeAllListeners("data");
      socket.removeAllListeners("timeout");
      socket.removeAllListeners("end");
      socket.setTimeout(0);
      socket.pause();
      httpReady.push({
        socket,
        header: data.toString("latin1", 0, headerLength),
        body: data.toString("latin1", headerLength, total),
      });
      // The runner may be waiting out an idle tick; the next one has work.
      threadworkRunner.wake();
    }
  });
}

function http_listen(port, anyAddress) {
  if (!Number.isInteger(port) || port <= 0 || port > 65535) return -1;
  httpServer = net.createServer(httpRead);
  // Binding completes on the event loop; a failure surfaces through http_accept.
  httpServer.on("error", (error) => {
    httpFailure = error;
  });
  httpServer.listen({ port, host: anyAddress ? "0.0.0.0" : "127.0.0.1", backlog: 64 });
  return 0;
}

// True when http_accept has a request. Node.js cannot block here: the runner's idle
// ticks wait on the event loop instead, and a completed request wakes them.
function http_wait(timeoutMs) {
  return httpFailure !== null || httpReady.length > 0;
}

function http_close() {
  for (const waiting of httpReady.splice(0)) waiting.socket.destroy();
  if (httpServer !== null) httpServer.close();
  httpServer = null;
}

// The next complete request and 1, null and 0 when there is none, or null and -1 on failure.
function http_accept(sequence) {
  if (httpFailure !== null) return [null, -1];
  const ready = httpReady.shift();
  if (ready === undefined) return [null, 0];
  httpNextConnection += 1;
  httpConnections.set(httpNextConnection, ready.socket);
  return [
    {
      id: `req-${sequence}`,
      connection: httpNextConnection,
      request: { header: ready.header, body: ready.body },
      response: { status: 0, content_type: "", header: "", body: "" },
    },
    1,
  ];
}

function http_render_header(response) {
  const type = response.content_type || "text/plain; charset=utf-8";
  const allow = response.status === 405 ? "Allow: GET, HEAD\r\n" : "";
  return `HTTP/1.1 ${response.status} ${http_status_text(response.status)}\r\nContent-Type: ${type}\r\n` +
    `Content-Length: ${Buffer.byteLength(response.body || "")}\r\nConnection: close\r\n` +
    `Server: Wirecraft-WebServer/0.1\r\n${allow}\r\n`;
}

// Writes the response to the pending request's connection, logs it and closes it; 0 on success.
function http_resolve(pending, resolved) {
  const socket = httpConnections.get(pending.connection);
  httpConnections.delete(pending.connection);
  const body = Buffer.from(resolved.response.body || "", "utf8");
  const head = pending.request.header.startsWith("HEAD ");
  let status = -1;
  if (socket !== undefined && socket.writable) {
    socket.write(resolved.response.header, "latin1");
    if (!head && body.length > 0) socket.write(body);
    socket.end();
    status = 0;
  }
  const [method = "", path = ""] = pending.request.header.trim().split(/\s+/);
  console.log(`${pending.id} ${method} ${path} ${resolved.response.status} ${body.length}`);
  return status;
}
