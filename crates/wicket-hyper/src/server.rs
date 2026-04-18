use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use std::sync::Arc;
use wicket_core::{
    protocol::http::WebApplication,
    request::{Body, Request, Response},
};

pub async fn handle_hyper_connection(
    app: Arc<WebApplication>,
    hyper_req: hyper::Request<hyper::body::Incoming>,
) -> hyper::Response<Full<Bytes>> {
    // 1. Conversion (Consuming Hyper Request)
    let (parts, incoming_body) = hyper_req.into_parts();

    let body_bytes = if parts.method == hyper::Method::GET {
        Body::None
    } else {
        let collected = incoming_body.collect().await.unwrap();
        Body::Bytes(collected.to_bytes())
    };

    let request = Request::new(parts, body_bytes);

    // 2. Dispatch to Wicket Core
    let response = app.process_request(request).await;

    // 3. Convert WicketResponse back to Hyper
    to_hyper_response(response)
}

pub fn to_hyper_response(_res: Response) -> hyper::Response<Full<Bytes>> {
    todo!()
}
