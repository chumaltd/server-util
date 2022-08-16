use config::{Config, ConfigError, Environment, File};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::env;

const CONFIG_FILE_PATH: &str = "./config/default";

pub static SV_CONF: Lazy<BackendConfig> = Lazy::new(|| BackendConfig::new());

/// listen IP & port. Default "0.0.0.0:50051" for gRPC.
pub static SERVER_BIND: Lazy<std::net::SocketAddr> = Lazy::new(|| {
    format!("{}:{}", SV_CONF.listen.host,
            SV_CONF.listen.port.to_string())
        .parse().unwrap()
});

#[derive(Debug, Deserialize, Clone)]
pub struct BackendConfig {
    pub listen: ServerConf,
    pub db: DbConf,
    pub redis: Option<RedisConf>,
    pub mail: Option<MailConf>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConf {
    pub host: String,
    pub port: u16,
    pub domain: String,
    pub origin: Option<String> // exposed origin name
}

#[derive(Debug, Deserialize, Clone)]
pub struct DbConf {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedisConf {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MailConf {
    pub api_key: String,
    pub from: String,
    pub admin_addr: String
}

impl BackendConfig {
    pub fn new() -> Self {
        let env = env::var("RUST_CONF_ENV").unwrap_or_else(|_| "test".into());
        let s = Config::builder()
            .set_default("listen.host", "0.0.0.0".to_string()).unwrap()
            .set_default("listen.port", 50051i64).unwrap()
            .set_default("listen.domain", "".to_string()).unwrap()
            .set_default("db.host", "localhost".to_string()).unwrap()
            .set_default("db.port", 5432i64).unwrap()
            .add_source(File::with_name(CONFIG_FILE_PATH).required(false))
            .add_source(File::with_name(&format!("./config/{}", env)).required(false))
            .add_source(Environment::with_prefix("sv_").separator("__"))
            .build().unwrap();
        s.try_deserialize().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_returns_default_config() {
        assert_eq!(SV_CONF.listen.host, "0.0.0.0");
        assert_eq!(SV_CONF.listen.port, 50051);
        assert_eq!(SV_CONF.db.host, "localhost");
        assert_eq!(SV_CONF.db.port, 5432);

        assert_eq!(*SERVER_BIND, "0.0.0.0:50051".parse().unwrap());
    }

    #[test]
    fn it_loads_test_toml() {
        assert_eq!(SV_CONF.db.name, "some_database");
        assert_eq!(SV_CONF.db.user, "some_user");
        assert_eq!(SV_CONF.db.password, "some_password");
    }
}
