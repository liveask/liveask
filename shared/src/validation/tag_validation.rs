use super::ValidationState;

#[derive(Debug)]
pub enum TagError {
    MaxLength(usize, usize),
    MinLength(usize, usize),
}

const TRIMMED_MIN_LEN: usize = 3;
const MAX_LEN: usize = 30;

#[derive(Default, Debug)]
pub struct TagValidation {
    pub content: ValidationState<TagError>,
}

impl TagValidation {
    pub fn check(&mut self, content: &str) {
        self.content = Self::check_content(content);
    }

    #[must_use]
    pub const fn has_any(&self) -> bool {
        !self.content.is_valid()
    }

    fn check_content(v: &str) -> ValidationState<TagError> {
        if v.trim().len() < TRIMMED_MIN_LEN {
            ValidationState::Invalid(TagError::MinLength(v.trim().len(), TRIMMED_MIN_LEN))
        } else if v.len() > MAX_LEN {
            ValidationState::Invalid(TagError::MaxLength(v.len(), MAX_LEN))
        } else {
            ValidationState::Valid
        }
    }
}
