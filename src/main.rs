use std::str::FromStr;

mod assets;
mod lib;
mod model;
mod server;
mod templates;

fn main() {
    let config = get_config();

    init_logging(&config);
    log::debug!("{:?}", &config);
    set_shared_values(config);
    server::start_http_service();
}

fn get_config() -> model::Config<'static, 'static> {
    use clap::Parser;

    let args: model::CliArgs = model::CliArgs::parse();

    model::Config {
        follow_redirect: args.follow_redirect,
        hmac_secret: std::borrow::Cow::Owned(
            base64::decode(&args.hmac_secret).expect("HMAC secret couldn't be [base64] decoded"),
        ),
        listen: parse_socket_listener(&args.listen),
        log_level: args.log_level,
        request_timeout: args.request_timeout,
        proxy_address: args.proxy_address.map(std::borrow::Cow::Owned),
    }
}

fn parse_socket_listener(input: &str) -> model::SocketListener {
    if let Ok(address) = std::net::SocketAddr::from_str(input) {
        model::SocketListener::Tcp(address)
    } else if let Ok(path) = std::path::PathBuf::from_str(input) {
        model::SocketListener::Unix(path)
    } else {
        panic!("Listener could not be parsed: '{}'", input)
    }
}

fn init_logging(config: &model::Config) {
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
    if lib::HMAC
        .set(hmac_sha256::HMAC::new(config.hmac_secret.as_ref()))
        .is_err()
    {
        panic!("Failed to set HMAC instance");
    }

    let mut request_client_builder = reqwest::Client::builder()
        .referer(false)
        .deflate(true)
        .gzip(true)
        .brotli(true)
        .redirect(if config.follow_redirect {
            reqwest::redirect::Policy::limited(15)
        } else {
            reqwest::redirect::Policy::none()
        })
        .tcp_nodelay(true)
        .tcp_keepalive(None)
        .timeout(std::time::Duration::from_secs(
            config.request_timeout as u64,
        ))
        .connect_timeout(std::time::Duration::from_secs(
            config.request_timeout as u64,
        ))
        .user_agent(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:91.0) Gecko/20100101 Firefox/91.0",
        );

    if let Some(proxy_address) = config.proxy_address.as_deref() {
        request_client_builder = request_client_builder
            .proxy(reqwest::Proxy::all(proxy_address).expect("Can't use given proxy config"));
    }

    lib::REQUEST_CLIENT
        .set(
            request_client_builder
                .build()
                .expect("Request client initialization failed"),
        )
        .expect("Failed to set request client");

    lib::GLOBAL_CONFIG
        .set(config)
        .expect("Failed to set global config");
}
