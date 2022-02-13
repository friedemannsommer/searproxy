use std::borrow::Cow;

#[derive(Debug)]
pub struct Config<'address, 'secret, 'proxy> {
    pub follow_redirect: bool,
    pub hmac_secret: Cow<'secret, [u8]>,
    pub listen_address: Cow<'address, str>,
    pub log_level: tracing::Level,
    pub proxy_address: Option<Cow<'proxy, str>>,
    pub request_timeout: u8,
}
