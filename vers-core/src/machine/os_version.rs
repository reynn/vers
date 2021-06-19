use super::errors::*;

#[derive(Debug, Clone)]
pub struct OsVersion {
    // variant: String,
}

impl OsVersion {
    pub fn get() -> Result<Self> {
        Ok(Self {})
    }
}
