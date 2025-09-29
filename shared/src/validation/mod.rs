pub mod add_question;
pub mod context_validation;
pub mod create_event;
pub mod pwd_validation;
pub mod tag_validation;

#[derive(Debug, Default)]
pub enum ValidationState<T> {
    #[default]
    Unused,
    Valid,
    Invalid(T),
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
