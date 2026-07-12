//
// pump.rs
//
// Copyright (C) 2026 Posit Software, PBC. All rights reserved.
//
//

//! Bidirectional byte pump between the bridge's stdio and the LSP TCP stream.
//!
//! LSP framing (`Content-Length: N\r\n\r\n{json}`) is transport-independent, so
//! the pump never parses it: it copies raw bytes in two independent directions,
//! each on its own thread.
//!
//! - **stdin → TCP**: forwards the editor's requests to the kernel's LSP server.
//!   This thread blocks in `stdin.read()` whenever the editor is idle. A blocked
//!   `read()` can't be cleanly interrupted from another thread, so we never try
//!   to join it: it is detached and reaped when the process exits (an acceptable
//!   way to end a blocked stdin reader).
//! - **TCP → stdout**: forwards the server's responses back to the editor.
//!   Because stdout must carry LSP frames the instant they arrive, each chunk is
//!   flushed immediately (bypassing stdout's line buffering, which would
//!   otherwise hold back a frame whose JSON body has no trailing newline).
//!
//! A small supervisor ([`Bridge::wait`]) blocks until whichever direction ends
//! first, so the caller can shut the kernel down and then drain the remaining
//! stdout bytes via [`Bridge::shutdown`].

use std::io::stdin;
use std::io::stdout;
use std::io::Read;
use std::io::Write;
use std::net::Shutdown;
use std::net::TcpStream;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

use anyhow::Context;

/// Buffer size for a single copy chunk. LSP messages are small, so 8 KiB is
/// plenty and keeps latency low.
const COPY_BUFFER_SIZE: usize = 8192;

/// Which direction of the pump reached its end first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    /// stdin reached EOF: the editor closed us.
    StdinClosed,

    /// The LSP TCP stream closed: the kernel's LSP server exited.
    TcpClosed,
}

/// A running bidirectional pump between stdio and the LSP TCP stream.
///
/// Owns the joinable stdout pump so trailing bytes can be flushed during
/// shutdown. The stdin pump is deliberately detached (see the module docs).
pub struct Bridge {
    /// Receives the first [`Ending`] from whichever direction finishes first.
    endings: Receiver<Ending>,

    /// The TCP → stdout pump, joined during [`Bridge::shutdown`] so any bytes
    /// still in flight reach stdout before the process exits.
    stdout_thread: Option<JoinHandle<()>>,
}

impl Bridge {
    /// Start pumping bytes between stdio and `stream`.
    ///
    /// Spawns both copy loops and returns immediately; use [`Bridge::wait`] to
    /// block until one direction ends.
    pub fn start(stream: TcpStream) -> anyhow::Result<Self> {
        let (ending_tx, endings) = channel();

        // Each direction gets its own handle to the shared socket. Closing one
        // direction (via `shutdown`) is visible to the other.
        let stdin_stream = stream
            .try_clone()
            .context("Failed to clone the LSP stream for the stdin pump")?;
        let stdout_stream = stream;

        // stdin → TCP: detached, since it may block forever in `stdin.read()`.
        let stdin_ending_tx = ending_tx.clone();
        stdext::spawn!("lsp-bridge-stdin", move || {
            pump_stdin_to_tcp(stdin_stream, stdin_ending_tx);
        });

        // TCP → stdout: joined during shutdown to flush trailing bytes.
        let stdout_thread = stdext::spawn!("lsp-bridge-stdout", move || {
            pump_tcp_to_stdout(stdout_stream, ending_tx);
        });

        Ok(Self {
            endings,
            stdout_thread: Some(stdout_thread),
        })
    }

    /// Block until one direction of the pump ends, returning which one.
    ///
    /// The caller should shut the kernel down after this returns; that closes
    /// the TCP stream, which lets any still-running stdout pump drain and end.
    pub fn wait(&self) -> Ending {
        // A `recv` error means both senders dropped without sending, which can't
        // happen while either pump thread is alive. Treat it as stdin closing so
        // the caller still shuts down.
        self.endings.recv().unwrap_or(Ending::StdinClosed)
    }

    /// Join the stdout pump so trailing bytes reach stdout before we return.
    ///
    /// Call this only after the kernel has been asked to shut down: that closes
    /// the TCP stream, which ends the stdout pump. The detached stdin pump (if
    /// still blocked on `read()`) is left to be reaped by process exit.
    pub fn shutdown(mut self) {
        if let Some(thread) = self.stdout_thread.take() {
            let _ = thread.join();
        }
    }
}

/// Copy stdin to the LSP server until stdin reaches EOF (or errors), then
/// half-close the socket so the server sees no more requests are coming.
fn pump_stdin_to_tcp(mut stream: TcpStream, ending_tx: Sender<Ending>) {
    let mut stdin = stdin().lock();

    if let Err(err) = copy_stream(&mut stdin, &mut stream) {
        log::trace!("stdin → LSP pump ended with error: {err:?}");
    }

    // Tell the kernel's LSP server the client is done sending: this drives its
    // read loop to EOF, which is the LSP transport's disconnect signal.
    let _ = stream.shutdown(Shutdown::Write);
    let _ = ending_tx.send(Ending::StdinClosed);
}

/// Copy the LSP server's output to stdout until the stream closes (or errors),
/// flushing each chunk so frames reach the editor without delay.
fn pump_tcp_to_stdout(mut stream: TcpStream, ending_tx: Sender<Ending>) {
    let mut stdout = stdout().lock();

    if let Err(err) = copy_stream(&mut stream, &mut stdout) {
        log::trace!("LSP → stdout pump ended with error: {err:?}");
    }

    let _ = stdout.flush();
    let _ = ending_tx.send(Ending::TcpClosed);
}

/// Copy bytes from `reader` to `writer`, flushing after every chunk.
///
/// Unlike [`std::io::copy`], this flushes eagerly. That matters for stdout,
/// whose default line buffering would otherwise hold back an LSP frame until the
/// next newline (LSP JSON bodies have none), stalling the reply.
fn copy_stream(reader: &mut impl Read, writer: &mut impl Write) -> std::io::Result<()> {
    let mut buffer = [0u8; COPY_BUFFER_SIZE];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            return Ok(());
        }
        writer.write_all(&buffer[..count])?;
        writer.flush()?;
    }
}
