//
// lsp_connection.rs
//
// Copyright (C) 2026 Posit Software, PBC. All rights reserved.
//
//

//! Starts ark's LSP server inside the sidecar kernel and connects to it over
//! TCP.
//!
//! The sequence mirrors the test frontend's `start_server`: send a `comm_open`
//! for the `"lsp"` target, read back the `server_started` message carrying the
//! TCP port, then connect. The iopub messages come from a temporary
//! [`IopubSubscription`] rather than the socket directly, since the sidecar's
//! drain thread otherwise consumes them.

use std::net::TcpStream;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;
use std::time::Instant;

use amalthea::wire::comm_open::CommOpen;
use amalthea::wire::jupyter_message::Message;
use anyhow::anyhow;
use anyhow::Context;
use uuid::Uuid;

use super::sidecar::IopubSubscription;
use super::sidecar::Sidecar;

/// The comm target name ark's kernel registers its LSP server under.
const LSP_TARGET_NAME: &str = "lsp";

/// The loopback address the kernel binds the LSP server to and the bridge
/// connects back on.
const LSP_IP_ADDRESS: &str = "127.0.0.1";

/// Start ark's LSP server inside `sidecar` and connect to it over TCP.
///
/// Opens the `"lsp"` comm, waits up to `timeout` for the kernel to report the
/// port it bound to, and returns a stream connected to it. The kernel waits for
/// R to finish initializing before binding, so the wait can take several
/// seconds on a cold start.
pub fn start(sidecar: &Sidecar, timeout: Duration) -> anyhow::Result<TcpStream> {
    let subscription = sidecar.subscribe_iopub();

    let comm_id = Uuid::new_v4().to_string();
    sidecar
        .send_shell(CommOpen {
            comm_id: comm_id.clone(),
            target_name: String::from(LSP_TARGET_NAME),
            data: serde_json::json!({ "ip_address": LSP_IP_ADDRESS }),
        })
        .context("Failed to open the LSP comm")?;

    let port = wait_for_server_started(&subscription, &comm_id, timeout)?;
    drop(subscription);

    let address = format!("{LSP_IP_ADDRESS}:{port}");
    TcpStream::connect(&address)
        .with_context(|| format!("Failed to connect to LSP server at {address}"))
}

/// Read iopub messages until the kernel's `server_started` message arrives,
/// returning the port it carries or an error if `timeout` elapses first.
fn wait_for_server_started(
    subscription: &IopubSubscription,
    comm_id: &str,
    timeout: Duration,
) -> anyhow::Result<u16> {
    let deadline = Instant::now() + timeout;

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(anyhow!(
                "Timed out after {timeout:?} waiting for the LSP server to start"
            ));
        }

        let message = match subscription.recv_timeout(remaining) {
            Ok(message) => message,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => {
                return Err(anyhow!(
                    "The kernel iopub stream closed before the LSP server started"
                ))
            },
        };

        if let Some(port) = server_started_port(&message, comm_id)? {
            return Ok(port);
        }
    }
}

/// Extract the LSP port from our comm's `server_started` message.
///
/// Returns `Ok(None)` for the surrounding status messages and any other
/// traffic, and an error only for a malformed `server_started` message.
fn server_started_port(message: &Message, comm_id: &str) -> anyhow::Result<Option<u16>> {
    let Message::CommMsg(comm_msg) = message else {
        return Ok(None);
    };

    if comm_msg.content.comm_id != comm_id {
        return Ok(None);
    }

    let data = &comm_msg.content.data;
    if data.get("method").and_then(|method| method.as_str()) != Some("server_started") {
        return Ok(None);
    }

    let port = data["params"]["port"]
        .as_u64()
        .ok_or_else(|| anyhow!("`server_started` message from the kernel is missing the port"))?;

    Ok(Some(port as u16))
}
