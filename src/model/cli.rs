use crate::model::PermittedIpRange;

const ABOUT_WITH_LICENSE: &str = "This is a SearX & SearXNG compatible web proxy which \
excludes potentially malicious HTML tags. It also rewrites links to external resources \
to prevent leaks.

This program is free software: you can redistribute it and/or modify it under the terms \
of the GNU Affero General Public License as published by the Free Software Foundation, \
either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; \
without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.

See the GNU Affero General Public License for more details:
<https://www.gnu.org/licenses/agpl-3.0.txt>

This product includes software developed by the OpenSSL Project for use in \
the OpenSSL Toolkit. <https://www.openssl.org/>";

/// A SearX[NG] compatible web content sanitizer proxy.
/// This program comes with ABSOLUTELY NO WARRANTY;
/// This is free software, and you are welcome to redistribute it under certain conditions;
/// type `--help` for more details.
#[derive(clap::Parser, Debug)]
#[clap(version, about, long_about = Some(ABOUT_WITH_LICENSE))]
pub struct Cli {
    /// Allow "Location" response header following.
    #[clap(short, long, env = "SEARPROXY_FOLLOW_REDIRECTS")]
    pub follow_redirects: bool,
    /// Base64 encoded string to use as HMAC 256 secret.
    #[clap(short = 's', long, env = "SEARPROXY_HMAC_SECRET")]
    pub hmac_secret: String,
    /// Enable IMG element rewriting with "lazy" loading.
    /// Since this can be used to measure the clients scroll position, it's disabled by default.
    #[clap(long, env = "SEARPROXY_LAZY_IMAGES")]
    pub lazy_images: bool,
    /// <IPv4 / IPv6>:port or socket to listen on.
    #[clap(short, long, env = "SEARPROXY_LISTEN")]
    pub listen: String,
    /// Log level to use. Keep in mind that this can include PII.
    /// Possible values include: "off", "error", "warn", "info", "debug", "trace".
    #[clap(short = 'v', long, env = "SEARPROXY_LOG_LEVEL", default_value_t = log::LevelFilter::Warn)]
    pub log_level: log::LevelFilter,
    /// Permitted IP (v4, v6) ranges
    /// Possible values include: "none", "global", "private", "local".
    #[clap(short = 'r', long, env = "SEARPROXY_PERMITTED_IP_RANGE", default_value_t = PermittedIpRange::None)]
    pub permitted_ip_range: PermittedIpRange,
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
    /// Worker thread count for handling incoming HTTP requests.
    #[clap(short = 'w', long, env = "SEARPROXY_WORKER_COUNT", default_value_t = 0)]
    pub worker_count: u8,
}

#[cfg(test)]
mod tests {
    use super::Cli;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;

        Cli::command().debug_assert()
    }
}
