use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RequestId(pub String);

pub async fn request_id(mut request: Request, next: Next) -> Response {
    let request_id = Uuid::new_v4().to_string();
    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));

    let mut response = next.run(request).await;
    let header_name = HeaderName::from_static("x-request-id");
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(header_name, header_value);
    }

    response
}
