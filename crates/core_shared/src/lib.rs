pub mod error;
pub mod id;

pub use error::{ApiError, ErrorBody, error_body};
pub use id::{decode_credential_id, encode_credential_id};
