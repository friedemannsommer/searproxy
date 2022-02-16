pub mod lib;
mod routes;

#[actix_web::main]
pub async fn start_http_service() {
    let config = crate::lib::GLOBAL_CONFIG
        .get()
        .expect("Global config is not initialized");
    let http_server = actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .wrap(actix_web::middleware::Compress::default())
            .wrap(actix_web::middleware::NormalizePath::new(
                actix_web::middleware::TrailingSlash::Trim,
            ))
            .wrap(actix_web::middleware::Logger::new("%a '%r' %s %T"))
            .wrap(get_default_headers_middleware())
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
            .service(crate::static_asset_route!(
                "/main.css",
                crate::assets::MAIN_STYLESHEET,
                "text/css"
            ))
            .service(crate::static_asset_route!(
                "/header.css",
                crate::assets::HEADER_STYLESHEET,
                "text/css"
            ))
            .route("/", actix_web::web::get().to(routes::handle_index))
    })
    .backlog(4096)
    .shutdown_timeout(5);

    match match &config.listen {
        crate::model::SocketListener::Tcp(address) => http_server.bind(address),
        crate::model::SocketListener::Unix(path) => http_server.bind_uds(path),
    } {
        Ok(server_socket) => {
            log::info!("Listening on {:?}", &config.listen);
            server_socket
        }
        Err(err) => {
            log::error!("Couldn't bind to '{:?}'", &config.listen);
            panic!("{:?}", err);
        }
    }
    .run()
    .await
    .expect("Couldn't start HTTP workers");
}

fn get_default_headers_middleware() -> actix_web::middleware::DefaultHeaders {
    actix_web::middleware::DefaultHeaders::new()
        .add((
            actix_web::http::header::CONTENT_SECURITY_POLICY,
            "default-src 'none'; block-all-mixed-content; img-src data: 'self'; style-src 'self'; prefetch-src 'self'; media-src 'self'; frame-src 'self'; font-src 'self'; frame-ancestors 'self'",
        ))
        .add((actix_web::http::header::REFERRER_POLICY, "no-referrer"))
        .add((actix_web::http::header::X_FRAME_OPTIONS, "SAMEORIGIN"))
        .add((actix_web::http::header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
}
