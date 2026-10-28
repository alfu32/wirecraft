// Counters that a C body keeps in statics live in the run context.
const state = context.imports.socket_accept ??= {
  sequence: 0,
  resolved: 0,
  limit: parseInt(process.env.WEBSERVER_MAX_REQUESTS ?? "", 10) || 0,
};
// Both buffers of the awaitResponseLoop self-link hold the pending copies.
const pending = [awaitResponseLoop_in, awaitResponseLoop_out];

// 1. Resolve every response against its pending copy, rotating each loop buffer once.
while (response.length > 0) {
  const item = response.shift();
  let found = false;
  for (const side of pending) {
    for (let remaining = side.length; remaining > 0; remaining -= 1) {
      const waiting = side.shift();
      if (found || waiting.id !== item.id) {
        side.push(waiting);
        continue;
      }
      found = true;
      if (libhttp.http_resolve(waiting, item) !== 0) {
        console.error("webserver: client closed the connection early");
      }
    }
  }
  if (!found) throw new Error(`webserver: response ${item.id} has no pending request`);
  state.resolved += 1;
  if (state.limit > 0 && state.resolved >= state.limit) threadworkRunner.shutdownRequest();
}
// 2. Accept at most one connection; wait briefly only when nothing is in flight.
if (threadworkRunner.isRunning()) {
  const idle = awaitResponseLoop_in.length === 0 && awaitResponseLoop_out.length === 0;
  if (libhttp.http_wait(idle ? 25 : 0)) {
    state.sequence += 1;
    const [accepted, status] = libhttp.http_accept(state.sequence);
    if (status < 0) throw new Error("webserver: accepting a connection failed");
    if (accepted !== null) {
      // Packets are copied by value, as in C: each path gets its own request.
      request.push(JSON.parse(JSON.stringify(accepted)));
      awaitResponseLoop_out.push(accepted);
    }
  }
} else if (awaitResponseLoop_in.length === 0 && awaitResponseLoop_out.length === 0) {
  libhttp.http_close();
}
