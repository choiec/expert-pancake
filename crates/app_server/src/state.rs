use mod_memory::bootstrap::CredentialModule;
use serde::Serialize;

#[derive(Clone)]
pub struct AppState {
    pub module: CredentialModule,
}

impl AppState {
    pub fn new(module: CredentialModule) -> Self {
        Self { module }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadinessResponse {
    pub status: &'static str,
    pub authoritative_store: &'static str,
    pub search: &'static str,
}
