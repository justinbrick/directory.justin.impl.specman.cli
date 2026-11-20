use specman::GitReference;

use crate::error::{CliError, ExitStatus};

/// Logging/output preferences captured once per invocation.
#[derive(Clone, Copy, Debug, Default)]
pub struct Verbosity {
    pub json: bool,
    pub verbose: bool,
}

/// Parses an `alias=url@ref` dependency token into a validated [`GitReference`].
///
/// The `alias` part must be a portable filename segment, the `url` must be an SSH or HTTPS
/// git URL, and the `ref` must be a full commit hash or a git tag (branches are never consulted).
/// See spec/specman-cli/spec.md#concept-lifecycle-command-surface and
/// spec/specman-core/spec.md#entity-gitreference.
pub fn parse_reference(token: &str, field: &str) -> Result<GitReference, CliError> {
    let trimmed = token.trim();
    let usage = |msg: String| CliError::new(msg, ExitStatus::Usage);

    let (alias, rest) = trimmed
        .split_once('=')
        .ok_or_else(|| usage(format!("{field} entry '{trimmed}' must be `alias=url@ref`")))?;
    let alias = alias.trim();
    let rest = rest.trim();

    if alias.is_empty() {
        return Err(usage(format!(
            "{field} entry '{trimmed}' has an empty alias"
        )));
    }
    if !is_portable_alias(alias) {
        return Err(usage(format!(
            "{field} alias '{alias}' must match ^[A-Za-z0-9][A-Za-z0-9._-]*$"
        )));
    }

    let (url, reference) = rest
        .rsplit_once('@')
        .ok_or_else(|| usage(format!("{field} entry '{trimmed}' must be `alias=url@ref`")))?;
    let url = url.trim();
    let reference = reference.trim();

    if url.is_empty() || reference.is_empty() {
        return Err(usage(format!(
            "{field} entry '{trimmed}' must have non-empty url and ref"
        )));
    }

    parse_reference_parts(alias, url, reference, field)
}

/// Parses an override `site=<site> ref=<commit-or-tag>` into its `(site, ref)` parts.
pub fn parse_override(token: &str, field: &str) -> Result<(String, String), CliError> {
    let trimmed = token.trim();
    let usage = |msg: String| CliError::new(msg, ExitStatus::Usage);

    let mut site: Option<String> = None;
    let mut reference: Option<String> = None;
    for part in trimmed.split_whitespace() {
        if let Some(value) = part.strip_prefix("site=") {
            site = Some(value.to_string());
        } else if let Some(value) = part.strip_prefix("ref=") {
            reference = Some(value.to_string());
        }
    }

    let site =
        site.ok_or_else(|| usage(format!("{field} entry '{trimmed}' is missing `site=`")))?;
    let reference =
        reference.ok_or_else(|| usage(format!("{field} entry '{trimmed}' is missing `ref=`")))?;

    if site.is_empty() || reference.is_empty() {
        return Err(usage(format!(
            "{field} entry '{trimmed}' has empty site or ref"
        )));
    }

    Ok((site, reference))
}

/// Validates `alias`, `url`, and `ref` directly, constructing the typed [`GitReference`].
pub(crate) fn parse_reference_parts(
    alias: &str,
    url: &str,
    reference: &str,
    field: &str,
) -> Result<GitReference, CliError> {
    let usage = |msg: String| CliError::new(msg, ExitStatus::Usage);

    let alias = alias
        .parse::<specman::GitReferenceAlias>()
        .map_err(|e| usage(format!("{field} alias '{alias}' is invalid: {e}")))?;
    let url = url
        .parse::<specman::GitReferenceUrl>()
        .map_err(|e| usage(format!("{field} url '{url}' is invalid: {e}")))?;

    // `ref` must be a full 40-char commit hash or a git tag (branches are never consulted).
    let is_hash = reference.len() == 40 && reference.chars().all(|c| c.is_ascii_hexdigit());
    if !is_hash {
        // Validate a plausible tag name (non-empty, no whitespace, no reserved traversal).
        if reference.is_empty()
            || reference.contains(char::is_whitespace)
            || reference.starts_with('-')
            || reference.contains("..")
        {
            return Err(usage(format!(
                "{field} ref '{reference}' must be a full commit hash or a valid git tag"
            )));
        }
    }

    Ok(GitReference {
        alias,
        ref_: reference.to_string(),
        url,
    })
}

/// Whether `s` is a portable filename segment matching `^[A-Za-z0-9][A-Za-z0-9._-]*$`.
fn is_portable_alias(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphanumeric() => {
            chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        }
        _ => false,
    }
}

/// Splits a comma-separated list into trimmed, non-empty tokens.
pub fn split_csv(value: Option<&String>) -> Vec<String> {
    value
        .map(|v| {
            v.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_http_reference_with_tag() {
        let reference = parse_reference(
            "core=https://github.com/owner/repo.git@v1.0.0",
            "dependency",
        )
        .unwrap();
        assert_eq!(reference.alias.as_str(), "core");
        assert_eq!(reference.url.as_str(), "https://github.com/owner/repo.git");
        assert_eq!(reference.ref_, "v1.0.0");
    }

    #[test]
    fn parses_ssh_reference_with_hash() {
        let hash = "a".repeat(40);
        let reference =
            parse_reference(&format!("core=git@github.com:owner/repo.git@{hash}"), "dep").unwrap();
        assert_eq!(reference.ref_, hash);
    }

    #[test]
    fn rejects_missing_equals() {
        let err = parse_reference("noequals", "dependency").unwrap_err();
        assert_eq!(err.status(), ExitStatus::Usage);
    }

    #[test]
    fn rejects_unportable_alias() {
        let err = parse_reference("../bad=https://x/y.git@v1", "dependency").unwrap_err();
        assert_eq!(err.status(), ExitStatus::Usage);
    }

    #[test]
    fn parses_override_token() {
        let (site, reference) =
            parse_override("site=github.com/owner/repo ref=v2.0.0", "override").unwrap();
        assert_eq!(site, "github.com/owner/repo");
        assert_eq!(reference, "v2.0.0");
    }

    #[test]
    fn split_csv_handles_whitespace() {
        assert_eq!(
            split_csv(Some(&"a, b ,c,".to_string())),
            vec!["a", "b", "c"]
        );
    }
}
