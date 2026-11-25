use std::{future::Future, io};

use signal_hook::consts::SIGTERM;
use signal_hook_tokio::Signals;
use tokio::net::TcpListener;
use tokio_stream::{StreamExt, wrappers::TcpListenerStream};
use tonic::transport::{Error, server::Router};
use triggered::Listener;

mod tls;
pub use tls::server_builder;

/// Serve a configured gRPC router on an already bound listener.
pub async fn serve(
    router: Router,
    listener: TcpListener,
    shutdown: impl Future<Output = ()>,
) -> Result<(), Error> {
    router
        .serve_with_incoming_shutdown(TcpListenerStream::new(listener), shutdown)
        .await
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
