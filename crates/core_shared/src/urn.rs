use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryItemUrn(String);

impl MemoryItemUrn {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for MemoryItemUrn {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for MemoryItemUrn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<String> for MemoryItemUrn {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}
