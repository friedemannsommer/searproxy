pub mod lib;
mod routes;

use tracing::{error, info};

#[actix_web::main]
pub async fn start_http_service() {
    let config = crate::lib::GLOBAL_CONFIG
        .get()
        .expect("Global config is not initialized");

    match actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .wrap(actix_web::middleware::Compress::default())
            .wrap(actix_web::middleware::NormalizePath::new(
                actix_web::middleware::TrailingSlash::Trim,
            ))
            .wrap(tracing_actix_web::TracingLogger::<lib::RootSpan>::new())
            .service(crate::static_asset_route!(
                "/favicon.ico",
                crate::assets::FAVICON_ICO_FILE,
                "image/ico"
            ))
            .service(crate::static_asset_route!(
                "/favicon-16x16.png",
                crate::assets::FAVICON_PNG_16_FILE,
                "image/png"
            ))
            .service(crate::static_asset_route!(
                "/favicon-32x32.png",
                crate::assets::FAVICON_PNG_32_FILE,
                "image/png"
            ))
            .service(crate::static_asset_route!(
                "/robots.txt",
                crate::assets::ROBOTS_FILE,
                "text/plain"
            ))
            .route("/", actix_web::web::get().to(routes::handle_index))
    })
    .backlog(4096)
    .shutdown_timeout(5)
    .bind(config.listen_address.as_ref())
    {
        Ok(server_socket) => {
            info!("Listening on {}", &config.listen_address);
            server_socket
        }
        Err(err) => {
            error!("Couldn't bind to '{}'", &config.listen_address);
            panic!("{:?}", err);
        }
    }
    .run()
    .await
    .expect("Couldn't start HTTP workers");
}
