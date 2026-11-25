use config::{Config, ConfigBuilder, ConfigError, Environment, File, builder::DefaultState};
use serde::Deserialize;
use std::env;

const CONFIG_FILE_PATH: &str = "./config/default";

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct BackendConfig {
    pub listen: ServerConf,
    pub db: DbConf,
    pub dbr: Option<DbConf>,      // for Read Replica connection
    pub redis: Option<RedisConf>,
    pub mail: Option<MailConf>,
    pub mtls: Option<MtlsConf>,
    pub otel: Option<OtelConf>,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            listen: ServerConf::default(),
            db: DbConf::default(),
            dbr: None,
            redis: None,
            mail: None,
            mtls: None,
            otel: None,
        }
    }
}

impl BackendConfig {
    pub fn new() -> Self {
        Self::from_config(load_config_source().build().unwrap()).unwrap()
    }

    fn from_config(source: Config) -> Result<Self, ConfigError> {
        let mut config: Self = source.try_deserialize()?;
        config.apply_mtls_fallback();
        if let Some(tls) = &config.listen.tls {
            if tls.key.is_none() && (tls.cert.is_some() || tls.ca.is_some()) {
                return Err(ConfigError::Message(
                    "listen.tls.key is required when listen.tls.cert or listen.tls.ca is set".into(),
                ));
            }
        }
        Ok(config)
    }

