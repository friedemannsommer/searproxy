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

use crate::model::AppState;

mod assets;
mod model;
mod server;
mod templates;
mod utilities;

fn main() {
    let config = get_config();

    init_logging(&config);
    log::debug!("{:?}", &config);
    set_shared_values(AppState::try_from(config).unwrap());
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

    panic!("Listener could not be parsed: '{input}'")
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

fn set_shared_values(app_state: AppState<'static, 'static>) {
    utilities::HMAC
        .set(app_state.hmac)
        .expect("Failed to set HMAC instance");

    utilities::REQUEST_CLIENT
        .set(app_state.request_client)
        .expect("Failed to set request client");

    utilities::GLOBAL_CONFIG
        .set(app_state.config)
        .expect("Failed to set global config");
}
