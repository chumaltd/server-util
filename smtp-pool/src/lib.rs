use lettre::{
    transport::smtp::{
        self,
        authentication::Credentials,
        client::{Certificate, Identity, Tls, TlsParameters},
        extension::ClientId,
        PoolConfig, SUBMISSIONS_PORT,
    },
    AsyncSmtpTransport, Tokio1Executor,
};
pub use lettre::AsyncTransport;
use log::error;
use server_conf::{SV_CONF, MailConf};
use std::{fs, sync::LazyLock};

pub static SMTP_MAILER: LazyLock<AsyncSmtpTransport<Tokio1Executor>> = LazyLock::new(|| {
    let mail_conf = SV_CONF.mail.as_ref()
        .expect("mail config is required");
    mailer(mail_conf).unwrap()
});

fn mailer(mail_conf: &MailConf) -> Result<AsyncSmtpTransport<Tokio1Executor>, smtp::Error> {
    let tls_params = tls_builder(mail_conf)?;

    let port = match mail_conf.api_port {
        Some(port) => port,
        None => SUBMISSIONS_PORT
    };
    let builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&mail_conf.api_host);
    let builder = builder
        .tls(Tls::Wrapper(tls_params))
        .port(port)
        .pool_config(pool_set(mail_conf));
    let builder = if let (Some(smtp_user), Some(smtp_pass)) = (mail_conf.api_user.as_ref(), mail_conf.api_key.as_ref()) {
        builder.credentials(Credentials::new(
            smtp_user.to_string(),
            smtp_pass.to_string()
        ))
    } else {
        builder
    };
    let builder = if let Some(hello_host) = mail_conf.sender_host.as_ref() {
        builder.hello_name(ClientId::Domain(hello_host.to_string()))
    } else {
        builder
    };
    Ok(builder.build())
}

fn pool_set(mail_conf: &MailConf) -> PoolConfig {
    let config = PoolConfig::new();
    let config = if let Some(max) = mail_conf.pool_max {
        config.max_size(max)
    } else {
        config
    };
    if let Some(timeout) = mail_conf.pool_idle {
        let timeout = std::time::Duration::from_secs(timeout as u64);
        config.idle_timeout(timeout)
    } else {
        config
    }
}

fn tls_builder(mail_conf: &MailConf) -> Result<TlsParameters, smtp::Error> {
    let builder = TlsParameters::builder(mail_conf.api_host.clone());
    let builder = if let Some(cert_path) = mail_conf.tls_root_cert.as_ref() {
        let cert_pem = fs::read(cert_path)
            .expect("Certificate fle cannot read");
        let cert = Certificate::from_pem(&cert_pem)
            .map_err(|e| {
                error!("Certificate not initialized: {e}");
                e
            })?;
        builder.add_root_certificate(cert)
    } else if mail_conf.tls_verify_host == Some(false) {
        builder.dangerous_accept_invalid_certs(true)
    } else {
        builder
    };
    let builder = if let (Some(cert_path), Some(key_path)) = (mail_conf.tls_client_cert.as_ref(), mail_conf.tls_client_key.as_ref()) {
        let client_cert = fs::read(cert_path)
            .expect("ClientCertificate fle cannot read");
        let client_key = fs::read(key_path)
            .expect("ClientKey fle cannot read");
        let identity = Identity::from_pem(&client_cert, &client_key)
            .map_err(|e| {
                error!("ClientCertificate not initialized: {e}");
                e
            })?;
        builder.identify_with(identity)
    } else {
        builder
    };
    builder.build()
        .map_err(|e| {
            error!("tls_builder: {e}");
            e
        })
}
