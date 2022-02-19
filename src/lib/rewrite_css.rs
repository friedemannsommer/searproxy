use std::rc::Rc;

use url::Url;

use crate::lib::rewrite_url::{rewrite_url, RewriteUrlError};

#[derive(thiserror::Error, Debug)]
pub enum RewriteCssError {
    #[error("Not enough data")]
    Incomplete,
    #[error("UTF-8 decode failed")]
    DecodeFailed(#[from] std::str::Utf8Error),
    #[error("URL rewrite failed")]
    UrlRewrite(#[from] RewriteUrlError),
}

#[derive(Debug, Eq, PartialEq)]
enum MatchState {
    None,
    U,
    R,
    L,
    OpeningBracket,
    Quote(u8),
    ClosingBracket,
}

pub struct CssRewrite {
    base_url: Rc<url::Url>,
    buffer: Vec<u8>,
    last_index: usize,
    match_start: usize,
    match_state: MatchState,
    output: Vec<u8>,
    url_start: usize,
}

impl CssRewrite {
    pub fn new(base_url: Rc<Url>) -> Self {
        Self {
            base_url,
            buffer: Vec::new(),
            last_index: 0,
            match_start: 0,
            match_state: MatchState::None,
            output: Vec::new(),
            url_start: 0,
        }
    }

    pub fn write(&mut self, chunk: &[u8]) -> Result<(), RewriteCssError> {
        self.buffer.extend_from_slice(chunk);

        let result = self.parse_buffer();

        match result {
            Err(RewriteCssError::DecodeFailed(_) | RewriteCssError::UrlRewrite(_)) => result,
            _ => Ok(()),
        }
    }

    pub fn end(mut self) -> Result<Vec<u8>, RewriteCssError> {
        self.parse_buffer()?;
        Ok(self.output)
    }

    /**
     * This implementation will try to parse the following formats:
     * - url("https://www.example.com")
     * - url('https://www.example.com')
     * - url(https://www.example.com)
     **/
    fn parse_buffer(&mut self) -> Result<(), RewriteCssError> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let offset = self.last_index;

        for (index, byte) in self.buffer.iter().skip(offset).enumerate() {
            self.last_index = offset + index + 1;

            match byte {
                b'u' if self.match_state == MatchState::None => {
                    self.output.push(*byte);
                    self.match_start = offset + index;
                    self.match_state.next();
                }
                b'r' if self.match_state == MatchState::U => {
                    self.output.push(*byte);
                    self.match_state.next();
                }
                b'l' if self.match_state == MatchState::R => {
                    self.output.push(*byte);
                    self.match_state.next();
                }
                b'(' if self.match_state == MatchState::L => {
                    self.output.push(*byte);
                    self.url_start = offset + index + 1;
                    self.match_state.next();
                }
                b'"' if self.match_state == MatchState::OpeningBracket
                    || self.match_state == MatchState::Quote(b'"') =>
                {
                    match self.match_state {
                        MatchState::OpeningBracket => {
                            self.output.push(*byte);
                            self.url_start = offset + index + 1;
                            self.match_state = MatchState::Quote(b'"')
                        }
                        _ => {
                            self.output.extend_from_slice(
                                self.rewrite_url(self.url_start, offset + index)?.as_bytes(),
                            );
                            self.output.push(*byte);
                            self.match_state.next()
                        }
                    }
                }
                b'\''
                    if self.match_state == MatchState::OpeningBracket
                        || self.match_state == MatchState::Quote(b'\'') =>
                {
                    match self.match_state {
                        MatchState::OpeningBracket => {
                            self.output.push(*byte);
                            self.url_start = offset + index + 1;
                            self.match_state = MatchState::Quote(b'\'')
                        }
                        _ => {
                            self.output.extend_from_slice(
                                self.rewrite_url(self.url_start, offset + index)?.as_bytes(),
                            );
                            self.output.push(*byte);
                            self.match_state.next();
                        }
                    }
                }
                b')' if self.match_state.inside_brackets() => {
                    if self.match_state != MatchState::ClosingBracket {
                        self.output.extend_from_slice(
                            self.rewrite_url(self.url_start, offset + index)?.as_bytes(),
                        );
                    }

                    self.output.push(*byte);
                    self.match_state.reset()
                }
                b' ' | b'\n' | b'\r' | b'\t' => {
                    if self.match_state.whitespace_allowed() {
                        /* ignore any whitespace */
                    } else {
                        self.output.push(*byte);
                        self.match_state.reset()
                    }
                }
                _ => {
                    if self.match_state == MatchState::None || !self.match_state.inside_brackets() {
                        self.output.push(*byte);
                        self.match_state.reset()
                    }
                }
            }
        }

        if self.match_start != 0 {
            self.buffer.drain(0..self.match_start);
            self.last_index = self.last_index.saturating_sub(self.match_start);
            self.url_start = self.url_start.saturating_sub(self.match_start);
            self.match_start = 0
        } else if self.match_state == MatchState::None {
            self.buffer.clear();
            self.last_index = 0;
        }

        if self.match_state != MatchState::None {
            Err(RewriteCssError::Incomplete)
        } else {
            Ok(())
        }
    }

    fn rewrite_url(&self, start: usize, end: usize) -> Result<String, RewriteCssError> {
        Ok(rewrite_url(
            &self.base_url,
            std::str::from_utf8(&self.buffer[start..end])?,
        )?)
    }
}

impl MatchState {
    fn next(&mut self) {
        *self = match self {
            Self::None => Self::U,
            Self::U => Self::R,
            Self::R => Self::L,
            Self::L => Self::OpeningBracket,
            Self::OpeningBracket | Self::Quote(_) => Self::ClosingBracket,
            _ => Self::None,
        };
    }

