use super::ValidationState;

#[derive(Debug)]
pub enum AddQuestionError {
    MaxLength(usize, usize),
    MinLength(usize, usize),
    MinWordCount(usize, usize),
    WordLengthMax(usize),
}

const TRIMMED_MIN_LEN: usize = 10;
const TRIMMED_MAX_LEN: usize = 200;
const WORD_MIN: usize = 3;
const WORD_LEN_MAX: usize = 30;

#[derive(Default, Debug)]
pub struct AddQuestionValidation {
    pub content: ValidationState<AddQuestionError>,
}

impl AddQuestionValidation {
    pub fn check(&mut self, content: &str) {
        self.content = Self::check_content(content);
    }

    #[must_use]
    pub const fn has_any(&self) -> bool {
        !self.content.is_valid()
    }

    fn check_content(v: &str) -> ValidationState<AddQuestionError> {
        let trimmed_len = v.trim().len();
        let words = v.split_whitespace().count();

        if trimmed_len < TRIMMED_MIN_LEN {
            ValidationState::Invalid(AddQuestionError::MinLength(trimmed_len, TRIMMED_MIN_LEN))
        } else if trimmed_len > TRIMMED_MAX_LEN {
            ValidationState::Invalid(AddQuestionError::MaxLength(trimmed_len, TRIMMED_MAX_LEN))
        } else if words < WORD_MIN {
            ValidationState::Invalid(AddQuestionError::MinWordCount(words, WORD_MIN))
        } else if v
            .split_ascii_whitespace()
            .any(|word| word.len() > WORD_LEN_MAX)
        {
            ValidationState::Invalid(AddQuestionError::WordLengthMax(WORD_LEN_MAX))
        } else {
            ValidationState::Valid
        }
    }
}
