# Teaching note 02 — How Positron and ark communicate

*Q&A from 2026-07-11: the same deep dive as [note 01](01-lsp-basics.md), from the
ark–Positron side. All `file:line` references verified against the repo on that date.*

Positron and ark speak **two protocols at once**: the Jupyter protocol (running code,
side channels) and LSP (editing intelligence). The punchline of this note is that
Jupyter's only LSP job is to be the *matchmaker* — it introduces the two parties, then
gets out of the way.

## Layer 0: the birth of the connection

Like Zed, Positron spawns ark as a child process. Unlike Zed, it does not keep the
stdio pipes as the protocol channel. Instead it passes a file:

```
ark --connection_file /tmp/connection.json --session-mode console
```

The connection file (`crates/amalthea/src/connection_file.rs:18`) contains everything
needed to rendezvous over the network:

```jsonc
{
  "control_port": 50160, "shell_port": 57503, "stdin_port": 52597,
  "iopub_port": 40885,  "hb_port": 42540,
  "transport": "tcp", "ip": "127.0.0.1",
  "signature_scheme": "hmac-sha256",
  "key": "a0436f6c-1916-498b-8eb9-e81ab9368e84"   // shared secret
}
```

Five ports, one per socket (next layer), plus a shared secret for signing messages.

**A recurring motif: whoever binds the socket picks the port.** Two processes can't
both "just pick" port 57503 — someone else might grab it first (a race condition).
The classic Jupyter flow has the *frontend* bind... which creates a race when the
kernel comes up. So ark also supports a second scheme
(`crates/amalthea/src/kernel.rs:344-373`): the file can instead be a
**registration file** containing one port the frontend is listening on; ark then binds
all five sockets itself on OS-assigned free ports and *phones home* with a
`HandshakeRequest` announcing where it landed (`kernel.rs:304-312`). Keep this motif
in mind — the LSP bootstrap in Layer 3 uses it again.

## Layer 1: the transport — ZMQ, and how it differs from a pipe

Jupyter traffic runs over **ZeroMQ (ZMQ)** sockets. The conceptual difference from
stdio matters more than the technology: a pipe is a byte hose (hence LSP's
`Content-Length` framing), while **ZMQ is message-oriented** — it delivers whole
multipart messages, atomically. No framing headers needed; the transport itself knows
where messages begin and end.

There are five sockets because the conversation has five different shapes:

| Socket | Pattern | Carries |
|---|---|---|
| **shell** | request → reply | "run this code", completion requests, comm traffic |
| **iopub** | broadcast (pub/sub) | everything the world may watch: stdout/stderr streams, results, busy/idle status |
| **stdin** | *reversed* request | the kernel asks the *frontend* a question (`readline()`, password prompts) |
| **control** | request → reply, priority lane | interrupt, shutdown — works even while shell is busy computing |
| **heartbeat** | echo | "are you alive?" ping |

The split earns its keep: iopub lets *multiple* frontends watch one session's output;
control lets you interrupt a kernel whose shell socket is stuck behind a long
computation.

Each message is a multipart ZMQ frame sequence
(`crates/amalthea/src/wire/wire_message.rs:24`):

```
[ routing ids… , "<IDS|MSG>" , signature , header , parent_header , metadata , content ]
```

- **signature** — HMAC-SHA256 over the payload using the connection file's `key`
  (`crates/amalthea/src/session.rs:18-47`). Anyone on localhost could connect to the
  ports; only the party holding the key can forge valid messages.
- **header** — message id, type (`execute_request`, `comm_open`, …), session.
- **parent_header** — the header of the message *this one responds to*. This is
  Jupyter's version of JSON-RPC's `id` matching: when results broadcast on iopub, the
  parent header tells frontends which request produced them.

## Layer 2: comms — Jupyter's extension mechanism

On top of shell/iopub, Jupyter offers **comms**: named, free-form side channels.
Either side can send `comm_open` (with a `target_name` and a JSON payload), then
`comm_msg` traffic flows both ways, then `comm_close`. Positron builds most of its
IDE integration this way — there are comm targets for the variables pane, plots,
help, UI events (see `crates/ark/src/*` modules and the generated types in
`crates/amalthea/src/comm/`).

Ark registers two *special* comm targets at startup
(`crates/ark/src/start.rs:133-135`): `"lsp"` and `"ark_dap"`. These are "server
handlers" — comms whose only job is to start a server. `ServerComm`
(`crates/amalthea/src/comm/server_comm.rs`) is the adapter that makes the LSP look
like a comm to the routing code (`crates/amalthea/src/socket/shell.rs:272,426`).

