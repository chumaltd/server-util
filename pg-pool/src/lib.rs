pub mod pg;
pub mod pgr;

/// RAII guard that closes [`PG_POOL`] and [`PGR_POOL`] on drop.
///
/// Obtain via [`pool_guard()`]. Typical usage:
/// ```ignore
/// let _guard = pg_pool::pool_guard();
/// // ... use pools ...
/// // pools are closed when _guard goes out of scope
/// ```
pub struct PoolGuard;

/// Returns a [`PoolGuard`] that closes both pools when dropped.
pub fn pool_guard() -> PoolGuard {
    PoolGuard
}

impl Drop for PoolGuard {
    fn drop(&mut self) {
        pg::close();
        pgr::close();
    }
}

pub use deadpool_postgres::{
    Pool, PoolError,
    tokio_postgres::{Error, Row, Statement, types::Type}
};

use log::error;
use deadpool_postgres::{
    Config, Hook, HookError, ManagerConfig, RecyclingMethod, Runtime, Timeouts,
    tokio_postgres::NoTls,
};
use server_conf::{SV_CONF, DbConf};
use std::sync::LazyLock;
use std::time::Duration;
mod driver;

pub static PG_POOL: LazyLock<Pool> = LazyLock::new(|| create_pool(&SV_CONF.db, "w").unwrap());

// Connection pool for read replica
pub static PGR_POOL: LazyLock<Option<Pool>> = LazyLock::new(|| {
    match &SV_CONF.dbr {
        Some(dbr) => Some(create_pool(dbr, "r").unwrap()),
        None => None
    }
});

pub fn create_pool(db: &DbConf, role: &str) -> Result<Pool, String> {
    let pool_max: usize = db.pool_max.unwrap_or(1);
    let create_timeout = match SV_CONF.dbr.as_ref().map(|dbr| dbr.fallback).unwrap_or(false) {
        true => 1000,   // reader with fallback: fail faster
        false => 3000,
    };
    let timeouts = timeouts_object(db.timeout.unwrap_or(500), create_timeout, 1500);

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
    cfg.application_name = db.application_name.as_deref().map(|n| format!("{n}-{role}"));
    cfg.keepalives = Some(true);
    cfg.keepalives_idle = Some(Duration::from_secs(60));
    // NOTE: Runtime is also configurable.
    cfg.manager = Some(ManagerConfig { recycling_method: RecyclingMethod::Verified });
    let mut builder = cfg.builder(NoTls)
        .map_err(|e| {
            error!("{} {:?}", e, SV_CONF.db);
            format!("Cannot process pg config: {e}")
        })?
        .max_size(pool_max)
        .timeouts(timeouts)
        .runtime(Runtime::Tokio1);

    if let Some(schema) = db.schema.clone() {
        builder = builder.post_create(Hook::async_fn(move |client, _| {
            let schema = schema.clone();
            Box::pin(async move {
                client.simple_query(&format!("SET search_path TO \"{schema}\""))
                    .await
                    .map(|_| ())
                    .map_err(HookError::Backend)
            })
        }));
    }

    builder.build().map_err(|e| {
        error!("{e}");
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
