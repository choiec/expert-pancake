use core_shared::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct MeilisearchClient {
    pub http_addr: String,
    pub master_key: String,
    available: bool,
}

impl MeilisearchClient {
    pub fn new(http_addr: impl Into<String>, master_key: impl Into<String>) -> Self {
        let http_addr = http_addr.into();
        let master_key = master_key.into();
        let available = !http_addr.trim().is_empty() && !master_key.trim().is_empty();
        Self {
            http_addr,
            master_key,
            available,
        }
    }

    pub fn is_available(&self) -> bool {
        self.available
    }

    pub fn readiness_probe(&self) -> AppResult<()> {
        if self.available {
            Ok(())
        } else {
            Err(AppError::search_unavailable(
                "Meilisearch projection dependency is unavailable",
            ))
        }
    }
}
