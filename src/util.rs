use lazy_static::lazy_static;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderName};
use rocket::{
    http::{Header, Status},
    outcome::Outcome,
    request::FromRequest,
    response::{self, Responder},
    Request, Response,
};
use std::io::Cursor;

lazy_static! {
    pub static ref HOST_EXTRACTOR_REGEX: Regex =
        Regex::new(r"^(?:https?://)?(?:[^@/\n]+@)?(?:www\.)?([^:/?\n]+)").unwrap(); // from https://www.folkstalk.com/tech/regex-get-only-domain-name-from-url-with-code-examples/
}

const IGNORED_HEADERS: &[&str] = &[
    "server",
    "connection",
    "cache-control",
    "keep-alive",
    "x-proxy-cache",
    "referrer-policy",
    "transfer-encoding",
];

pub struct ReqwestResponse<'r>(pub (u16, Vec<u8>, Vec<Header<'r>>)); // (status, body, headers)

impl<'r> Responder<'r, 'r> for ReqwestResponse<'r> {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        info!("Responding to request");
        let inner = self.0;
        let status = inner.0;
        let body = inner.1;
        let headers = inner.2;

        info!("Building response");
        let mut response = Response::new();
        response.set_status(Status::new(status));
        response.set_sized_body(body.len(), Cursor::new(body));

        let extractor_regex = &crate::util::HOST_EXTRACTOR_REGEX;

        let host_baseurl = match std::env::var("YOUR_BASEURL") {
            Ok(host_baseurl) => host_baseurl,
            Err(_) => {
                warn!("Failed to get YOUR_BASEURL");
                return Err(Status::InternalServerError);
            }
        };
        let host_hostname = match extractor_regex.captures(host_baseurl.as_str()) {
            Some(captures) => match captures.get(1) {
                Some(hostname) => hostname.as_str().to_string(),
                None => {
                    warn!("Failed to extract hostname from YOUR_BASEURL");
                    return Err(Status::InternalServerError);
                }
            },
            None => {
                warn!("Failed to extract hostname from YOUR_BASEURL");
                return Err(Status::InternalServerError);
            }
        };

        let prox_baseurl = match std::env::var("PROX_BASEURL") {
            Ok(prox_baseurl) => prox_baseurl,
            Err(_) => {
                warn!("Failed to get PROX_BASEURL");
                return Err(Status::InternalServerError);
            }
        };
        let prox_hostname = match extractor_regex.captures(prox_baseurl.as_str()) {
            Some(captures) => match captures.get(1) {
                Some(hostname) => hostname.as_str().to_string(),
                None => {
                    warn!("Failed to extract hostname from PROX_BASEURL");
                    return Err(Status::InternalServerError);
                }
            },
            None => {
                warn!("Failed to extract hostname from PROX_BASEURL");
                return Err(Status::InternalServerError);
            }
        };

        let domain = format!("Domain={};", host_hostname);
        for header in headers {
            let name = header.name().as_str().to_string();
            if IGNORED_HEADERS.contains(&name.as_str()) {
                continue;
            }
            if name == "set-cookie" {
                let value = header.value().to_string();
                value
                    .split(format!("Domain=.{};", prox_hostname).as_str())
                    .collect::<Vec<&str>>()
                    .join(&domain);
                response.set_header(Header::new(name, value));
            } else {
                response.set_header(header.clone());
            }
        }
        info!("Built response");
        info!("Returning response");
        Ok(response)
    }
}

pub struct RequestHeaders(pub HeaderMap);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RequestHeaders {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let headers = request.headers().clone();
        let mut header_map = HeaderMap::new();
        for header in headers.iter() {
            let raw_name = header.name().to_string();
            let value = header.value().to_string();

            let name = match HeaderName::try_from(raw_name) {
                Ok(name) => name,
                Err(e) => {
                    warn!("Failed to convert header name to HeaderName: {}", e);
                    continue;
                }
            };
            header_map.insert(name, value.parse().unwrap());
        }
        Outcome::Success(RequestHeaders(header_map))
    }
}

pub struct QueryParams(pub Vec<(String, String)>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for QueryParams {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let query = match request.uri().query() {
            Some(query) => query,
            None => return Outcome::Success(QueryParams(vec![])),
        };
        let query_params = query
            .split('&')
            .map(|param| {
                let mut split = param.split('=');
                let key = split.next().unwrap_or("".into()).to_string();
                let value = split.next().unwrap_or("".into()).to_string();
                (key, value)
            })
            .collect::<Vec<(String, String)>>();
        Outcome::Success(QueryParams(query_params))
    }
}
