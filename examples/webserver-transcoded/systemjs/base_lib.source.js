// Platform modules for the server libraries: TCP sockets from the host. systemJS scripts are
// engine-neutral; this design needs a host with `node:net`, such as Bun or Node.js.
const net = require("node:net");
