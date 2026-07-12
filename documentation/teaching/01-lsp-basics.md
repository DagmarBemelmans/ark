# Teaching note 01 — How LSP communication works (starting from Zed)

*Q&A from 2026-07-11: "How does communication exactly happen? What is stdio?"*
*Companion reading: [../03-what-zed-needs.md](../03-what-zed-needs.md), [../glossary.md](../glossary.md).*

The editor (Zed) is the **client**; the language brain (ark) is the **server**; they
are separate processes having a conversation. This note builds that conversation up
in layers: the byte streams, the message boundaries, the message grammar, and a real
session timeline.

## Layer 0: what stdio actually is

Every process on Linux is born with three open byte streams, handed to it by the
operating system. They have numbers (*file descriptors*) and names:

| fd | name | direction | conventional purpose |
|---|---|---|---|
| 0 | **stdin** | into the process | input to read |
| 1 | **stdout** | out of the process | the "real" output |
| 2 | **stderr** | out of the process | errors, logs, noise |

In a terminal, all three point at the terminal: stdin is the keyboard, stdout and
stderr are the screen. The key move: **whoever starts a process decides where those
three streams point.** A parent process can attach them to *pipes* — one-way byte
channels the kernel maintains between two processes.

The shell does this constantly: in `ls | wc -l`, the shell connects `ls`'s stdout to
`wc`'s stdin with a pipe. Neither program knows; they just read fd 0 and write fd 1.

"LSP over stdio" is exactly this, with the editor as the parent:

```
        ┌─────────┐  pipe: Zed writes → server's stdin (fd 0)   ┌────────────┐
        │   Zed   │ ═══════════════════════════════════════════▶│  ark lsp   │
        │(client) │ ◀═══════════════════════════════════════════│  (server)  │
        └─────────┘  pipe: server's stdout (fd 1) → Zed reads    └────────────┘
                     stderr (fd 2) ──▶ Zed's "language server logs" panel
```

Zed spawns the command our extension hands it, keeps both pipes, and talks LSP
through them. The design-critical implication: **stdout belongs to the protocol.**
If any code in the server process does the equivalent of `print("debug!")`, those
bytes land in the same pipe, mixed into protocol traffic, and the conversation
derails (Layer 1 shows why concretely). Hence: logs go to stderr or a file, and
[decision 0002](../decisions/0002-sidecar-kernel-architecture.md) puts R in a
separate child process — R and its C libraries love writing to stdout.

## Layer 1: framing — chopping a byte hose into messages

A pipe is a *continuous stream of bytes* with no boundaries. If Zed writes two
messages back to back, nothing in the pipe says where one ends and the next begins.
So LSP borrows HTTP's trick — every message is preceded by a header stating its length:

```
Content-Length: 133\r\n
\r\n
{"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{...}}
```

The reader's loop is dead simple: read the header line, parse the number, read
*exactly* that many bytes, parse them as JSON, repeat forever. This is why one stray
`print` is fatal: the reader finishes a message, expects `Content-Length: ...`, and
instead sees `debug!`. It cannot resynchronize — it has no idea how many bytes to
skip — so the session hangs or dies.

Two consequences for this project:

- The framing works over **any** byte stream. A TCP socket is also just a byte hose,
  so ark serving LSP over TCP (for Positron) and over stdio (for Zed) is the *same
  protocol* — and the bridge in [T04](../tasks/T04-stdio-bridge-and-lifecycle.md) can
  be a dumb byte-copier that never parses anything.
- The framing layer is handled for ark by the `tower-lsp` library; ark's own code
  never touches `Content-Length`.

## Layer 2: JSON-RPC — the grammar of the conversation

Inside each frame is a JSON-RPC message. There are only three kinds.

**A request** — carries an `id`, demands exactly one response:

```json
{"jsonrpc": "2.0", "id": 42, "method": "textDocument/completion",
 "params": {"textDocument": {"uri": "file:///home/dagmar/analysis.R"},
            "position": {"line": 10, "character": 8}}}
```

**A response** — carries the *same* `id`, plus a `result` or an `error`:

```json
{"jsonrpc": "2.0", "id": 42,
 "result": {"items": [{"label": "mutate", "kind": 3}]}}
```

**A notification** — no `id`, fire-and-forget, no reply allowed:

```json
{"jsonrpc": "2.0", "method": "textDocument/didChange",
 "params": {"textDocument": {"uri": "file:///home/dagmar/analysis.R", "version": 7},
            "contentChanges": [{"range": {}, "text": "d"}]}}
```

The `id` is what makes the conversation asynchronous: Zed can have a completion
request, a hover request, and a folding-range request all in flight, and it matches
responses to requests by `id` regardless of answer order. The conversation is
**two-way** — the server sends messages too. Diagnostics, for instance, are a
*server→client notification* (`textDocument/publishDiagnostics`): ark decides when
to push them; Zed never asks.

## Layer 3: a real Zed session, start to finish

1. You open an `.R` file. Zed asks the extension "what command?", gets `ark lsp`,
   spawns it with the pipes attached.
2. **`initialize` request** (client→server): Zed introduces itself — workspace
   folders and its *capabilities* (which protocol features it understands).
3. The server responds with **its** capabilities: "I do completions (trigger on `$`,
   `@`, `:`…), hover, definitions…". This negotiation is how Zed learns what ark can
   do — ark builds that answer in `crates/ark/src/lsp/capabilities.rs`. Zed sends the
   `initialized` notification; the session is live.
4. **`textDocument/didOpen`** notification: contains the *entire file text*. From
   here on the server keeps its own in-memory copy of the buffer — it does not read
   your unsaved file from disk.
5. You type. Each keystroke batch becomes a **`didChange`** notification carrying
   just the edited range (that's what `version: 7` tracks). Server and editor stay
   byte-identical without touching disk.
6. You pause after `dplyr::mut` → **`completion` request** → ark consults its index
   and the live R session → response with items. Meanwhile ark pushes
   **`publishDiagnostics`** whenever its analysis finds problems.
7. You close Zed → **`shutdown` request**, then **`exit` notification**, then Zed
   closes the pipes. A closed stdin ("EOF") is itself a signal: it's the tripwire the
   bridge uses ([T02](../tasks/T02-sidecar-kernel-boot.md)/[T04](../tasks/T04-stdio-bridge-and-lifecycle.md))
   to know the editor is gone and the R sidecar must be torn down.

## See it live, today

No project code needed — Zed already runs rust-analyzer on this repo. Open any `.rs`
file in Zed, then command palette → **`dev: open language server logs`** → pick
rust-analyzer → view **RPC messages**. You'll watch layers 1–3 scroll by:
`initialize`, `didChange` on every keystroke, completion requests with their `id`s.

## Next question in the queue

What happens *inside* ark once a completion request arrives — the LSP main loop,
`r_task`, and how the live R session gets consulted. (Future teaching note.)
