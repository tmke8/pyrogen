#[derive(Debug, Copy, Clone, Hash, result_like::BoolLike)]
pub enum TypeIgnore {
    /// Normal situation in which "type: ignore" comments can disable errors.
    Enabled,
    /// If "type: ignore" comments are disabled, they are completely ignored;
    /// as if they weren't there.
    Disabled,
}

#[derive(Debug, Copy, Clone, Hash, result_like::BoolLike)]
pub enum Cache {
    Enabled,
    Disabled,
}
