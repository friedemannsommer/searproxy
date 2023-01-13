#![deny(
    clippy::correctness,
    clippy::style,
    keyword_idents,
    macro_use_extern_crate,
    non_ascii_idents,
    nonstandard_style,
    noop_method_call,
    pointer_structural_match,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_crate_dependencies
)]
#![warn(
    clippy::cargo,
    clippy::complexity,
    clippy::perf,
    clippy::suspicious,
    rust_2018_idioms,
    unused
)]
#![allow(clippy::multiple_crate_versions)]

use base64::Engine;

mod assets;
mod model;
mod server;
mod templates;
mod utilities;

fn main() {
    let config = get_config();

    init_logging(&config);
    log::debug!("{:?}", &config);
    set_shared_values(config);
    server::start_http_service();
}

fn get_config() -> model::Config<'static, 'static> {
    use clap::Parser;

    let args: model::Cli = model::Cli::parse();

    model::Config {
        follow_redirects: args.follow_redirects,
        hmac_secret: std::borrow::Cow::Owned(
            utilities::BASE64_ENGINE
                .decode(&args.hmac_secret)
                .expect("HMAC secret couldn't be [base64] decoded"),
        ),
        lazy_images: args.lazy_images,
        listen: parse_socket_listener(&args.listen),
        log_level: args.log_level,
        request_timeout: args.request_timeout,
        proxy_address: args.proxy_address.map(std::borrow::Cow::Owned),
        worker_count: args.worker_count,
    }
}

fn parse_socket_listener(input: &str) -> model::SocketListener {
    use std::str::FromStr;

    if let Ok(address) = std::net::SocketAddr::from_str(input) {
        return model::SocketListener::Tcp(address);
    }

    #[cfg(unix)]
    if let Ok(path) = std::path::PathBuf::from_str(input) {
        return model::SocketListener::Unix(path);
    }

    panic!("Listener could not be parsed: '{}'", input)
}

fn init_logging(config: &model::Config<'_, '_>) {
    if config.log_level != log::LevelFilter::Off {
        let mut logger = fern::Dispatch::new().level(config.log_level);

        if config.log_level != log::LevelFilter::Error {
            logger = logger.chain(
                fern::Dispatch::new()
                    .filter(|meta| meta.level() != log::LevelFilter::Error)
                    .chain(std::io::stdout()),
            )
        }

        logger
            .chain(
                fern::Dispatch::new()
                    .level(log::LevelFilter::Error)
                    .chain(std::io::stderr()),
            )
            .apply()
            .expect("logging subscriber registration failed");
    } else {
        log::set_max_level(config.log_level);
    }
}

fn set_shared_values(config: model::Config<'static, 'static>) {
    use hmac::digest::KeyInit;

    if utilities::HMAC
        .set(hmac::Hmac::new_from_slice(config.hmac_secret.as_ref()).expect("Invalid HMAC secret"))
        .is_err()
    {
        panic!("Failed to set HMAC instance");
    }

    let timeout = std::time::Duration::from_secs(config.request_timeout as u64);
    let mut request_client_builder = reqwest::Client::builder()
        .referer(false)
        .deflate(true)
        .gzip(true)
        .brotli(true)
        .redirect(reqwest::redirect::Policy::none())
        .trust_dns(true)
        .tcp_nodelay(true)
        .tcp_keepalive(None)
        .timeout(timeout)
        .connect_timeout(timeout)
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:91.0) Gecko/20100101 Firefox/91.0",
        );

    if let Some(proxy_address) = config.proxy_address.as_deref() {
        request_client_builder = request_client_builder
            .proxy(reqwest::Proxy::all(proxy_address).expect("Can't use given proxy config"));
    }

    utilities::REQUEST_CLIENT
        .set(
            request_client_builder
                .build()
                .expect("Request client initialization failed"),
        )
        .expect("Failed to set request client");

    utilities::GLOBAL_CONFIG
        .set(config)
        .expect("Failed to set global config");
}
