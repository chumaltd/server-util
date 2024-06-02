pub mod pg;
pub mod pgr;
pub use deadpool_postgres::{
    Pool, PoolError,
    tokio_postgres::{Error, Row, Statement, types::Type}
};

use log::error;
use once_cell::sync::Lazy;
use deadpool_postgres::{
    Config, ManagerConfig, RecyclingMethod, Runtime, Timeouts,
    tokio_postgres::{NoTls}
};
use server_conf::{SV_CONF, DbConf};
use std::time::Duration;
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
    let timeouts = match SV_CONF.dbr.as_ref().map(|dbr| dbr.fallback ).unwrap_or(false) {
        true => timeouts_object(db.timeout.unwrap_or(500), 900, 1500),
        false => Timeouts::wait_millis(db.timeout.unwrap_or(500))
    };

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
    // NOTE: Runtime is also configurable.
    cfg.manager = Some(ManagerConfig { recycling_method: RecyclingMethod::Fast });
    cfg.builder(NoTls)
        .map_err(|e| {
            error!("{} {:?}", e,  SV_CONF.db);
            format!("Cannot process pg config: {e}")
        })?
        .max_size(pool_max)
        .timeouts(timeouts)
        .runtime(Runtime::Tokio1)
        .build()
        .map_err(|e| {
            error!("{}", e);
            format!("Cannot build pg pool: {e}")
        })
}

fn timeouts_object(wait: u64, create: u64, recycle: u64) -> Timeouts {
    let mut timeouts = Timeouts::new();
    timeouts.wait = Some(Duration::from_millis(wait));
    timeouts.create = Some(Duration::from_millis(create));
    timeouts.recycle = Some(Duration::from_millis(recycle));

    timeouts
}
