use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use server_conf::JwtConf;

#[derive(Deserialize)]
#[serde(untagged)]
enum Audience {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Deserialize)]
pub(super) struct StandardClaims {
    sub: String,
    iss: String,
    aud: Audience,
    exp: f64,
}

impl StandardClaims {
    pub(super) fn validate(
        &self,
        config: Option<&JwtConf>,
        now: SystemTime,
    ) -> Result<(), &'static str> {
        let required_claims_present = !self.sub.is_empty()
            && !self.iss.is_empty()
            && match &self.aud {
                Audience::Single(audience) => !audience.is_empty(),
                Audience::Multiple(audiences) => {
                    !audiences.is_empty() && audiences.iter().all(|audience| !audience.is_empty())
                }
            };
        if !required_claims_present {
            return Err("empty_required_claim");
        }
        if let Some(config) = config {
            if self.iss != config.iss {
                return Err("issuer_mismatch");
            }
            if let Some(expected) = &config.aud {
                let matches = match &self.aud {
                    Audience::Single(audience) => audience == expected,
                    Audience::Multiple(audiences) => audiences.contains(expected),
                };
                if !matches {
                    return Err("audience_mismatch");
                }
            }
        }
        let now = now
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "invalid_system_time")?
            .as_secs_f64();
        if now >= self.exp {
            return Err("token_expired");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_claims_must_match_exactly() {
        let mut config = JwtConf { iss: "issuer".into(), aud: Some("service".into()) };
        for audience in [r#""service""#, r#"["other","service"]"#] {
            let claims: StandardClaims = serde_json::from_str(&format!(
                r#"{{"sub":"user","iss":"issuer","aud":{audience},"exp":100}}"#
            )).unwrap();
            assert_eq!(claims.validate(None, UNIX_EPOCH), Ok(()));
            assert_eq!(claims.validate(Some(&config), UNIX_EPOCH), Ok(()));
        }

        let claims: StandardClaims =
            serde_json::from_str(r#"{"sub":"user","iss":"issuer","aud":["Service"],"exp":100}"#).unwrap();
        assert_eq!(claims.validate(Some(&config), UNIX_EPOCH), Err("audience_mismatch"));
        config.aud = None;
        assert_eq!(claims.validate(Some(&config), UNIX_EPOCH), Ok(()));
        config.iss = "Issuer".into();
        assert_eq!(claims.validate(Some(&config), UNIX_EPOCH), Err("issuer_mismatch"));
    }

    #[test]
    fn expiration_rejects_at_or_after_the_deadline() {
        use std::time::Duration;

        let claims: StandardClaims =
            serde_json::from_str(r#"{"sub":"user","iss":"issuer","aud":"service","exp":100.5}"#)
                .unwrap();
        let at = |millis| UNIX_EPOCH + Duration::from_millis(millis);
        assert_eq!(claims.validate(None, at(100_499)), Ok(()));
        assert_eq!(claims.validate(None, at(100_500)), Err("token_expired"));
        assert_eq!(claims.validate(None, at(100_501)), Err("token_expired"));

        for exp in ["null", r#""100.5""#] {
            let payload = format!(r#"{{"sub":"user","iss":"issuer","aud":"service","exp":{exp}}}"#);
            assert!(serde_json::from_str::<StandardClaims>(&payload).is_err());
        }
    }
}
