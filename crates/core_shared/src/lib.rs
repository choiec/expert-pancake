pub mod error;
pub mod id_gen;
pub mod urn;

pub use error::{AppError, CoreResult, ErrorKind, StartupError};
pub use id_gen::{DefaultIdGenerator, IdGenerator};
pub use urn::MemoryItemUrn;

pub type AppResult<T> = CoreResult<T>;
pub type ErrorCode = ErrorKind;