## Layer 3: the LSP bootstrap dance

Now the full sequence Positron performs — the one our bridge will impersonate:

```
Positron                                   ark (kernel process)
   │                                          │
   │ 1. comm_open target="lsp"                │
   │    data: {"ip_address": "127.0.0.1"} ───▶│  ServerStartMessage (server_comm.rs:19)
   │                                          │  2. waits until R is initialized
   │                                          │     (handler.rs:71, kernel_init bus)
   │                                          │  3. binds TCP 127.0.0.1:0 → OS picks
   │                                          │     port, e.g. 39217 (backend.rs:564)
   │ 4. comm_msg method="server_started"      │
   │◀── params: {"port": 39217} ──────────────│  ServerStartedMessage (server_comm.rs:38)
   │                                          │
   │ 5. TCP connect to 127.0.0.1:39217 ──────▶│  accept (backend.rs:586)
   │                                          │
   │◀═══ 6. LSP: Content-Length + JSON-RPC ══▶│  tower-lsp (backend.rs:635)
   │        (identical to teaching note 01)   │
```

Things worth savoring:

- **The port-picking motif again**: the *server* binds, so the *server* picks the
  port and reports it back — the comment on `ServerStartMessage` says exactly this
  ("to prevent race conditions", `server_comm.rs:20-23`).
- **Roles are reversed relative to intuition**: ark (the LSP *server*) listens; the
  editor (the *client*) dials in. With Zed there is no dialing at all — the parent
  simply owns both pipe ends.
- **After step 5, Jupyter is out of the picture.** The LSP bytes flow over their own
  TCP socket, framed with `Content-Length`, exactly as in note 01. Same grammar, same
  lifecycle (`initialize` → capabilities → `didOpen` → …). Only the hose differs.
- Reconnects are supported: the `Lsp` handler keeps its tokio runtime so a new
  `comm_open` can start a fresh cycle (`crates/ark/src/lsp/handler.rs:79-81`).

## Layer 4: life inside the shared process

Why go through all this instead of running the LSP as its own program? Because
cohabitation with R *is the feature*:

- A completion request arriving on the LSP's TCP socket is handled on tokio worker
  threads, but the interesting answers (what does `dplyr` export?) live inside the R
  interpreter on the **main thread**. `r_task()` (`crates/ark/src/r_task.rs:272`)
  queues a closure over to R and waits — one process, so it's a channel hop, not IPC.
- The bridge runs the other way too: when you execute code in Positron's console, the
  console notifies the LSP (`Console::get_mut().set_lsp_channel(...)`,
  `crates/ark/src/lsp/backend.rs:605`) so completions instantly know about variables
  you just created and packages you just attached.
- **stdout has the opposite fate from Zed-mode.** In kernel mode, stdout is not
  sacred — it's deliberately *captured* and rebroadcast as iopub `stream` messages
  (the `--no-capture-streams` flag in `main.rs` turns this off). In Zed-mode stdout
  *is* the protocol. Same file descriptor, two opposite designs — which is why
  decision 0002 keeps R in a separate process from the stdio bridge.

## Side by side

| | Positron ↔ ark | Zed ↔ ark (our project) |
|---|---|---|
| Who spawns ark | Positron's R runtime code | Zed, via our extension |
| Bootstrap | connection file + ZMQ + comm handshake | none — process start *is* the handshake |
| Code execution | yes — shell socket (`execute_request`) | not in v1 (REPL uses a separate kernel instance) |
| LSP transport | TCP socket, port negotiated via comm | the stdio pipes themselves |
| LSP protocol | identical (JSON-RPC + `Content-Length`) | identical |
| stdout of the R process | captured → iopub stream messages | must never touch the LSP stream → R exiled to sidecar child |

And the design payoff: our bridge ([decision 0002](../decisions/0002-sidecar-kernel-architecture.md))
speaks the **left column downward to its sidecar kernel** and presents the **right
column upward to Zed**. Every message in this note is one the bridge will send or
receive — and the test suite already impersonates Positron this exact way
(`crates/ark_test/src/dummy_frontend.rs:898-933`).

## Next question in the queue

Inside ark when a completion request arrives: the LSP main loop, the auxiliary loop,
`r_task` mechanics, and where oak's static analysis takes over from the live session.
