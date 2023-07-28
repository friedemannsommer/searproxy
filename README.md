# SearProxy

A [SearX][1] / [SearXNG][2] compatible web content sanitizer proxy

> This implementation is heavily inspired by [Morty](https://github.com/asciimoo/morty).

## Usage

```shell
searproxy [OPTIONS] --hmac-secret <HMAC_SECRET> --listen <LISTEN_ADDRESS>
```

## Options

* `--lazy-images` - Enable IMG element rewriting with "lazy" loading. (default: false)
* `-f` / `--follow-redirect` - Allow "Location" response header following (default: false)
* `-l` / `--listen` - <IPv4 / IPv6>:port or socket to listen on
* `-p` / `--proxy-address` - HTTP(s) / SOCKS5 proxy for outgoing HTTP(s) requests
* `-s` / `--hmac-secret` - Base64 encoded string to use as HMAC 256 secret
* `-t` / `--request-timeout` - Timeout in seconds to wait for a request to complete (default: 5s)
* `-v` / `--log-level` - Log level to use (default: WARN)
* `-w` / `--worker-count` - Worker thread count for handling incoming HTTP requests (default: CPU core count)
* `-r` / `--permitted-ip-range` - Permitted IP (v4, v6) ranges (default: "global")
* `-h` / `--help` - Print help information
* `-V` / `--version` - Print version information

## ENV options

> Passed options will override ENV options

* `SEARPROXY_LAZY_IMAGES` - Enable IMG element rewriting with "lazy" loading. (default: false)
* `SEARPROXY_FOLLOW_REDIRECTS` - Allow "Location" response header following (default: false)
* `SEARPROXY_LISTEN` - <IPv4 / IPv6>:port or socket to listen on
* `HTTP_PROXY` - HTTP(s) / SOCKS5 proxy for outgoing HTTP(s) requests
* `SEARPROXY_HMAC_SECRET` - Base64 encoded string to use as HMAC 256 secret
* `SEARPROXY_REQUEST_TIMEOUT` - Timeout in seconds to wait for a request to complete (default: 5s)
* `SEARPROXY_LOG_LEVEL` - Log level to use (default: WARN)
* `SEARPROXY_WORKER_COUNT` - Worker thread count for handling incoming HTTP requests (default: CPU core count)
* `SEARPROXY_PERMITTED_IP_RANGE` - Permitted IP (v4, v6) ranges (default: "global")

## Open source licenses

A list of licenses for the projects used in SearProxy can be found
here: [friedemannsommer.github.io/searproxy/licenses.html](https://friedemannsommer.github.io/searproxy/licenses.html).

This product includes software developed by the OpenSSL Project for use in the OpenSSL
Toolkit. ([www.openssl.org](https://www.openssl.org/))

[1]: https://github.com/searx/searx
[2]: https://github.com/searxng/searxng
