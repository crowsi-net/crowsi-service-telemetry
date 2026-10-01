#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueDisposition {
    Accepted,
    Invalid,
    Sensitive,
}

pub fn classify(key: &str, value: &str) -> ValueDisposition {
    if sensitive(value) {
        return ValueDisposition::Sensitive;
    }
    if value.is_empty() || value.len() > 128 || transport_shaped(value) {
        return ValueDisposition::Invalid;
    }
    let accepted = match key {
        "environment" => matches!(
            value,
            "local" | "development" | "test" | "staging" | "production"
        ),
        "protocol" => matches!(
            value,
            "http" | "https" | "sse" | "unix" | "sqlite" | "grpc" | "mcp" | "tcp" | "udp"
        ),
        "runtime" => matches!(
            value,
            "rust"
                | "node"
                | "browser"
                | "native"
                | "wsl"
                | "windows"
                | "linux"
                | "container"
                | "external"
        ),
        "version" => semantic_version(value),
        "component" | "dependency_id" | "endpoint_id" | "operation" | "reason_code" | "region" => {
            bounded_identifier(value)
        }
        _ => false,
    };
    if accepted {
        ValueDisposition::Accepted
    } else {
        ValueDisposition::Invalid
    }
}

fn bounded_identifier(value: &str) -> bool {
    value.len() <= 96
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn semantic_version(value: &str) -> bool {
    let value = value.strip_prefix('v').unwrap_or(value);
    let (version, build) = value
        .split_once('+')
        .map_or((value, None), |parts| (parts.0, Some(parts.1)));
    let (core, prerelease) = version
        .split_once('-')
        .map_or((version, None), |parts| (parts.0, Some(parts.1)));
    let mut numbers = core.split('.');
    let numeric = (0..3).all(|_| {
        numbers
            .next()
            .is_some_and(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    }) && numbers.next().is_none();
    numeric && prerelease.is_none_or(suffix) && build.is_none_or(suffix)
}

fn suffix(value: &str) -> bool {
    !value.is_empty()
        && value.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

fn transport_shaped(value: &str) -> bool {
    value.contains("://")
        || value.starts_with(['/', '\\'])
        || value
            .bytes()
            .any(|byte| matches!(byte, b'?' | b'&' | b'=' | b'@' | b'\\'))
}

fn sensitive(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    let markers = [
        "-----begin",
        "authorization:",
        "bearer ",
        "client_secret",
        "private_key",
        "refresh_token",
        "access_token",
        "api_key",
        "password=",
        "github_pat_",
    ];
    markers.iter().any(|marker| normalized.contains(marker))
        || ["ghp_", "gho_", "ghu_", "ghs_", "ghr_", "sk-"]
            .iter()
            .any(|prefix| normalized.starts_with(prefix))
        || (value.len() >= 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
}
