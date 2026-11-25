//! Loopback gRPC servers for integration tests.

use std::{
    error::Error,
    io,
    net::{IpAddr, Ipv6Addr, SocketAddr},
    time::Duration,
};

use tokio::{net::TcpListener, task::JoinHandle, time::timeout};
use tonic::transport::{Channel, Endpoint, server::Router};
use triggered::Trigger;

/// A running test server and its connected client transport.
pub struct TestServer {
    addr: SocketAddr,
    channel: Channel,
    shutdown: Trigger,
    task: ServerTask,
}

impl TestServer {
    /// Start a server on IPv6 loopback and connect over plaintext HTTP/2.
    pub async fn start(router: Router) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Self::start_with_endpoint(router, Ipv6Addr::LOCALHOST.into(), |addr| {
            Ok(Endpoint::from_shared(format!("http://{addr}"))?)
        })
        .await
    }

    /// Start a server on an explicit loopback IP with a custom endpoint.
    ///
    /// The endpoint receives the bound loopback address. Its connect timeout
    /// bounds startup, with a five-second default when it is unset.
    pub async fn start_with_endpoint(
        router: Router,
        bind_ip: IpAddr,
        endpoint: impl FnOnce(SocketAddr) -> Result<Endpoint, Box<dyn Error + Send + Sync>>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        if !bind_ip.is_loopback() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "test server requires a loopback IP").into());
        }
        let listener = TcpListener::bind((bind_ip, 0)).await?;
        let addr = listener.local_addr()?;
        let endpoint = endpoint(addr)?;
        let connect_timeout = endpoint
            .get_connect_timeout()
            .unwrap_or(Duration::from_secs(5));
        let (shutdown, signal) = triggered::trigger();
        let mut task = ServerTask(tokio::spawn(crate::serve(router, listener, signal)));

        let channel = tokio::select! {
            result = &mut task.0 => {
                result??;
                return Err(io::Error::other("test server exited before the client connected").into());
            }
            result = timeout(connect_timeout, endpoint.connect()) => result??,
        };
        Ok(Self {
            addr,
            channel,
            shutdown,
            task,
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn channel(&self) -> Channel {
        self.channel.clone()
    }

    /// Request graceful shutdown and report any server or task error.
    pub async fn shutdown(mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.shutdown.trigger();
        (&mut self.task.0).await??;
        Ok(())
    }
}

struct ServerTask(JoinHandle<Result<(), tonic::transport::Error>>);

impl Drop for ServerTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use tonic::{service::Routes, transport::Server};

    #[tokio::test]
    async fn connects_to_a_dynamic_port_and_shuts_down() {
        let router = Server::builder().add_routes(Routes::default());
        let server = TestServer::start(router).await.unwrap();
        assert_eq!(server.addr().ip(), IpAddr::V6(Ipv6Addr::LOCALHOST));
        assert_ne!(server.addr().port(), 0);
        timeout(Duration::from_secs(5), server.shutdown())
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn connects_over_ipv4_when_requested() {
        let router = Server::builder().add_routes(Routes::default());
        let server = TestServer::start_with_endpoint(router, Ipv4Addr::LOCALHOST.into(), |addr| {
            Ok(Endpoint::from_shared(format!("http://{addr}"))?)
        })
        .await
        .unwrap();
        assert_eq!(server.addr().ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
        timeout(Duration::from_secs(5), server.shutdown())
            .await
            .unwrap()
            .unwrap();
    }
}
