use super::ValidationState;

///
#[derive(Debug)]
pub enum ContextLabelError {
    MaxLength(usize, usize),
    MinLength(usize, usize),
}

///
#[derive(Debug)]
pub enum ContextUrlError {
    Invalid(url::ParseError),
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
            Ok(_) => ValidationState::Valid,
        }
    }
}