    fn apply_mtls_fallback(&mut self) {
        let Some(mtls) = &self.mtls else {
            return;
        };

        mtls.fill_tls(&mut self.listen.tls);
        mtls.fill_tls(&mut self.db.tls);
        if let Some(dbr) = &mut self.dbr {
            mtls.fill_tls(&mut dbr.tls);
        }
        if let Some(redis) = &mut self.redis {
            mtls.fill_tls(&mut redis.tls);
        }
        if let Some(mail) = &mut self.mail {
            mtls.fill_tls(&mut mail.tls);
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ServerConf {
    pub host: String,
    pub port: u16,
    pub domain: String,
    pub basedir: Option<String>,   // for deployment under a subpath
    pub origin: Option<String>,    // exposed origin name
    pub tls: Option<TlsConf>,
    pub tls_verify_host: Option<bool>,
}

impl Default for ServerConf {
    fn default() -> Self {
        Self {
            host: "[::]".into(),
            port: 50051,
            domain: "".into(),
            basedir: None,
            origin: None,
            tls: None,
            tls_verify_host: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct OtelConf {
    pub endpoint: String,
    #[serde(default = "default_true")]
    pub traces: bool,
    #[serde(default = "default_true")]
    pub logs: bool,
    #[serde(default = "default_true")]
    pub metrics: bool,
    #[serde(default)]
    pub log_level: OtelLogLevel,
}

#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OtelLogLevel {
    Error,
    #[default]
    Warn,
    Info,
    Debug,
    Trace,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Clone)]
pub struct MtlsConf {
    pub cert: String,
    pub key: String,
    pub ca: String,
}

impl MtlsConf {
    fn fill_tls(&self, tls: &mut Option<TlsConf>) {
        let tls = tls.get_or_insert_with(TlsConf::default);
        tls.cert.get_or_insert_with(|| self.cert.clone());
        tls.key.get_or_insert_with(|| self.key.clone());
        tls.ca.get_or_insert_with(|| self.ca.clone());
    }
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct TlsConf {
    pub cert: Option<String>,
    pub key: Option<String>,
    pub ca: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct DbConf {
    pub name: String,
    pub host: String,
    pub hosts: Option<Vec<String>>,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub pool_max: Option<usize>,  // Max size of connection pool
    pub timeout: Option<u64>,     // Timeout in millisec for getting connection pool
    pub fallback: bool,
    pub application_name: Option<String>,
    pub schema: Option<String>,
    pub tls: Option<TlsConf>,
    pub tls_verify_host: Option<bool>,
}

impl Default for DbConf {
    fn default() -> Self {
        Self {
            name: "".into(),
            host: "localhost".into(),
            hosts: None,
            port: 5432,
            user: "".into(),
            password: "".into(),
            pool_max: None,
            timeout: None,
            fallback: false,
            application_name: None,
            schema: None,
            tls: None,
            tls_verify_host: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedisConf {
    pub url: String,
    pub tls: Option<TlsConf>,
    pub tls_verify_host: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MailConf {
    pub from: String,
    pub admin_addr: String,
    pub api_host: String,
    pub api_port: Option<u16>,
    pub api_user: Option<String>,
    pub api_key: Option<String>,
    pub pool_max: Option<u32>,
    pub pool_idle: Option<u32>,
    pub sender_host: Option<String>,
    pub tls_verify_host: Option<bool>,
    pub tls: Option<TlsConf>,
}

pub fn load_config_source() -> ConfigBuilder<DefaultState> {
    let env = env::var("RUST_CONF_ENV").unwrap_or_else(|_| "test".into());
    Config::builder()
        .add_source(File::with_name(CONFIG_FILE_PATH).required(false))
        .add_source(File::with_name(&format!("./config/{env}", )).required(false))
        .add_source(Environment::with_prefix("sv").separator("__"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::FileFormat;

    #[test]
    fn otel_is_optional_and_signal_flags_default_to_true() {
        fn load(toml: &str) -> Result<BackendConfig, ConfigError> {
            let source = Config::builder()
                .add_source(File::from_str(toml, FileFormat::Toml))
                .build()?;
            BackendConfig::from_config(source)
        }

        assert!(load("").unwrap().otel.is_none());
        assert!(load("[otel]\nlogs = false").is_err());

        let otel = load("[otel]\nendpoint = 'http://localhost:4317'")
            .unwrap()
            .otel
            .unwrap();
        assert_eq!(otel.endpoint, "http://localhost:4317");
        assert!(otel.traces && otel.logs && otel.metrics);
        assert_eq!(otel.log_level, OtelLogLevel::Warn);

        for (name, level) in [
            ("error", OtelLogLevel::Error),
            ("warn", OtelLogLevel::Warn),
            ("info", OtelLogLevel::Info),
            ("debug", OtelLogLevel::Debug),
            ("trace", OtelLogLevel::Trace),
        ] {
            let toml = format!("[otel]\nendpoint = 'http://localhost:4317'\nlog_level = '{name}'");
            assert_eq!(load(&toml).unwrap().otel.unwrap().log_level, level);
        }
        assert!(load("[otel]\nendpoint = 'http://localhost:4317'\nlog_level = 'off'").is_err());

        let otel = load(
            "[otel]\nendpoint = 'http://localhost:4317'\ntraces = false\nlogs = false\nmetrics = false",
        )
        .unwrap()
        .otel
        .unwrap();
        assert!(!otel.traces && !otel.logs && !otel.metrics);
    }

    #[test]
    fn listen_tls_requires_a_key_only_when_cert_or_ca_is_set() {
        for (toml, valid) in [
            ("", true),
            ("[listen.tls]", true),
            ("[listen.tls]\nkey = 'listen.key'", true),
            ("[listen.tls]\ncert = 'listen.crt'\nkey = 'listen.key'", true),
            ("[listen.tls]\ncert = 'listen.crt'", false),
            ("[listen.tls]\nca = 'ca.crt'", false),
        ] {
            let source = Config::builder()
                .add_source(File::from_str(toml, FileFormat::Toml))
                .build()
                .unwrap();
            let result = BackendConfig::from_config(source);
            if valid {
                result.unwrap();
            } else {
                assert!(matches!(result, Err(ConfigError::Message(message))
                    if message.contains("listen.tls.key")));
            }
        }
    }

    #[test]
    fn tls_settings_override_mtls_per_field() {
        let source = Config::builder()
            .add_source(File::from_str(
                r#"
                [mtls]
                cert = "shared.crt"
                key = "shared.key"
                ca = "shared-ca.crt"

                [listen.tls]
                cert = "listen.crt"

                [dbr.tls]
                key = "replica.key"

                [redis]
                url = "redis://localhost"

                [redis.tls]
                ca = "redis-ca.crt"

                [mail]
                from = "sender@example.com"
                admin_addr = "admin@example.com"
                api_host = "mail.example.com"

                [mail.tls]
                cert = "mail.crt"
                "#,
                FileFormat::Toml,
            ))
            .build()
            .unwrap();
        let config = BackendConfig::from_config(source).unwrap();

        let listen = config.listen.tls.unwrap();
        assert_eq!(listen.cert.as_deref(), Some("listen.crt"));
        assert_eq!(listen.key.as_deref(), Some("shared.key"));
        assert_eq!(listen.ca.as_deref(), Some("shared-ca.crt"));

        let db = config.db.tls.unwrap();
        assert_eq!(db.cert.as_deref(), Some("shared.crt"));
        assert_eq!(db.key.as_deref(), Some("shared.key"));
        assert_eq!(db.ca.as_deref(), Some("shared-ca.crt"));

        let replica = config.dbr.unwrap().tls.unwrap();
        assert_eq!(replica.key.as_deref(), Some("replica.key"));
        let redis = config.redis.unwrap().tls.unwrap();
        assert_eq!(redis.ca.as_deref(), Some("redis-ca.crt"));
        let mail = config.mail.unwrap().tls.unwrap();
        assert_eq!(mail.cert.as_deref(), Some("mail.crt"));
        assert_eq!(mail.key.as_deref(), Some("shared.key"));
        assert_eq!(mail.ca.as_deref(), Some("shared-ca.crt"));
    }
}
