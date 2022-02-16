use std::borrow::Cow;

#[derive(Debug)]
pub enum SocketListener {
    Tcp(std::net::SocketAddr),
    Unix(std::path::PathBuf),
}

#[derive(Debug)]
pub struct Config<'secret, 'proxy> {
    pub follow_redirect: bool,
    pub hmac_secret: Cow<'secret, [u8]>,
    pub listen: SocketListener,
    pub log_level: tracing::Level,
    pub proxy_address: Option<Cow<'proxy, str>>,
    pub request_timeout: u8,
}
