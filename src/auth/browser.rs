use super::{AuthToken, RawResult};
use crate::client::Client;
use crate::error::{Error, Result};
use crate::parse::ProcessedResult;
use crate::utils;
use crate::utils::constants::{USER_AGENT, YTM_URL};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt::Debug;
use std::path::Path;

#[derive(Clone, Serialize, Deserialize)]
pub struct BrowserToken {
    sapisid: String,
    client_version: String,
    cookies: String,
}

impl AuthToken for BrowserToken {
    fn client_version(&self) -> Cow<'_, str> {
        (&self.client_version).into()
    }
    fn deserialize_response<Q>(
        raw: RawResult<Q, Self>,
    ) -> Result<crate::parse::ProcessedResult<Q>> {
        let processed = ProcessedResult::try_from(raw)?;
        // Guard against error codes in json response.
        // TODO: Check for a response the reflects an expired Headers token
        // TODO: Add a test for this
        if let Some(error) = processed.get_json().pointer("/error") {
            let Some(code) = error.pointer("/code").and_then(|v| v.as_u64()) else {
                // TODO: Better error.
                return Err(Error::response("API reported an error but no code"));
            };
            let message = error
                .pointer("/message")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();
            return Err(Error::other_code(code, message));
        }
        Ok(processed)
    }
    fn headers(&self) -> Result<impl IntoIterator<Item = (&str, Cow<'_, str>)>> {
        let hash = utils::hash_sapisid(&self.sapisid);
        Ok([
            ("User-Agent", USER_AGENT.into()),
            ("X-Origin", YTM_URL.into()),
            ("Origin", YTM_URL.into()),
            ("Content-Type", "application/json".into()),
            ("Authorization", format!("SAPISIDHASH {hash}").into()),
            ("Cookie", self.cookies.as_str().into()),
            ("Accept", "*/*".into()),
            ("Accept-Encoding", "gzip, deflate".into()),
        ])
    }
}

impl BrowserToken {
    pub async fn from_str(cookie_str: &str, client: &Client) -> Result<Self> {
        let cookies = cookie_str.trim().to_string();
        let user_agent = USER_AGENT;
        // TODO: Confirm if parsing for expired user agent also relevant here.
        let initial_headers = [
            ("User-Agent", user_agent.into()),
            ("Cookie", cookies.as_str().into()),
        ];
        let response_text = client.get_query(YTM_URL, initial_headers, &()).await?.text;
        // parse for user agent issues here.
        if response_text.contains("Sorry, YouTube Music is not optimised for your browser. Check for updates or try Google Chrome.") {
            return Err(Error::invalid_user_agent(user_agent));
        };
        // TODO: Better error.
        let client_version = response_text
            .split_once("INNERTUBE_CLIENT_VERSION\":\"")
            .ok_or(Error::header())?
            .1
            .split_once('\"')
            .ok_or(Error::header())?
            .0
            .to_string();
        // Try SAPISID first, fallback to __Secure-3PAPISID (same value, secure version)
        let sapisid = extract_sapisid(&cookies)?;
        Ok(Self {
            sapisid,
            client_version,
            cookies,
        })
    }
    pub async fn from_cookie_file<P>(path: P, client: &Client) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let contents = tokio::fs::read_to_string(path).await?;
        let cookies = normalize_cookie_file_contents(&contents)?;
        BrowserToken::from_str(&cookies, client).await
    }
}

fn normalize_cookie_file_contents(contents: &str) -> Result<String> {
    if looks_like_netscape_cookie_jar(contents) {
        parse_netscape_cookie_jar(contents)
    } else {
        Ok(contents.trim().to_string())
    }
}

fn looks_like_netscape_cookie_jar(contents: &str) -> bool {
    contents
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .is_some_and(|line| line.starts_with("# Netscape HTTP Cookie File"))
}

fn parse_netscape_cookie_jar(contents: &str) -> Result<String> {
    let cookies = contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with('#') || line.starts_with("#HttpOnly_"))
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let (_domain, _include_subdomains, _path, _secure, _expires, name, value) = (
                fields.next()?,
                fields.next()?,
                fields.next()?,
                fields.next()?,
                fields.next()?,
                fields.next()?,
                fields.next()?,
            );
            Some(format!("{}={}", name.trim(), value.trim()))
        })
        .collect::<Vec<_>>();

    if cookies.is_empty() {
        return Err(Error::header());
    }

    Ok(cookies.join("; "))
}

