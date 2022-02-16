# searproxy

A SearX[NG] compatible content sanitizer proxy

> This implementation is heavily inspired by [Morty](https://github.com/asciimoo/morty).

## Usage

```shell
searproxy [OPTIONS] --hmac-secret <HMAC_SECRET> --listen <LISTEN_ADDRESS>
```

## Options

* `-f` / `--follow-redirect` - Allow "Location" response header following (default: false)
* `-h` / `--help` - Print help information
* `-l` / `--listen` - address:port or socket to listen on
* `-p` / `--proxy-address` - HTTP(s) / SOCKS5 proxy for outgoing HTTP(s) requests
* `-s` / `--hmac-secret` - Base64 encoded string to use as HMAC 256 secret
* `-t` / `--request-timeout` - Timeout in seconds to wait for a request to complete (default: 5s)
* `-v` / `--log-level` - Log level to use (default: WARN)
* `-V` / `--version` - Print version information

## ENV options

> Passed options will override ENV options

* `SEARPROXY_FOLLOW_REDIRECTS` - Allow "Location" response header following (default: false)
* `SEARPROXY_HMAC_SECRET` - Base64 encoded string to use as HMAC 256 secret
* `SEARPROXY_LISTEN` - address:port or socket to listen on
* `SEARPROXY_LOG_LEVEL` - Log level to use (default: WARN)
* `HTTP_PROXY` - HTTP(s) / SOCKS5 proxy for outgoing HTTP(s) requests
* `SEARPROXY_REQUEST_TIMEOUT` - Timeout in seconds to wait for a request to complete (default: 5s)