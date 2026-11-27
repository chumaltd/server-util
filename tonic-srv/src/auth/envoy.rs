use std::time::SystemTime;

use base64::prelude::*;
use serde::de::DeserializeOwned;
use server_conf::SV_CONF;
use tonic::{Extensions, Request, Status, metadata::MetadataMap, service::Interceptor};

use super::claims::StandardClaims;

pub const ENVOY_JWT_PAYLOAD_HEADER: &str = "x-jwt-payload";

/// Read Envoy's `forward_payload_header` with its default unpadded encoding.
///
/// Register this interceptor on services reached through a trusted Envoy that
/// verifies the JWT and controls the payload header. The interceptor checks
/// the required claims, expiration, and any configured JWT
/// issuer and audience before passing the same payload to application code.
pub fn envoy_jwt_interceptor<C, F>(check_attributes: F) -> impl Interceptor
where
    C: DeserializeOwned,
    F: Fn(C, &mut Extensions) -> Result<(), Status>,
{
    move |mut request: Request<()>| {
        let payload = envoy_payload(request.metadata())?;
        if payload.iter().copied().find(|byte| !byte.is_ascii_whitespace()) != Some(b'{') {
            return Err(authentication_failed("claims_not_object"));
        }
        let standard: StandardClaims = serde_json::from_slice(&payload)
            .map_err(|_| authentication_failed("invalid_standard_claims"))?;
        standard
            .validate(SV_CONF.jwt.as_ref(), SystemTime::now())
            .map_err(authentication_failed)?;
        let claims = serde_json::from_slice::<C>(&payload)
            .map_err(|_| authentication_failed("invalid_application_claims"))?;
        check_attributes(claims, request.extensions_mut())?;
        Ok(request)
    }
}

fn envoy_payload(metadata: &MetadataMap) -> Result<Vec<u8>, Status> {
    let values = metadata.get_all(ENVOY_JWT_PAYLOAD_HEADER);
    let mut values = values.iter();
    let value = values
        .next()
        .ok_or_else(|| authentication_failed("missing_payload_header"))?;
    if values.next().is_some() {
        return Err(authentication_failed("duplicate_payload_header"));
    }
    BASE64_URL_SAFE_NO_PAD.decode(value.as_encoded_bytes())
        .map_err(|_| authentication_failed("invalid_payload_encoding"))
}

fn authentication_failed(reason: &'static str) -> Status {
    let level = if reason == "token_expired" {
        log::Level::Info
    } else {
        log::Level::Warn
    };
    log::log!(level, "Envoy JWT payload rejected: reason={reason}");
    Status::unauthenticated("")
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use super::super::bearer_token;

    fn request_with_payload(json: &str) -> Request<()> {
        let mut request = Request::new(());
        request.metadata_mut().insert(
            ENVOY_JWT_PAYLOAD_HEADER,
            BASE64_URL_SAFE_NO_PAD.encode(json).parse().unwrap(),
        );
        request
    }

    #[test]
    fn envoy_payload_is_the_input_to_standard_and_application_claims() {
        #[derive(Deserialize)]
        struct ApplicationClaims {
            sub: String,
        }

        let mut interceptor = envoy_jwt_interceptor(|claims: ApplicationClaims, extensions| {
            extensions.insert(claims.sub);
            Ok(())
        });
        let mut request =
            request_with_payload(r#"{"sub":"envoy-user","iss":"issuer","aud":["service"],"exp":4102444800}"#);
        request.metadata_mut().insert("authorization", "Bearer a.b.c".parse().unwrap());
        let request = interceptor.call(request).unwrap();
        assert_eq!(request.extensions().get::<String>().unwrap(), "envoy-user");
        assert_eq!(bearer_token(&request), Ok(Some("a.b.c")));
    }

    #[test]
    fn invalid_envoy_payload_never_reaches_application() {
        let mut interceptor = envoy_jwt_interceptor(|_: serde::de::IgnoredAny, _| {
            panic!("invalid claims reached the application");
        });
        let mut bearer_only = Request::new(());
        bearer_only.metadata_mut().insert("authorization", "Bearer a.b.c".parse().unwrap());
        let mut duplicate = request_with_payload(r#"{"sub":"user","iss":"issuer","aud":"service","exp":4102444800}"#);
        duplicate.metadata_mut().append(ENVOY_JWT_PAYLOAD_HEADER, "e30".parse().unwrap());
        let missing_audience = request_with_payload(r#"{"sub":"user","iss":"issuer","exp":4102444800}"#);
        let duplicate_issuer =
            request_with_payload(r#"{"sub":"user","iss":"a","iss":"b","aud":"service","exp":4102444800}"#);
        let missing_expiration =
            request_with_payload(r#"{"sub":"user","iss":"issuer","aud":"service"}"#);
        let expired =
            request_with_payload(r#"{"sub":"user","iss":"issuer","aud":"service","exp":0}"#);
        let array = request_with_payload(r#"["user","issuer","service"]"#);

        let mut invalid_encoding = Request::new(());
        invalid_encoding.metadata_mut().insert(
            ENVOY_JWT_PAYLOAD_HEADER,
            "!".parse().unwrap(),
        );

        for request in [
            bearer_only,
            duplicate,
            missing_audience,
            duplicate_issuer,
            missing_expiration,
            expired,
            array,
            invalid_encoding,
        ] {
            let status = interceptor.call(request).unwrap_err();
            assert_eq!(status.code(), tonic::Code::Unauthenticated);
            assert!(status.message().is_empty());
        }
    }
}
