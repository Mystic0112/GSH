use crate::BrowserError;
pub fn validate_url(url: &str) -> Result<(), BrowserError> {
    if url == "about:blank" || url.starts_with("http://") || url.starts_with("https://") {
        Ok(())
    } else {
        Err(BrowserError::Message(
            "only http://, https://, and about:blank URLs are allowed".into(),
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restricts_schemes() {
        assert!(validate_url("https://x").is_ok());
        assert!(validate_url("file:///x").is_err());
        assert!(validate_url("javascript:x").is_err());
    }
}
