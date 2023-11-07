///
#[derive(Debug)]
pub enum CreateEventError {
    Empty,
    MaxLength(usize, usize),
    MinLength(usize, usize),
    MaxWords(usize, usize),
}

const DESC_TRIMMED_MIN_LEN: usize = 30;
const NAME_TRIMMED_MIN_LEN: usize = 8;
const NAME_TRIMMED_MAX_LEN: usize = 30;
const NAME_TRIMMED_MAX_WORDS: usize = 13;

#[derive(Default, Debug)]
pub struct CreateEventValidation {
    pub name: Option<CreateEventError>,
    pub desc: Option<CreateEventError>,
}

impl CreateEventValidation {
    pub fn check(&mut self, name: &str, desc: &str) {
        self.name = Self::check_name(name);
        self.desc = Self::check_desc(desc);
    }

    #[must_use]
    pub const fn has_any(&self) -> bool {
        self.name.is_some() || self.desc.is_some()
    }

    fn check_name(v: &str) -> Option<CreateEventError> {
        let trimmed_len = v.trim().len();
        let words = v.split_whitespace().count();

        if trimmed_len == 0 {
            Some(CreateEventError::Empty)
        } else if trimmed_len < NAME_TRIMMED_MIN_LEN {
            Some(CreateEventError::MinLength(
                trimmed_len,
                NAME_TRIMMED_MIN_LEN,
            ))
        } else if trimmed_len > NAME_TRIMMED_MAX_LEN {
            Some(CreateEventError::MaxLength(
                trimmed_len,
                NAME_TRIMMED_MAX_LEN,
            ))
        } else if words > NAME_TRIMMED_MAX_WORDS {
            Some(CreateEventError::MaxWords(words, NAME_TRIMMED_MAX_WORDS))
        } else {
            None
        }
    }

    #[must_use]
    pub fn check_desc(v: &str) -> Option<CreateEventError> {
        let trimmed_len = v.trim().len();

        if trimmed_len == 0 {
            Some(CreateEventError::Empty)
        } else if trimmed_len < DESC_TRIMMED_MIN_LEN {
            Some(CreateEventError::MinLength(
                trimmed_len,
                DESC_TRIMMED_MIN_LEN,
            ))
        } else {
            None
        }
    }
}
