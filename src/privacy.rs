use crate::attribute_policy::{ValueDisposition, classify};
use crate::{Attribute, TelemetryError};
use std::collections::HashSet;

const ALLOWED_KEYS: &[&str] = &[
    "component",
    "dependency_id",
    "endpoint_id",
    "environment",
    "operation",
    "protocol",
    "reason_code",
    "region",
    "runtime",
    "version",
];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrivacyReport {
    pub accepted: usize,
    pub dropped: usize,
    pub redacted: usize,
}

/// Applies the closed persistence allow-list before any database write.
///
/// # Errors
/// Returns an error when the caller exceeds the bounded input cardinality.
pub fn sanitize_attributes(
    input: &[Attribute],
) -> Result<(Vec<Attribute>, PrivacyReport), TelemetryError> {
    if input.len() > 32 {
        return Err(TelemetryError::InvalidInput(
            "at most 32 attributes are accepted".into(),
        ));
    }
    let allowed: HashSet<&str> = ALLOWED_KEYS.iter().copied().collect();
    let mut output = Vec::new();
    let mut report = PrivacyReport::default();
    for attribute in input {
        if !allowed.contains(attribute.key.as_str())
            || output.iter().any(|a: &Attribute| a.key == attribute.key)
        {
            report.dropped += 1;
            continue;
        }
        match classify(&attribute.key, &attribute.value) {
            ValueDisposition::Accepted => {}
            ValueDisposition::Invalid => {
                report.dropped += 1;
                continue;
            }
            ValueDisposition::Sensitive => {
                report.dropped += 1;
                report.redacted += 1;
                continue;
            }
        }
        output.push(Attribute {
            key: attribute.key.clone(),
            value: attribute.value.clone(),
        });
        report.accepted += 1;
    }
    Ok((output, report))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_unknown_and_secret_shaped_values() {
        let input = vec![
            Attribute {
                key: "unknown".into(),
                value: "x".into(),
            },
            Attribute {
                key: "operation".into(),
                value: "Bearer abc".into(),
            },
        ];
        let (clean, report) = sanitize_attributes(&input).expect("privacy policy");
        assert!(clean.is_empty());
        assert_eq!((report.dropped, report.redacted), (2, 1));
    }

    #[test]
    fn accepts_only_key_specific_closed_values() {
        let input = vec![
            Attribute {
                key: "environment".into(),
                value: "local".into(),
            },
            Attribute {
                key: "protocol".into(),
                value: "http".into(),
            },
            Attribute {
                key: "runtime".into(),
                value: "rust".into(),
            },
            Attribute {
                key: "version".into(),
                value: "1.2.3-rc.1".into(),
            },
            Attribute {
                key: "endpoint_id".into(),
                value: "https://host/path?token=x".into(),
            },
            Attribute {
                key: "operation".into(),
                value: "github_pat_abcdef012345".into(),
            },
            Attribute {
                key: "region".into(),
                value: "ap-northeast-1&key=value".into(),
            },
        ];
        let (clean, report) = sanitize_attributes(&input).expect("privacy policy");
        assert_eq!(clean.len(), 4);
        assert_eq!(report.accepted, 4);
        assert_eq!(report.dropped, 3);
        assert_eq!(report.redacted, 1);
    }
}
