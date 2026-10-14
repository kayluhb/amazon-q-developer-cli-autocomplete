use std::borrow::Cow;
use std::error::Error;

use anyhow::Result;
use image::Rgb;
use wry::http::header::CONTENT_TYPE;
use wry::http::{
    Response,
    StatusCode,
};

pub fn res_ok(body: Vec<u8>, content_type: &str) -> Result<Response<Cow<'static, [u8]>>> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .body(body.into())?)
}

pub fn res_404() -> Result<Response<Cow<'static, [u8]>>> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(CONTENT_TYPE, "text/plain")
        .body(b"Not Found".as_ref().into())?)
}

pub fn res_400() -> Result<Response<Cow<'static, [u8]>>> {
    Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header(CONTENT_TYPE, "text/plain")
        .body(b"Bad Request".as_ref().into())?)
}

pub fn res_500(err: impl Error) -> Result<Response<Cow<'static, [u8]>>> {
    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(CONTENT_TYPE, "text/plain")
        .body(err.to_string().into_bytes().into())?)
}

/// The MIME type of the common web file extensions, or `None` for any other extension.
pub fn mime_for_extension(ext: Option<&str>) -> Option<&'static str> {
    Some(match ext? {
        "html" => mime::TEXT_HTML_UTF_8.as_ref(),
        "css" => mime::TEXT_CSS_UTF_8.as_ref(),
        "js" => mime::APPLICATION_JAVASCRIPT_UTF_8.as_ref(),
        "json" => mime::APPLICATION_JSON.as_ref(),
        "svg" => mime::IMAGE_SVG.as_ref(),
        "png" => mime::IMAGE_PNG.as_ref(),
        "jpg" | "jpeg" => mime::IMAGE_JPEG.as_ref(),
        "woff2" => mime::FONT_WOFF2.as_ref(),
        "woff" => mime::FONT_WOFF.as_ref(),
        "txt" | "text" => mime::TEXT_PLAIN.as_ref(),
        _ => return None,
    })
}

pub fn parse_hex_rgb(s: &str) -> Option<Rgb<u8>> {
    if s.len() != 6 {
        return None;
    }
    let radix = 16;
    Some(Rgb([
        u8::from_str_radix(&s[0..2], radix).ok()?,
        u8::from_str_radix(&s[2..4], radix).ok()?,
        u8::from_str_radix(&s[4..6], radix).ok()?,
    ]))
}

pub const fn scale_u8(a: u8, b: u8) -> u8 {
    ((a as u16 * b as u16) / 256) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_res_ok() {
        let res = res_ok(b"{}".to_vec(), "application/json").unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()[CONTENT_TYPE], "application/json");
        assert_eq!(res.body().as_ref(), b"{}");
    }

    #[test]
    fn test_mime_for_extension() {
        assert_eq!(mime_for_extension(Some("html")), Some("text/html; charset=utf-8"));
        assert_eq!(mime_for_extension(Some("json")), Some("application/json"));
        assert_eq!(mime_for_extension(Some("svg")), Some("image/svg+xml"));
        assert_eq!(mime_for_extension(Some("jpeg")), Some("image/jpeg"));
        assert_eq!(mime_for_extension(Some("exe")), None);
        assert_eq!(mime_for_extension(None), None);
    }

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_rgb("000000"), Some(Rgb([0, 0, 0])));
        assert_eq!(parse_hex_rgb("ffffff"), Some(Rgb([255, 255, 255])));
        assert_eq!(parse_hex_rgb("ff0000"), Some(Rgb([255, 0, 0])));
        assert_eq!(parse_hex_rgb("00ff00"), Some(Rgb([0, 255, 0])));
        assert_eq!(parse_hex_rgb("0000ff"), Some(Rgb([0, 0, 255])));
        assert_eq!(parse_hex_rgb("00000f"), Some(Rgb([0, 0, 15])));
        assert_eq!(parse_hex_rgb("00000"), None);
        assert_eq!(parse_hex_rgb("0000000"), None);
        assert_eq!(parse_hex_rgb("00000g"), None);
    }

    #[test]
    fn test_scale() {
        assert_eq!(scale_u8(0, 0), 0);
        assert_eq!(scale_u8(128, 0), 0);
        assert_eq!(scale_u8(255, 0), 0);
        assert_eq!(scale_u8(128, 128), 64);
        assert_eq!(scale_u8(255, 255), 254);
    }
}
