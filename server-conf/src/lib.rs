use config::{Config, ConfigError, Environment, File};
use once_cell::sync::Lazy;
use serde::Deserialize;

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
        let mut s = Config::new();
        s.set_default("listen.host", "0.0.0.0".to_string()).unwrap();
        s.set_default("listen.port", 50051).unwrap();
        s.set_default("listen.domain", "".to_string()).unwrap();
        s.set_default("db.host", "localhost".to_string()).unwrap();
        s.set_default("db.port", 5432).unwrap();
        s.merge(File::with_name(CONFIG_FILE_PATH).required(false)).unwrap();
        if cfg!(test) {
            s.merge(File::with_name("./config/test").required(false)).unwrap();
        } else if cfg!(debug_assertions) {
            s.merge(File::with_name("./config/development").required(false)).unwrap();
        }
        s.merge(Environment::with_prefix("sv_").separator("__")).unwrap();
        s.try_into().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::set_var;

    #[test]
    fn it_returns_default_config() {
        set_var("sv__db__name", "some_database");
        set_var("sv__db__user", "some_user");
        set_var("sv__db__password", "some_password");
        assert_eq!(SV_CONF.listen.host, "0.0.0.0");
        assert_eq!(SV_CONF.listen.port, 50051);
        assert_eq!(SV_CONF.db.host, "localhost");
        assert_eq!(SV_CONF.db.port, 5432);

        assert_eq!(*SERVER_BIND, "0.0.0.0:50051".parse().unwrap());
    }
}
