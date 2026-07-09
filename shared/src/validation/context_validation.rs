use super::ValidationState;

#[derive(Debug)]
pub enum ContextLabelError {
    MaxLength(usize, usize),
    MinLength(usize, usize),
}

#[derive(Debug)]
pub enum ContextUrlError {
    Invalid(url::ParseError),
    /// scheme other than http/https (e.g. `javascript:`, `data:`) — refused because the
    /// url is rendered as a clickable link for every event viewer.
    DisallowedScheme,
}

const LABEL_TRIMMED_MIN_LEN: usize = 4;
const LABEL_MAX_LEN: usize = 20;

#[derive(Default, Debug)]
pub struct ContextValidation {
    pub label: ValidationState<ContextLabelError>,
    pub url: ValidationState<ContextUrlError>,
}

impl ContextValidation {
    pub fn check(&mut self, label: &str, url: &str) {
        self.label = Self::check_label(label);
        self.url = Self::check_url(url);
    }

    #[must_use]
    pub const fn has_any(&self) -> bool {
        !self.label.is_valid() || !self.url.is_valid()
    }

    fn check_label(v: &str) -> ValidationState<ContextLabelError> {
        if v.trim().len() < LABEL_TRIMMED_MIN_LEN {
            ValidationState::Invalid(ContextLabelError::MinLength(
                v.trim().len(),
                LABEL_TRIMMED_MIN_LEN,
            ))
        } else if v.len() > LABEL_MAX_LEN {
            ValidationState::Invalid(ContextLabelError::MaxLength(v.len(), LABEL_MAX_LEN))
        } else {
            ValidationState::Valid
        }
    }

    fn check_url(v: &str) -> ValidationState<ContextUrlError> {
        match url::Url::parse(v) {
            Err(e) => ValidationState::Invalid(ContextUrlError::Invalid(e)),
            // only http(s) may be stored: the url becomes a clickable href for every event
            // viewer, so `javascript:`/`data:`/etc. would be a stored-XSS vector
            Ok(url) if !matches!(url.scheme(), "http" | "https") => {
                ValidationState::Invalid(ContextUrlError::DisallowedScheme)
            }
            Ok(_) => ValidationState::Valid,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn valid(v: &str) -> bool {
        ContextValidation::check_url(v).is_valid()
    }

    #[test]
    fn accepts_http_and_https() {
        assert!(valid("https://example.com"));
        assert!(valid("http://example.com/path?q=1"));
    }

    #[test]
    fn rejects_dangerous_schemes() {
        assert!(!valid("javascript:alert(1)"));
        assert!(!valid("data:text/html,<script>alert(1)</script>"));
        assert!(!valid("vbscript:msgbox(1)"));
        assert!(!valid("file:///etc/passwd"));
    }

    #[test]
    fn rejects_unparseable() {
        assert!(!valid("not a url"));
    }
}
