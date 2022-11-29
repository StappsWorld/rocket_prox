#[macro_use]
extern crate rocket;

use rocket::{http::Method, routes};
use rocket_cors::{AllowedHeaders, AllowedOrigins};
use std::path::PathBuf;

#[get("/<path..>")]
async fn get_path(path: PathBuf) -> String {
    format!("Path: {:?}", path)
}

#[rocket::main]
async fn main() {
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