    fn reset(&mut self) {
        *self = Self::None;
    }

    fn inside_brackets(&self) -> bool {
        matches!(
            self,
            Self::OpeningBracket | Self::Quote(_) | Self::ClosingBracket
        )
    }

    fn whitespace_allowed(&self) -> bool {
        matches!(
            self,
            Self::L | Self::OpeningBracket | Self::Quote(_) | Self::ClosingBracket
        )
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::lib::rewrite_css::CssRewrite;

    #[test]
    fn no_quotes_relative_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter =
            CssRewrite::new(Rc::new(url::Url::parse("https://www.example.com").unwrap()));

        rewriter.write(b"url(main.css)").unwrap();

        assert_eq!(std::str::from_utf8( rewriter.end().unwrap().as_slice()).unwrap(),"url(./?mortyurl=https%3A%2F%2Fwww.example.com%2Fmain.css&mortyhash=7d40cd69599262cfe009ac148491a37e9ec47dcf2386c2807bc2255fff6d5fa3)");
    }

    #[test]
    fn single_quotes_relative_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter =
            CssRewrite::new(Rc::new(url::Url::parse("https://www.example.com").unwrap()));

        rewriter.write(b"url('main.css')").unwrap();

        assert_eq!(std::str::from_utf8( rewriter.end().unwrap().as_slice()).unwrap(),"url('./?mortyurl=https%3A%2F%2Fwww.example.com%2Fmain.css&mortyhash=7d40cd69599262cfe009ac148491a37e9ec47dcf2386c2807bc2255fff6d5fa3')");
    }

    #[test]
    fn double_quotes_relative_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter =
            CssRewrite::new(Rc::new(url::Url::parse("https://www.example.com").unwrap()));

        rewriter.write(b"url(\"main.css\")").unwrap();

