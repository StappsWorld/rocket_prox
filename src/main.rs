#![allow(unused_must_use)]

#[macro_use]
extern crate rocket;

use reqwest::header::{HOST, REFERER};
use rocket::{
    http::{Header, Method, Status},
    response::Responder,
    routes,
};
use rocket_cors::{AllowedHeaders, AllowedOrigins};
use std::path::PathBuf;

pub mod util;

#[get("/<path..>")]
async fn get_path<'o>(
    path: PathBuf,
    headers: util::RequestHeaders,
    query_params: util::QueryParams,
) -> impl Responder<'o, 'o> {
    let path = match path.to_str() {
        Some(path) => path.to_string(),
        None => {
            warn!("Failed to convert path to string");
            return Err(Status::BadRequest);
        }
    };

    let mut headers = headers.0;

    let extractor_regex = &crate::util::HOST_EXTRACTOR_REGEX;

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

    headers.insert(HOST, prox_hostname.parse().unwrap());
    if headers.contains_key(REFERER) {
        headers.insert(REFERER, prox_baseurl.parse().unwrap());
    }

    for header in &headers {
        info!("{:#?}: {:#?}", header.0, header.1);
    }

    let params = query_params.0;

    let url = if params.len() == 0 {
        format!("{}/{}", prox_baseurl, path)
    } else {
        let mut url = format!("{}/{}", prox_baseurl, path);
        url.push('?');
        for param in params {
            url.push_str(&format!("{}={}&", param.0, param.1));
        }
        url.pop();
        url
    };

    let reqwest_client = reqwest::Client::new();
    match reqwest_client.get(url).headers(headers).send().await {
        Ok(res) => {
            info!("Building headers");
            let headers = res
                .headers()
                .iter()
                .filter_map(|(key, value)| {
                    let key_str = key.as_str().to_owned();
                    let value_str = match value.to_str() {
                        Ok(value_str) => value_str.to_owned(),
                        Err(e) => {
                            warn!("Failed to convert header {} value to string: {}", key, e);
                            return None;
                        }
                    };
                    let header = Header::new(key_str, value_str);
                    info!(
                        "Built header: {:#?} : {:#?}",
                        header.name().as_str(),
                        header.value()
                    );
                    Some(header)
                })
                .collect::<Vec<Header>>();
            info!("Built headers");
            info!("Building status");
            let status = res.status().as_u16();
            info!("Status was {}", status);
            info!("Built status");
            info!("Getting response bytes");
            let body = match res.bytes().await {
                Ok(body) => body.to_vec(),
                Err(e) => {
                    warn!("Failed to get response body: {}", e);
                    return Err(Status::InternalServerError);
                }
            };
            info!("Got response bytes of length {}", body.len());
            Ok(crate::util::ReqwestResponse((status, body, headers)))
        }
        Err(e) => {
            warn!("Failed to get response: {}", e);
            Err(Status::InternalServerError)
        }
    }
}

#[rocket::main]
async fn main() {
    match dotenv::dotenv() {
        Ok(p) => println!("Loaded .env from {}", p.display()),
        Err(e) => eprintln!("Failed to load .env: {}", e),
    }

    let cors = rocket_cors::CorsOptions {
        allowed_origins: AllowedOrigins::all(), // TODO: Restrict
        allowed_methods: vec![
            Method::Get,
            Method::Post,
            Method::Put,
            Method::Delete,
            Method::Options,
        ]
        .into_iter()
        .map(From::from)
        .collect(),
        allowed_headers: AllowedHeaders::all(), // TODO: Restrict
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()
    .expect("Failed to build Rocket CORS");

    match rocket::build()
        .attach(cors.clone())
        .manage(cors)
        .mount("/", routes![get_path])
        .mount("/", rocket_cors::catch_all_options_routes())
        .launch()
        .await
    {
        Ok(_) => (),
        Err(e) => eprintln!("Error: {}", e),
    }
}
