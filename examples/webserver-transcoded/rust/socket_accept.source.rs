// Counters that a C body keeps in statics live in the run context.
let mut sequence = *context.state::<u64>("socket_accept.sequence");
let mut resolved_total = *context.state::<u64>("socket_accept.resolved");
let limit = *context
    .state::<Option<u64>>("socket_accept.limit")
    .get_or_insert_with(|| {
        std::env::var("WEBSERVER_MAX_REQUESTS")
            .ok()
            .and_then(|text| text.parse().ok())
            .unwrap_or(0)
    });
// Both buffers of the awaitResponseLoop self-link hold the pending copies.
let pending = [awaitResponseLoop_in, awaitResponseLoop_out];

// 1. Resolve every response against its pending copy, rotating each loop buffer once.
while !response.is_empty() {
    let item = pop::<ReqRep>(response)
        .ok_or("webserver: response buffer holds a value that is not a ReqRep")?;
    let mut found = false;
    for side in pending {
        for _ in 0..side.len() {
            let waiting = pop::<ReqRep>(side)
                .ok_or("webserver: pending buffer holds a value that is not a ReqRep")?;
            if found || waiting.id != item.id {
                push(side, waiting);
                continue;
            }
            found = true;
            if libhttp.http_resolve(&waiting, &item) != 0 {
                eprintln!("webserver: client closed the connection early");
            }
        }
    }
    if !found {
        return Err(format!("webserver: response {} has no pending request", item.id).into());
    }
    resolved_total += 1;
    if limit > 0 && resolved_total >= limit {
        threadwork_runner.shutdown_request();
    }
}
// 2. Accept at most one connection; wait briefly only when nothing is in flight.
let idle = awaitResponseLoop_in.is_empty() && awaitResponseLoop_out.is_empty();
if threadwork_runner.is_running() {
    if libhttp.http_wait(if idle { 25 } else { 0 }) {
        sequence += 1;
        if let Some(accepted) = libhttp.http_accept(sequence) {
            push(request, accepted.clone());
            push(awaitResponseLoop_out, accepted);
        }
    }
} else if idle {
    libhttp.http_close();
}
*context.state::<u64>("socket_accept.sequence") = sequence;
*context.state::<u64>("socket_accept.resolved") = resolved_total;
