///
#[derive(Debug)]
pub enum ValidationError {
    Empty,
    MaxLength(usize, usize),
    MinLength(usize, usize),
    MaxWords(usize, usize),
}

pub const DESC_TRIMMED_MIN_LEN: usize = 30;
pub const NAME_TRIMMED_MIN_LEN: usize = 8;
pub const NAME_TRIMMED_MAX_LEN: usize = 30;
pub const NAME_TRIMMED_MAX_WORDS: usize = 13;

#[derive(Default, Debug)]
pub struct CreateEventErrors {
    pub name: Option<ValidationError>,
    pub desc: Option<ValidationError>,
}

impl CreateEventErrors {
    pub fn check(&mut self, name: &str, desc: &str) {
        self.name = Self::check_name(name);
        self.desc = Self::check_desc(desc);
    }

    #[must_use]
    pub const fn has_any(&self) -> bool {
        self.name.is_some() || self.desc.is_some()
    }

    fn check_name(v: &str) -> Option<ValidationError> {
        let trimmed_len = v.trim().len();
        let words = v.split_whitespace().count();

        if trimmed_len == 0 {
            Some(ValidationError::Empty)
        } else if trimmed_len < NAME_TRIMMED_MIN_LEN {
            Some(ValidationError::MinLength(
                trimmed_len,
                NAME_TRIMMED_MIN_LEN,
            ))
        } else if trimmed_len > NAME_TRIMMED_MAX_LEN {
            Some(ValidationError::MaxLength(
                trimmed_len,
                NAME_TRIMMED_MAX_LEN,
            ))
        } else if words > NAME_TRIMMED_MAX_WORDS {
            Some(ValidationError::MaxWords(words, NAME_TRIMMED_MAX_WORDS))
        } else {
            None
        }
    }

    fn check_desc(v: &str) -> Option<ValidationError> {
        let trimmed_len = v.trim().len();

        if trimmed_len == 0 {
            Some(ValidationError::Empty)
        } else if trimmed_len < DESC_TRIMMED_MIN_LEN {
            Some(ValidationError::MinLength(
                trimmed_len,
                DESC_TRIMMED_MIN_LEN,
            ))
        } else {
            None
        }
    }
}
