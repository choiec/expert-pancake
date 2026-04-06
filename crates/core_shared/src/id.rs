use percent_encoding::{NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};

pub fn encode_credential_id(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

pub fn decode_credential_id(value: &str) -> String {
    percent_decode_str(value).decode_utf8_lossy().to_string()
}
