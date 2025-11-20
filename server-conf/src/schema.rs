use config::{Config, ConfigBuilder, Environment, File, builder::DefaultState};
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

impl BackendConfig {
    pub fn new() -> Self {
        load_config_source()
            .build().unwrap()
            .try_deserialize().unwrap()
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ServerConf {
    pub host: String,
    pub port: u16,
    pub domain: String,
    pub basedir: Option<String>,   // for deployment under a subpath
    pub origin: Option<String>     // exposed origin name
}

impl Default for ServerConf {
    fn default() -> Self {
        Self {
            host: "[::]".into(),
            port: 50051,
            domain: "".into(),
            basedir: None,
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
    pub api_host: String,
    pub api_port: Option<u16>,
    pub api_user: Option<String>,
    pub api_key: Option<String>,
}

pub fn load_config_source() -> ConfigBuilder<DefaultState> {
    let env = env::var("RUST_CONF_ENV").unwrap_or_else(|_| "test".into());
    Config::builder()
        .add_source(File::with_name(CONFIG_FILE_PATH).required(false))
        .add_source(File::with_name(&format!("./config/{env}", )).required(false))
        .add_source(Environment::with_prefix("sv_").separator("__"))
}