// Don't use default Debug implementation for BrowserToken - contents are
// private
impl Debug for BrowserToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Private BrowserToken")
    }
}

/// Extract SAPISID from cookie string.
/// Tries SAPISID first, falls back to __Secure-3PAPISID (same value, secure version).
/// Returns the extracted value or Error::header() if neither found.
pub(crate) fn extract_sapisid(cookies: &str) -> Result<String> {
    // Parse all cookies into a map for deterministic lookup
    let cookie_map: std::collections::HashMap<&str, &str> = cookies
        .split(';')
        .filter_map(|pair| {
            let pair = pair.trim();
            pair.split_once('=').map(|(k, v)| (k.trim(), v.trim()))
        })
        .collect();

    // Prefer SAPISID, fallback to __Secure-3PAPISID
    cookie_map
        .get("SAPISID")
        .or_else(|| cookie_map.get("__Secure-3PAPISID"))
        .map(|v| v.to_string())
        .ok_or(Error::header())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_sapisid_with_sapisid() {
        let cookies = "SID=abc123; SAPISID=my_sapisid_value; HSID=xyz789";
        let result = extract_sapisid(cookies).unwrap();
        assert_eq!(result, "my_sapisid_value");
    }

    #[test]
    fn test_extract_sapisid_with_secure_fallback() {
        // Only __Secure-3PAPISID present, no SAPISID
        let cookies = "SID=abc123; __Secure-3PAPISID=secure_value; HSID=xyz789";
        let result = extract_sapisid(cookies).unwrap();
        assert_eq!(result, "secure_value");
    }

    #[test]
    fn test_extract_sapisid_prefers_sapisid_over_secure() {
        // Both present - should prefer SAPISID
        let cookies = "SAPISID=primary_value; __Secure-3PAPISID=secure_value; OTHER=x";
        let result = extract_sapisid(cookies).unwrap();
        assert_eq!(result, "primary_value");
    }

    #[test]
    fn test_extract_sapisid_at_end_no_semicolon() {
        // SAPISID at end with no trailing semicolon
        let cookies = "SID=abc123; SAPISID=last_value";
        let result = extract_sapisid(cookies).unwrap();
        assert_eq!(result, "last_value");
    }

    #[test]
    fn test_extract_sapisid_secure_at_end_no_semicolon() {
        // __Secure-3PAPISID at end with no trailing semicolon
        let cookies = "SID=abc123; __Secure-3PAPISID=secure_last";
        let result = extract_sapisid(cookies).unwrap();
        assert_eq!(result, "secure_last");
    }

    #[test]
    fn test_extract_sapisid_missing_returns_error() {
        let cookies = "SID=abc123; HSID=xyz789";
        let result = extract_sapisid(cookies);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_sapisid_empty_cookies_returns_error() {
        let cookies = "";
        let result = extract_sapisid(cookies);
        assert!(result.is_err());
    }

    #[test]
    fn test_normalize_cookie_file_contents_keeps_cookie_header_format() {
        let cookies = "SID=abc123; SAPISID=my_sapisid_value; HSID=xyz789";
        let result = normalize_cookie_file_contents(cookies).unwrap();
        assert_eq!(result, cookies);
    }

    #[test]
    fn test_normalize_cookie_file_contents_parses_netscape_cookie_jar() {
        let cookies = r#"# Netscape HTTP Cookie File
.youtube.com	TRUE	/	TRUE	0	SAPISID	my_sapisid_value
#HttpOnly_.youtube.com	TRUE	/	TRUE	0	__Secure-3PAPISID	secure_value
.youtube.com	TRUE	/	FALSE	0	HSID	xyz789
"#;
        let result = normalize_cookie_file_contents(cookies).unwrap();
        assert!(result.contains("SAPISID=my_sapisid_value"));
        assert!(result.contains("__Secure-3PAPISID=secure_value"));
        assert!(result.contains("HSID=xyz789"));
        assert_eq!(extract_sapisid(&result).unwrap(), "my_sapisid_value");
    }
}
