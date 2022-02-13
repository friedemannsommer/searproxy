use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Allow "Location" response header following
    #[clap(short, long, env = "SEARPROXY_FOLLOW_REDIRECTS")]
    pub follow_redirect: bool,
    /// Base64 encoded string to use as HMAC 256 secret
    #[clap(short = 's', long, env = "SEARPROXY_HMAC_SECRET")]
    pub hmac_secret: String,
    /// Address and port to listen on (HTTP)
    #[clap(short, long, env = "SEARPROXY_LISTEN_ADDRESS")]
    pub listen_address: String,
    /// Log level to use. WARNING: anything below WARN can leak PII
    #[clap(short = 'v', long, env = "SEARPROXY_LOG_LEVEL", default_value_t = tracing::Level::WARN)]
    pub log_level: tracing::Level,
    /// Use a HTTP(s) / SOCKS5 proxy for outgoing HTTP(s) requests
    #[clap(short, long, env = "HTTP_PROXY")]
    pub proxy_address: Option<String>,
    /// Timeout in seconds to wait for a request to complete
    #[clap(
        short = 't',
        long,
        env = "SEARPROXY_REQUEST_TIMEOUT",
        default_value_t = 5
    )]
    pub request_timeout: u8,
}
