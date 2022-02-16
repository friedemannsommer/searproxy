use clap::Parser;

/// A SearX[NG] compatible content sanitizer proxy
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Allow "Location" response header following.
    #[clap(short, long, env = "SEARPROXY_FOLLOW_REDIRECTS")]
    pub follow_redirect: bool,
    /// Base64 encoded string to use as HMAC 256 secret.
    #[clap(short = 's', long, env = "SEARPROXY_HMAC_SECRET")]
    pub hmac_secret: String,
    /// <IPv4 / IPv6>:port or socket to listen on.
    #[clap(short, long, env = "SEARPROXY_LISTEN")]
    pub listen: String,
    /// Log level to use. Keep in mind that this can include PII.
    /// Possible values include: "off", "error", "warn", "info", "debug", "trace".
    #[clap(short = 'v', long, env = "SEARPROXY_LOG_LEVEL", default_value_t = log::LevelFilter::Warn)]
    pub log_level: log::LevelFilter,
    /// Use a HTTP(s) / SOCKS5 proxy for outgoing HTTP(s) requests.
    /// Examples: "http://exam.ple", "https://exam.ple", "socks5://exam.ple", "socks5h://exam.ple"
    #[clap(short, long, env = "HTTP_PROXY")]
    pub proxy_address: Option<String>,
    /// Timeout in seconds to wait for a request to complete.
    #[clap(
        short = 't',
        long,
        env = "SEARPROXY_REQUEST_TIMEOUT",
        default_value_t = 5
    )]
    pub request_timeout: u8,
}
