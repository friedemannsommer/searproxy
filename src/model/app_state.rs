use crate::{model::Config, utilities::HmacInstance};

#[derive(thiserror::Error, Debug)]
pub enum AppStateError {
    #[error("Invalid HMAC secret")]
    Hmac(#[from] hmac::digest::InvalidLength),
    #[error("Failed to create request client")]
    RequestClient(#[from] reqwest::Error),
}

pub struct AppState<'secret, 'proxy> {
    pub config: Config<'secret, 'proxy>,
    pub hmac: HmacInstance,
    pub request_client: reqwest::Client,
}

impl<'secret, 'proxy> TryFrom<Config<'secret, 'proxy>> for AppState<'secret, 'proxy> {
    type Error = AppStateError;

    fn try_from(config: Config<'secret, 'proxy>) -> Result<Self, Self::Error> {
        use hmac::digest::KeyInit;

        Ok(Self {
            hmac: hmac::Hmac::new_from_slice(config.hmac_secret.as_ref())?,
            request_client: {
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
                    request_client_builder =
                        request_client_builder.proxy(reqwest::Proxy::all(proxy_address)?);
                }

                request_client_builder.build()?
            },
            config,
        })
    }
}
