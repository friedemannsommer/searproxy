use std::borrow::Cow;

use crate::model::ip_range::PermittedIpRange;

#[derive(Debug)]
pub enum SocketListener {
    Tcp(std::net::SocketAddr),
    #[cfg(unix)]
    Unix(std::path::PathBuf),
}

#[derive(Debug)]
pub struct Config<'secret, 'proxy> {
    pub connect_timeout: u8,
    pub follow_redirects: bool,
    pub hmac_secret: Cow<'secret, [u8]>,
    pub lazy_images: bool,
    pub listen: SocketListener,
    pub log_level: log::LevelFilter,
    pub permitted_ip_range: PermittedIpRange,
    pub proxy_address: Option<Cow<'proxy, str>>,
    pub request_timeout: Option<u16>,
    pub worker_count: u8,
}
