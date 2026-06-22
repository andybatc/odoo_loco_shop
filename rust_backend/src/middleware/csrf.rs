use axum_extra::extract::cookie::{Cookie, CookieJar};
use uuid::Uuid;

const CSRF_COOKIE_NAME: &str = "csrf_token";

pub fn generate_csrf_cookie() -> Cookie<'static> {
    let token = Uuid::new_v4().to_string();
    Cookie::build((CSRF_COOKIE_NAME, token))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Strict)
        .build()
}

pub fn validate_csrf(jar: &CookieJar, header_value: Option<&str>) -> bool {
    let cookie_token = match jar.get(CSRF_COOKIE_NAME) {
        Some(c) => c.value().to_string(),
        None => return false,
    };
    match header_value {
        Some(val) => val == cookie_token,
        None => false,
    }
}
