use std::fmt::Debug;

pub trait Violation: Debug + PartialEq + Eq {
    /// The message used to describe the violation.
    fn message(&self) -> String;

    /// The explanation used in documentation and elsewhere.
    fn explanation() -> Option<&'static str> {
        None
    }
}
