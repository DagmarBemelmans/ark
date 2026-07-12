//
// sidecar.rs
//
// Copyright (C) 2026 Posit Software, PBC. All rights reserved.
//
//

//! Boots a full ark kernel as a **child process** of the current binary and
//! connects to it as a minimal Jupyter frontend.
//!
//! We spawn ourselves (`std::env::current_exe()`) as a separate process rather
//! than starting the kernel on a thread (as the test suite does). This gives
//! the kernel its own file descriptors and process state, isolating the R
//! session from the bridge (see decision 0002).
//!
//! The wiring mirrors the test fixtures: we set up a loopback
//! [`DummyConnection`], hand the child a connection (really a registration)
//! file on disk, and complete the handshake with
//! [`DummyFrontend::from_connection`]. From then on the bridge can talk Jupyter
//! to a live R session.

use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::PoisonError;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::Instant;

use amalthea::fixtures::dummy_frontend::DummyConnection;
use amalthea::fixtures::dummy_frontend::DummyFrontend;
use amalthea::registration_file::RegistrationFile;
use amalthea::socket::Socket;
use amalthea::wire::jupyter_message::JupyterMessage;
use amalthea::wire::jupyter_message::Message;
use amalthea::wire::jupyter_message::ProtocolMessage;
use amalthea::wire::shutdown_request::ShutdownRequest;
use anyhow::Context;
use tempfile::TempDir;

/// How long the drain thread blocks on the iopub socket before looping back to
/// re-check its stop flag.
const IOPUB_POLL_MS: i64 = 250;

/// Total budget for a graceful shutdown: waiting for the kernel's shutdown
/// reply and for the child process to exit, before we resort to `kill()`.
const SHUTDOWN_BUDGET: Duration = Duration::from_secs(5);

/// Shared slot the iopub drain thread checks on every message: when a
/// subscription is active it holds the channel to forward messages onto,
/// otherwise the message is discarded.
type IopubSubscriber = Arc<Mutex<Option<Sender<Message>>>>;

/// A running ark kernel child process and the frontend connection to it.
///
/// Owns everything needed to keep the connection alive and to shut the kernel
/// down cleanly: the child process, the frontend sockets, the temporary
/// directory holding the connection file, and the background iopub drain
/// thread. Dropping the handle shuts the kernel down and guarantees the child
/// does not outlive us.
pub struct Sidecar {
    /// Control channel to the kernel; used to send the shutdown request and
    /// receive its reply.
    control_socket: Socket,

    /// The shell socket, used to open the LSP comm inside the kernel.
    shell_socket: Socket,

    /// The remaining frontend sockets. We don't use them, but we keep them
    /// alive so the full Jupyter connection stays established.
    _stdin_socket: Socket,
    _heartbeat_socket: Socket,

    /// While a subscription is active, the drain thread forwards iopub messages
    /// here instead of discarding them. Used briefly while opening the LSP comm
    /// and reading back the `server_started` port.
    iopub_subscriber: IopubSubscriber,

    /// The kernel child process.
    child: Child,

    /// The child's process id, cached so callers can report it after the child
    /// has been reaped.
    child_pid: u32,

    /// Holds the connection file on disk. Deleted when dropped.
    _connection_dir: TempDir,

    /// Set to signal the iopub drain thread to stop.
    drain_stop: Arc<AtomicBool>,

    /// The iopub drain thread, joined during shutdown.
    drain_thread: Option<JoinHandle<()>>,

    /// Whether [`Sidecar::shutdown`] has already run, so `Drop` doesn't repeat
    /// the work.
    shut_down: bool,
}

