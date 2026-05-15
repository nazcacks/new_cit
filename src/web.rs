use axum::{
    body::Body,
    http::{
        header::{CACHE_CONTROL, CONTENT_TYPE},
        HeaderValue, Response,
    },
};

const INDEX_HTML: &str = include_str!("../frontend/index.html");
const APP_CSS: &str = include_str!("../frontend/app.css");
const APP_JS: &str = include_str!("../frontend/app.js");

pub async fn index() -> Response<Body> {
    static_response(INDEX_HTML, "text/html; charset=utf-8")
}

pub async fn app_css() -> Response<Body> {
    static_response(APP_CSS, "text/css; charset=utf-8")
}

pub async fn app_js() -> Response<Body> {
    static_response(APP_JS, "application/javascript; charset=utf-8")
}

fn static_response(contents: &'static str, content_type: &'static str) -> Response<Body> {
    let mut response = Response::new(Body::from(contents));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    response.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("no-cache, no-store, must-revalidate"),
    );
    response
}
