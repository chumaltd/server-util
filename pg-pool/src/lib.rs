pub mod pg;
pub mod pgr;
pub use deadpool_postgres::{
    Pool, PoolError,
    tokio_postgres::{Error, Row, Statement}
};

use log::error;
use once_cell::sync::Lazy;
use deadpool_postgres::{
    Config, ManagerConfig, RecyclingMethod, Runtime, Timeouts,
    tokio_postgres::{NoTls}
};
use server_conf::{SV_CONF, DbConf};
mod driver;

pub static PG_POOL: Lazy<Pool> = Lazy::new(|| create_pool(&SV_CONF.db).unwrap());

// Connection pool for read replica
pub static PGR_POOL: Lazy<Option<Pool>> = Lazy::new(|| {
    match &SV_CONF.dbr {
        Some(dbr) => Some(create_pool(dbr).unwrap()),
        None => None
    }
});

pub fn create_pool(db: &DbConf) -> Result<Pool, String> {
    let pool_max: usize = db.pool_max.unwrap_or(1);
    let timeout_millis = Timeouts::wait_millis(db.timeout.unwrap_or(500));

    let mut cfg = Config::new();
    cfg.dbname = Some(db.name.clone());
    if db.hosts.is_some() {
        cfg.hosts = db.hosts.clone();
    } else {
        cfg.host = Some(db.host.clone());
    }
    cfg.port = Some(db.port);
    cfg.user = Some(db.user.clone());
    cfg.password = Some(db.password.clone());
    // Note: Runtime is also configurable.
    cfg.manager = Some(ManagerConfig { recycling_method: RecyclingMethod::Fast });
    cfg.builder(NoTls)
        .map_err(|e| {
            error!("{}", e);
            "Cannot process pg config".to_string()
        })?
        .max_size(pool_max)
        .timeouts(timeout_millis)
        .runtime(Runtime::Tokio1)
        .build()
        .map_err(|e| {
            error!("{}", e);
            "Cannot build pg pool".to_string()
        })
}