impl Sidecar {
    /// Boot the kernel child process and connect to it.
    ///
    /// `bridge_log_file` is the bridge's own `--log` path, if any. The kernel's
    /// log is placed alongside it (`<bridge-log-dir>/kernel.log`); when the
    /// bridge logs to stderr instead, the kernel log goes into the private temp
    /// directory.
    ///
    /// Blocks until the kernel is fully up (handshake complete and the initial
    /// iopub messages consumed).
    pub fn boot(bridge_log_file: Option<&str>) -> anyhow::Result<Self> {
        // Set up the loopback connection, exactly as the test fixtures do. The
        // registration file avoids the port-binding race: the child binds the
        // real ports and hands them back over the registration socket.
        let connection = DummyConnection::new();
        let (_connection_file, registration_file) = connection.get_connection_files();

        // Write the connection info to a private temp dir. The child parses it
        // as a `RegistrationFile` and completes the handshake.
        let connection_dir = tempfile::tempdir().context("Failed to create temp dir")?;
        let connection_path = connection_dir.path().join("connection.json");
        write_registration_file(&connection_path, &registration_file)?;

        let kernel_log = kernel_log_path(bridge_log_file, connection_dir.path());

        // Spawn ourselves as the kernel. stdin is closed; stdout and stderr are
        // piped so we can forward them to the bridge's stderr (never stdout,
        // which is reserved for LSP frames).
        let exe = std::env::current_exe().context("Failed to determine path to current exe")?;
        let mut child = Command::new(&exe)
            .arg("--connection_file")
            .arg(&connection_path)
            .arg("--session-mode")
            .arg("background")
            .arg("--log")
            .arg(&kernel_log)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn kernel child process {exe:?}"))?;

        let child_pid = child.id();
        log::info!("Spawned kernel child process (pid {child_pid})");

        forward_child_output(child.stdout.take(), "kernel out");
        forward_child_output(child.stderr.take(), "kernel err");

        // If the handshake below panics (e.g. the kernel died during boot),
        // make sure we don't leak the child process.
        let guard = ChildGuard(Some(child));
        let frontend = DummyFrontend::from_connection(connection);
        let child = guard.disarm();

        // The frontend bundles all sockets into a single value that is not
        // `Sync`, so we can't share it between the drain thread and the
        // shutdown path. `DummyFrontend` has no `Drop` impl, so we move the
        // sockets out individually (its private `session` field is left behind
        // and dropped). Each `Socket` carries its own `Session`, which is all
        // we need to send further messages.
        let control_socket = frontend.control_socket;
        let shell_socket = frontend.shell_socket;
        let iopub_socket = frontend.iopub_socket;
        let stdin_socket = frontend.stdin_socket;
        let heartbeat_socket = frontend.heartbeat_socket;

        // A full iopub queue freezes the kernel (bounded channels, see
        // `start.rs`), so drain it continuously on a background thread. The
        // drain forwards messages to a subscriber when one is registered, and
        // discards them otherwise.
        let drain_stop = Arc::new(AtomicBool::new(false));
        let iopub_subscriber: IopubSubscriber = Arc::new(Mutex::new(None));
        let drain_thread = spawn_iopub_drain(
            iopub_socket,
            Arc::clone(&drain_stop),
            Arc::clone(&iopub_subscriber),
        );

        Ok(Self {
            control_socket,
            shell_socket,
            _stdin_socket: stdin_socket,
            _heartbeat_socket: heartbeat_socket,
            child,
            child_pid,
            _connection_dir: connection_dir,
            drain_stop,
            drain_thread: Some(drain_thread),
            iopub_subscriber,
            shut_down: false,
        })
    }

    /// The kernel child's process id.
    pub fn child_pid(&self) -> u32 {
        self.child_pid
    }

    /// Send a Jupyter protocol message to the kernel on the shell socket.
    pub fn send_shell<T: ProtocolMessage>(&self, message: T) -> anyhow::Result<()> {
        JupyterMessage::create(message, None, &self.shell_socket.session)
            .send(&self.shell_socket)
            .context("Failed to send message on shell socket")
    }

    /// Temporarily route iopub messages to the caller instead of discarding
    /// them.
    ///
    /// While the returned [`IopubSubscription`] is alive, the drain thread
    /// hands every iopub message to it; dropping it restores draining. Only one
    /// subscription is meaningful at a time.
    pub fn subscribe_iopub(&self) -> IopubSubscription {
        let (tx, rx) = channel();
        *lock_subscriber(&self.iopub_subscriber) = Some(tx);

        IopubSubscription {
            rx,
            subscriber: Arc::clone(&self.iopub_subscriber),
        }
    }

    /// Shut the kernel down cleanly.
    ///
    /// Sends a Jupyter `shutdown_request`, waits up to [`SHUTDOWN_BUDGET`] for
    /// the reply and for the child to exit, then force-kills the child if it is
    /// still running. Idempotent.
    pub fn shutdown(&mut self) -> anyhow::Result<()> {
        if self.shut_down {
            return Ok(());
        }
        self.shut_down = true;

        let deadline = Instant::now() + SHUTDOWN_BUDGET;

        let request = JupyterMessage::create(
            ShutdownRequest { restart: false },
            None,
            &self.control_socket.session,
        );
        match request.send(&self.control_socket) {
            Ok(()) => wait_for_shutdown_reply(&self.control_socket, deadline),
            Err(err) => log::warn!("Failed to send shutdown request to kernel: {err:?}"),
        }

        self.wait_for_child_exit(deadline);

        // Stop and join the iopub drain thread now that the kernel is gone.
        self.drain_stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.drain_thread.take() {
            let _ = thread.join();
        }

        Ok(())
    }

    /// Wait for the child to exit before `deadline`, killing it if it overstays.
    fn wait_for_child_exit(&mut self, deadline: Instant) {
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    log::info!("Kernel child exited with status {status}");
                    return;
                },
                Ok(None) => {
                    if Instant::now() >= deadline {
                        log::warn!("Kernel child did not exit in time; killing it");
                        let _ = self.child.kill();
                        let _ = self.child.wait();
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                },
                Err(err) => {
                    log::warn!("Error waiting for kernel child: {err:?}");
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    return;
                },
            }
        }
    }
}

