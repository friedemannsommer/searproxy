pub const FAVICON_ICO_FILE: &[u8] = include_bytes!("favicon.ico");
pub const FAVICON_PNG_16_FILE: &[u8] = include_bytes!("favicon-16x16.png");
pub const FAVICON_PNG_32_FILE: &[u8] = include_bytes!("favicon-32x32.png");
pub const HEADER_STYLESHEET: &str = include_str!(concat!(env!("OUT_DIR"), "/header.css"));
pub const HEADER_STYLESHEET_HASH: &str = include_str!(concat!(env!("OUT_DIR"), "/header.hash"));
pub const MAIN_STYLESHEET: &str = include_str!(concat!(env!("OUT_DIR"), "/main.css"));
pub const MAIN_STYLESHEET_HASH: &str = include_str!(concat!(env!("OUT_DIR"), "/main.hash"));
pub const ROBOTS_FILE: &[u8] = include_bytes!("robots.txt");