        assert_eq!(std::str::from_utf8( rewriter.end().unwrap().as_slice()).unwrap(),"url(\"./?mortyurl=https%3A%2F%2Fwww.example.com%2Fmain.css&mortyhash=7d40cd69599262cfe009ac148491a37e9ec47dcf2386c2807bc2255fff6d5fa3\")");
    }

    #[test]
    fn no_quotes_relative_n_5() {
        crate::lib::test_setup_hmac();

        let mut rewriter =
            CssRewrite::new(Rc::new(url::Url::parse("https://www.example.com").unwrap()));

        rewriter
            .write("url(main.css)".repeat(5).as_bytes())
            .unwrap();

        assert_eq!(std::str::from_utf8( rewriter.end().unwrap().as_slice()).unwrap(),"url(./?mortyurl=https%3A%2F%2Fwww.example.com%2Fmain.css&mortyhash=7d40cd69599262cfe009ac148491a37e9ec47dcf2386c2807bc2255fff6d5fa3)".repeat(5));
    }

    #[test]
    fn single_quotes_relative_n_5() {
        crate::lib::test_setup_hmac();

        let mut rewriter =
            CssRewrite::new(Rc::new(url::Url::parse("https://www.example.com").unwrap()));

        rewriter
            .write("url('main.css')".repeat(5).as_bytes())
            .unwrap();

        assert_eq!(std::str::from_utf8( rewriter.end().unwrap().as_slice()).unwrap(),"url('./?mortyurl=https%3A%2F%2Fwww.example.com%2Fmain.css&mortyhash=7d40cd69599262cfe009ac148491a37e9ec47dcf2386c2807bc2255fff6d5fa3')".repeat(5));
    }

    #[test]
    fn double_quotes_relative_n_5() {
        crate::lib::test_setup_hmac();

        let mut rewriter =
            CssRewrite::new(Rc::new(url::Url::parse("https://www.example.com").unwrap()));

        rewriter
            .write("url(\"main.css\")".repeat(5).as_bytes())
            .unwrap();

        assert_eq!(std::str::from_utf8( rewriter.end().unwrap().as_slice()).unwrap(),"url(\"./?mortyurl=https%3A%2F%2Fwww.example.com%2Fmain.css&mortyhash=7d40cd69599262cfe009ac148491a37e9ec47dcf2386c2807bc2255fff6d5fa3\")".repeat(5));
    }

    #[test]
    fn chunked_single_quote_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter =
            CssRewrite::new(Rc::new(url::Url::parse("https://www.example.com").unwrap()));

        rewriter.write(b" ".repeat(2048).as_slice()).unwrap();
        rewriter.write(b"ur").unwrap();
        rewriter.write(b"l ( ").unwrap();
        rewriter.write(b" ".repeat(2048).as_slice()).unwrap();
        rewriter.write(b"'").unwrap();
        rewriter.write(b"https://www.").unwrap();
        rewriter.write(b"example.com").unwrap();
        rewriter.write(b"/main.css'").unwrap();
        rewriter.write(b"  ) ").unwrap();
        rewriter.write(b" ".repeat(2048).as_slice()).unwrap();

        assert!(std::str::from_utf8( rewriter.end().unwrap().as_slice()).unwrap().contains("url('./?mortyurl=https%3A%2F%2Fwww.example.com%2Fmain.css&mortyhash=7d40cd69599262cfe009ac148491a37e9ec47dcf2386c2807bc2255fff6d5fa3')"));
    }

    #[test]
    fn simple_single_quote_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter =
            CssRewrite::new(Rc::new(url::Url::parse("https://www.example.com").unwrap()));

        rewriter
            .write(b"url('https://www.example.com/main.css')")
            .unwrap();

        assert_eq!(std::str::from_utf8( rewriter.end().unwrap().as_slice()).unwrap(), "url('./?mortyurl=https%3A%2F%2Fwww.example.com%2Fmain.css&mortyhash=7d40cd69599262cfe009ac148491a37e9ec47dcf2386c2807bc2255fff6d5fa3')");
    }

    #[test]
    fn chunked_double_quote_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter =
            CssRewrite::new(Rc::new(url::Url::parse("https://www.example.com").unwrap()));

        rewriter.write(b" ".repeat(2048).as_slice()).unwrap();
        rewriter.write(b"ur").unwrap();
        rewriter.write(b"l ( ").unwrap();
        rewriter.write(b" ".repeat(2048).as_slice()).unwrap();
        rewriter.write(b"\"").unwrap();
        rewriter.write(b"https://www.").unwrap();
        rewriter.write(b"example.com").unwrap();
        rewriter.write(b"/main.css\"").unwrap();
        rewriter.write(b"  ) ").unwrap();
        rewriter.write(b" ".repeat(2048).as_slice()).unwrap();

        assert!(std::str::from_utf8( rewriter.end().unwrap().as_slice()).unwrap().contains("url(\"./?mortyurl=https%3A%2F%2Fwww.example.com%2Fmain.css&mortyhash=7d40cd69599262cfe009ac148491a37e9ec47dcf2386c2807bc2255fff6d5fa3\")"));
    }

    #[test]
    fn simple_double_quote_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter =
            CssRewrite::new(Rc::new(url::Url::parse("https://www.example.com").unwrap()));

        rewriter
            .write(b"url(\"https://www.example.com/main.css\")")
            .unwrap();

        assert_eq!(std::str::from_utf8( rewriter.end().unwrap().as_slice()).unwrap(), "url(\"./?mortyurl=https%3A%2F%2Fwww.example.com%2Fmain.css&mortyhash=7d40cd69599262cfe009ac148491a37e9ec47dcf2386c2807bc2255fff6d5fa3\")");
    }

    #[test]
    fn chunked_no_quotes_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter =
            CssRewrite::new(Rc::new(url::Url::parse("https://www.example.com").unwrap()));

        rewriter.write(b" ".repeat(2048).as_slice()).unwrap();
        rewriter.write(b"ur").unwrap();
        rewriter.write(b"l (").unwrap();
        rewriter.write(b"https://www.").unwrap();
        rewriter.write(b"example.com").unwrap();
        rewriter.write(b"/main.css").unwrap();
        rewriter.write(b") ").unwrap();
        rewriter.write(b" ".repeat(2048).as_slice()).unwrap();

        assert!(std::str::from_utf8( rewriter.end().unwrap().as_slice()).unwrap().contains("url(./?mortyurl=https%3A%2F%2Fwww.example.com%2Fmain.css&mortyhash=7d40cd69599262cfe009ac148491a37e9ec47dcf2386c2807bc2255fff6d5fa3)"));
    }

    #[test]
    fn simple_no_quotes_n_1() {
        crate::lib::test_setup_hmac();

        let mut rewriter =
            CssRewrite::new(Rc::new(url::Url::parse("https://www.example.com").unwrap()));

        rewriter
            .write(b"url(https://www.example.com/main.css)")
            .unwrap();

        assert_eq!(std::str::from_utf8( rewriter.end().unwrap().as_slice()).unwrap(), "url(./?mortyurl=https%3A%2F%2Fwww.example.com%2Fmain.css&mortyhash=7d40cd69599262cfe009ac148491a37e9ec47dcf2386c2807bc2255fff6d5fa3)");
    }
}
