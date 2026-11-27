use tonic::Request;

#[cfg(feature = "envoy")]
mod claims;

#[cfg(feature = "envoy")]
mod envoy;
#[cfg(feature = "envoy")]
pub use envoy::{ENVOY_JWT_PAYLOAD_HEADER, envoy_jwt_interceptor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BearerTokenError {
    DuplicateAuthorization,
    InvalidAuthorization,
}

impl std::fmt::Display for BearerTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::DuplicateAuthorization => "duplicate authorization metadata",
            Self::InvalidAuthorization => "invalid Bearer authorization metadata",
        })
    }
}

impl std::error::Error for BearerTokenError {}

/// Return the original Bearer token, if present.
pub fn bearer_token<T>(request: &Request<T>) -> Result<Option<&str>, BearerTokenError> {
    let values = request.metadata().get_all("authorization");
    let mut values = values.iter();
    let Some(value) = values.next() else {
        return Ok(None);
    };
    if values.next().is_some() {
        return Err(BearerTokenError::DuplicateAuthorization);
    }

    let value = value.to_str().map_err(|_| BearerTokenError::InvalidAuthorization)?;
    let (scheme, token) = value.split_once(' ').ok_or(BearerTokenError::InvalidAuthorization)?;
    let token = token.trim_start_matches(' ');
    if !scheme.eq_ignore_ascii_case("bearer")
        || token.is_empty()
        || !token.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'+' | b'/' | b'=')
        })
    {
        return Err(BearerTokenError::InvalidAuthorization);
    }
    Ok(Some(token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_is_optional_and_rejects_multiple_values() {
        let mut request = Request::new(());
        assert_eq!(bearer_token(&request), Ok(None));
        request.metadata_mut().append("authorization", "Bearer a.b.c".parse().unwrap());
        request.metadata_mut().append("authorization", "Bearer d.e.f".parse().unwrap());
        assert_eq!(bearer_token(&request), Err(BearerTokenError::DuplicateAuthorization));
    }
}
