pub mod add_question;
pub mod create_event;
pub mod pwd_validation;

#[derive(Debug)]
pub enum ValidationState<T> {
    Unused,
    Valid,
    Invalid(T),
}

impl<T> Default for ValidationState<T> {
    fn default() -> Self {
        Self::Unused
    }
}

impl<T> ValidationState<T> {
    pub const fn is_valid(&self) -> bool {
        matches!(&self, Self::Valid)
    }

    pub const fn is_invalid(&self) -> bool {
        matches!(&self, Self::Invalid(_))
    }

    pub const fn is_unused(&self) -> bool {
        matches!(&self, Self::Unused)
    }
}
