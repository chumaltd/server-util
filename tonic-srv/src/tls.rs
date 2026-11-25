use std::{error::Error, fs, io};

use server_conf::ServerConf;
use tonic::transport::{Certificate, Identity, Server, ServerTlsConfig};

/// Build a tonic server from the `[listen]` configuration.
///
/// With no TLS settings the server uses plaintext. TLS requires a PEM server
/// certificate and private key. Setting `tls_verify_host = true` also requires
/// a PEM client CA bundle and makes client certificates mandatory.
pub fn server_builder(config: &ServerConf) -> Result<Server, Box<dyn Error + Send + Sync>> {
    let require_client_cert = config.tls_verify_host == Some(true);
    let Some(tls) = &config.tls else {
        if require_client_cert {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "listen.tls is required when listen.tls_verify_host is true",
            )
            .into());
        }
        return Ok(Server::builder());
    };

    let cert = read_pem("listen.tls.cert", tls.cert.as_deref())?;
    let key = read_pem("listen.tls.key", tls.key.as_deref())?;
    let mut tls_config = ServerTlsConfig::new().identity(Identity::from_pem(cert, key));
    if require_client_cert {
        let ca = read_pem("listen.tls.ca", tls.ca.as_deref())?;
        tls_config = tls_config.client_ca_root(Certificate::from_pem(ca));
    }

    Ok(Server::builder().tls_config(tls_config)?)
}

fn read_pem(field: &str, path: Option<&str>) -> io::Result<Vec<u8>> {
    let path = path.filter(|path| !path.is_empty()).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("{field} is required"))
    })?;
    fs::read(path).map_err(|error| {
        io::Error::new(error.kind(), format!("failed to read {field} ({path}): {error}"))
    })
}
