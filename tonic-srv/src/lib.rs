use std::{
    fs, future::Future, io,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use signal_hook::consts::SIGTERM;
use signal_hook_tokio::Signals;
use tokio::net::TcpListener;
use tokio_stream::{Stream, StreamExt, wrappers::TcpListenerStream};
use tonic::transport::{Error, server::Router};
use triggered::Listener;

#[cfg(feature = "tls")]
mod tls;
#[cfg(feature = "tls")]
pub use tls::server_builder;

pub mod auth;

const READY_PATH: &str = "/tmp/grpc-ready";

/// Serve a configured gRPC router on an already bound listener.
///
/// Publish `/tmp/grpc-ready` when tonic starts polling for connections.
/// File operations are best effort.
pub async fn serve(
    router: Router,
    listener: TcpListener,
    shutdown: impl Future<Output = ()>,
) -> Result<(), Error> {
    let _ = fs::remove_file(READY_PATH);
    let ready = Arc::new(Mutex::new(ReadyFile { stopped: false }));
    let incoming = ReadyIncoming {
        incoming: TcpListenerStream::new(listener),
        ready: Some(Arc::clone(&ready)),
    };
    let shutdown = async {
        shutdown.await;
        ready.lock().unwrap().remove();
    };
    router.serve_with_incoming_shutdown(incoming, shutdown).await
}

struct ReadyIncoming {
    incoming: TcpListenerStream,
    ready: Option<Arc<Mutex<ReadyFile>>>,
}

impl Stream for ReadyIncoming {
    type Item = <TcpListenerStream as Stream>::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(ready) = self.ready.take() {
            let ready = ready.lock().unwrap();
            if !ready.stopped {
                let _ = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(READY_PATH);
            }
        }
        Pin::new(&mut self.incoming).poll_next(cx)
    }
}

struct ReadyFile {
    stopped: bool,
}

impl ReadyFile {
    fn remove(&mut self) {
        if !self.stopped {
            self.stopped = true;
            let _ = fs::remove_file(READY_PATH);
        }
    }
}

impl Drop for ReadyFile {
    fn drop(&mut self) {
        self.remove();
    }
}

/// Run a server until it exits or receives SIGTERM, then await graceful shutdown.
pub async fn with_sigterm<T, E, F>(run: impl FnOnce(Listener) -> F) -> Result<T, E>
where
    E: From<io::Error>,
    F: Future<Output = Result<T, E>>,
{
    let signals = Signals::new([SIGTERM])?;
    let (trigger, shutdown) = triggered::trigger();
    let server = run(shutdown);
    tokio::pin!(server);

    tokio::select! {
        result = &mut server => result,
        () = handle_signals(signals, trigger) => server.await,
    }
}

/// Translate SIGTERM into a graceful shutdown notification.
pub async fn handle_signals(mut signals: Signals, trigger: triggered::Trigger) {
    while let Some(signal) = signals.next().await {
        if signal == SIGTERM {
            println!("SIGTERM caught: going to shutdown");
            trigger.trigger();
        }
    }
}

#[cfg(feature = "otel")]
pub mod otel;

#[cfg(feature = "test-support")]
pub mod test_support;
