use std::fmt;

#[derive(Debug)]
pub struct SizeMismatchError;

impl fmt::Display for SizeMismatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Image sizes do not match!")
    }
}

impl std::error::Error for SizeMismatchError {}
