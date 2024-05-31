use config::{Config, ConfigBuilder, Environment, File, builder::DefaultState};
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
#[serde(default)]
pub struct BackendConfig {
    pub listen: ServerConf,
    pub db: DbConf,
    pub dbr: Option<DbConf>,      // for Read Replica connection
    pub redis: Option<RedisConf>,
    pub mail: Option<MailConf>,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            listen: ServerConf::default(),
            db: DbConf::default(),
            dbr: None,
            redis: None,
            mail: None
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ServerConf {
    pub host: String,
    pub port: u16,
    pub domain: String,
    pub origin: Option<String>    // exposed origin name
}

impl Default for ServerConf {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 50051,
            domain: "".into(),
            origin: None
        }
    }
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
    pub fallback: bool
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
            fallback: false
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedisConf {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MailConf {
    pub from: String,
    pub admin_addr: String,
    pub api_key: Option<String>,
}

impl BackendConfig {
    pub fn new() -> Self {
        load_config_source()
            .build().unwrap()
            .try_deserialize().unwrap()
    }
}

pub fn load_config_source() -> ConfigBuilder<DefaultState> {
    let env = env::var("RUST_CONF_ENV").unwrap_or_else(|_| "test".into());
    Config::builder()
        .add_source(File::with_name(CONFIG_FILE_PATH).required(false))
        .add_source(File::with_name(&format!("./config/{}", env)).required(false))
        .add_source(Environment::with_prefix("sv_").separator("__"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize)]
    struct ExtConf {
        pub ext: Params
    }

    #[derive(Debug, Deserialize)]
    struct Params {
        pub param: String,
    }

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

    #[test]
    fn it_builds_extra_fields() {
        let s = load_config_source().build().unwrap();
        let conf: ExtConf = s.try_deserialize().unwrap();
        assert_eq!(conf.ext.param, "some_parameter");
    }
}
