use std::net::SocketAddr;
use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

mod schema;
pub use schema::{
    BackendConfig,
    ServerConf, DbConf, RedisConf, MailConf,
    load_config_source,
};

pub static SV_CONF: LazyLock<BackendConfig> = LazyLock::new(|| BackendConfig::new());

/// listen IP & port. Default "[::]:50051" for gRPC.
pub static SERVER_BIND: LazyLock<SocketAddr> = LazyLock::new(|| {
    format!("{}:{}",
            SV_CONF.listen.host,
            SV_CONF.listen.port.to_string())
        .parse().unwrap()
});

pub fn abs_path(path: &str) -> String {
    abs_path_with_default(path, "")
}

#[inline]
pub fn abs_path_with_default(path: &str, default_basedir: &str) -> String {
    let basedir = SV_CONF.listen.basedir.as_ref().map(|s| s.as_str())
        .or(Some(default_basedir))
        .and_then(|s| capture_path(RE_BASEDIR, s));
    let path = capture_path(RE_PATH, path)
        .unwrap_or("");
    match basedir {
        Some(basedir) => format!("/{basedir}/{path}"),
        None => format!("/{path}")
    }
}

const RE_BASEDIR: &str = r"^/*([^/].*?)/*$";
const RE_PATH: &str = r"^/*([^/].*)$";

fn capture_path<'a>(regex: &str, path: &'a str) -> Option<&'a str> {
    let re = Regex::new(regex).unwrap();
    re.captures(path)
        .and_then(|cap| cap.get(1).map(|m| m.as_str()))
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
        assert_eq!(SV_CONF.listen.host, "[::]");
        assert_eq!(SV_CONF.listen.port, 50051);
        assert_eq!(SV_CONF.listen.basedir, None);
        assert_eq!(SV_CONF.listen.origin, None);
        assert_eq!(SV_CONF.db.host, "localhost");
        assert_eq!(SV_CONF.db.port, 5432);

        assert_eq!(*SERVER_BIND, "[::]:50051".parse().unwrap());
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

    #[test]
    fn it_returns_dir_as_abs_path() {
        assert_eq!(abs_path("dir"), "/dir");
        assert_eq!(abs_path("/dir"), "/dir");
        assert_eq!(abs_path("/"), "/");
        assert_eq!(abs_path(""), "/");
    }

    #[test]
    fn it_returns_dir_with_default_prefix() {
        assert_eq!(abs_path_with_default("dir", "base"), "/base/dir");
        assert_eq!(abs_path_with_default("/dir", "base"), "/base/dir");
        assert_eq!(abs_path_with_default("dir", "/base"), "/base/dir");
        assert_eq!(abs_path_with_default("dir", "/base/"), "/base/dir");
        assert_eq!(abs_path_with_default("/", "base"), "/base/");
        assert_eq!(abs_path_with_default("", "base"), "/base/");
    }

    #[test]
    fn it_captures_dir_fragment() {
        assert_eq!(capture_path(RE_BASEDIR, "dir"), Some("dir"));
        assert_eq!(capture_path(RE_PATH, "dir"), Some("dir"));
        assert_eq!(capture_path(RE_BASEDIR, "/dir/"), Some("dir"));
        assert_eq!(capture_path(RE_PATH, "/dir/"), Some("dir/"));
        assert_eq!(capture_path(RE_BASEDIR, "//dir//"), Some("dir"));
        assert_eq!(capture_path(RE_PATH, "//dir//"), Some("dir//"));
        assert_eq!(capture_path(RE_BASEDIR, "dir/"), Some("dir"));
        assert_eq!(capture_path(RE_PATH, "dir/"), Some("dir/"));
        assert_eq!(capture_path(RE_BASEDIR, ""), None);
        assert_eq!(capture_path(RE_PATH, ""), None);
    }
}
