use core_infra::NoopGraphProjectionAdapter;
use core_shared::AppResult;

use crate::domain::event::GraphProjectionEvent;

pub trait GraphProjectionPort: Send + Sync {
    fn project(&self, event: &GraphProjectionEvent) -> AppResult<()>;
}

impl GraphProjectionPort for NoopGraphProjectionAdapter {
    fn project(&self, _event: &GraphProjectionEvent) -> AppResult<()> {
        Ok(())
    }
}
