
use core::fmt;

#[derive(Debug)]
pub enum ApiError {
    #[allow(dead_code)]
    Unauthorized,
    Generic(String),
}

impl std::error::Error for ApiError {}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ApiError::Unauthorized => write!(f, "Unauthorized"),
            ApiError::Generic(s) => write!(f, "Generic: {}", s),
        }
    }
}
