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
const APP_API_JS: &str = include_str!("../frontend/app/api.js");
const APP_CONTEXT_JS: &str = include_str!("../frontend/app/context.js");
const APP_I18N_JS: &str = include_str!("../frontend/app/i18n.js");
const APP_GRID_JS: &str = include_str!("../frontend/app/components/grid.js");
const APP_MENU_JS: &str = include_str!("../frontend/app/menu.js");
const APP_ROUTER_JS: &str = include_str!("../frontend/app/router.js");
const APP_SCREENS_JS: &str = include_str!("../frontend/app/screens.js");

pub async fn index() -> Response<Body> {
    static_response(INDEX_HTML, "text/html; charset=utf-8")
}

pub async fn app_css() -> Response<Body> {
    static_response(APP_CSS, "text/css; charset=utf-8")
}

pub async fn app_js() -> Response<Body> {
    static_response(APP_JS, "application/javascript; charset=utf-8")
}

pub async fn app_api_js() -> Response<Body> {
    static_response(APP_API_JS, "application/javascript; charset=utf-8")
}

pub async fn app_context_js() -> Response<Body> {
    static_response(APP_CONTEXT_JS, "application/javascript; charset=utf-8")
}

pub async fn app_i18n_js() -> Response<Body> {
    static_response(APP_I18N_JS, "application/javascript; charset=utf-8")
}

pub async fn app_grid_js() -> Response<Body> {
    static_response(APP_GRID_JS, "application/javascript; charset=utf-8")
}

pub async fn app_menu_js() -> Response<Body> {
    static_response(APP_MENU_JS, "application/javascript; charset=utf-8")
}

pub async fn app_router_js() -> Response<Body> {
    static_response(APP_ROUTER_JS, "application/javascript; charset=utf-8")
}

pub async fn app_screens_js() -> Response<Body> {
    static_response(APP_SCREENS_JS, "application/javascript; charset=utf-8")
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