impl Drop for Sidecar {
    fn drop(&mut self) {
        if let Err(err) = self.shutdown() {
            log::warn!("Error shutting down kernel sidecar: {err:?}");
        }

        // Belt and suspenders: never let the child outlive us.
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// Where the kernel child should write its log.
///
/// It sits next to the bridge log when there is one, otherwise in the private
/// temp dir.
fn kernel_log_path(bridge_log_file: Option<&str>, temp_dir: &Path) -> PathBuf {
    let dir = bridge_log_file
        .map(Path::new)
        .and_then(Path::parent)
        .filter(|parent| !parent.as_os_str().is_empty());

    match dir {
        Some(dir) => dir.join("kernel.log"),
        None => temp_dir.join("kernel.log"),
    }
}

/// Serialize the registration info to `path` as JSON.
///
/// `RegistrationFile` only derives `Deserialize`, so we build the JSON object
/// by hand from its public fields.
fn write_registration_file(path: &Path, registration: &RegistrationFile) -> anyhow::Result<()> {
    let json = serde_json::json!({
        "transport": registration.transport,
        "signature_scheme": registration.signature_scheme,
        "ip": registration.ip,
        "key": registration.key,
        "registration_port": registration.registration_port,
    });

    let contents = serde_json::to_string_pretty(&json).context("Failed to serialize connection")?;
    std::fs::write(path, contents).with_context(|| format!("Failed to write {path:?}"))
}

/// Forward the child's stdout/stderr to the bridge's stderr, one tagged line at
/// a time. Never writes to stdout, which carries LSP frames.
fn forward_child_output<R>(reader: Option<R>, tag: &'static str)
where
    R: Read + Send + 'static,
{
    let Some(reader) = reader else {
        return;
    };

    stdext::spawn!(format!("sidecar-{tag}"), move || {
        let reader = BufReader::new(reader);
        for line in reader.lines() {
            let Ok(line) = line else {
                break;
            };
            eprintln!("{tag}: {line}");
        }
    });
}

/// Continuously receive iopub messages until signalled to stop, forwarding
/// each to the active subscriber or discarding it.
fn spawn_iopub_drain(
    iopub_socket: Socket,
    stop: Arc<AtomicBool>,
    subscriber: IopubSubscriber,
) -> JoinHandle<()> {
    stdext::spawn!("sidecar-iopub-drain", move || {
        while !stop.load(Ordering::Relaxed) {
            match iopub_socket.poll_incoming(IOPUB_POLL_MS) {
                Ok(true) => match Message::read_from_socket(&iopub_socket) {
                    Ok(message) => dispatch_iopub_message(&subscriber, message),
                    Err(err) => log::trace!("Error reading iopub message: {err:?}"),
                },
                Ok(false) => {},
                Err(err) => {
                    log::trace!("Error polling iopub socket: {err:?}");
                    break;
                },
            }
        }
    })
}

/// Hand `message` to the active subscriber, or discard it if there is none.
fn dispatch_iopub_message(subscriber: &IopubSubscriber, message: Message) {
    match lock_subscriber(subscriber).as_ref() {
        Some(tx) => {
            let _ = tx.send(message);
        },
        None => log::trace!("Draining iopub message: {message:?}"),
    }
}

/// Lock the subscriber slot, recovering the guard even if a previous holder
/// panicked. The slot holds no invariant worth propagating a poison for.
fn lock_subscriber(subscriber: &IopubSubscriber) -> MutexGuard<'_, Option<Sender<Message>>> {
    subscriber.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Poll the control socket for the kernel's `shutdown_reply`, up to `deadline`.
fn wait_for_shutdown_reply(control_socket: &Socket, deadline: Instant) {
    while Instant::now() < deadline {
        match control_socket.poll_incoming(50) {
            Ok(true) => match Message::read_from_socket(control_socket) {
                Ok(Message::ShutdownReply(_)) => return,
                Ok(other) => {
                    log::trace!("Ignoring control message while awaiting shutdown reply: {other:?}")
                },
                Err(err) => log::trace!("Error reading control message: {err:?}"),
            },
            Ok(false) => {},
            Err(err) => {
                log::trace!("Error polling control socket: {err:?}");
                return;
            },
        }
    }

    log::warn!("Timed out waiting for kernel shutdown reply");
}

/// Kills the wrapped child on drop unless [`ChildGuard::disarm`] is called
/// first. Guards the window between spawning the child and successfully
/// completing the handshake, which can panic.
struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn disarm(mut self) -> Child {
        match self.0.take() {
            Some(child) => child,
            None => panic!("ChildGuard disarmed twice"),
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// A temporary lease on the sidecar's iopub stream.
///
/// While alive, the drain thread forwards iopub messages here instead of
/// discarding them. Dropping it restores draining.
pub struct IopubSubscription {
    rx: Receiver<Message>,
    subscriber: IopubSubscriber,
}

impl IopubSubscription {
    /// Wait up to `timeout` for the next iopub message.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<Message, RecvTimeoutError> {
        self.rx.recv_timeout(timeout)
    }
}

impl Drop for IopubSubscription {
    fn drop(&mut self) {
        *lock_subscriber(&self.subscriber) = None;
    }
}
