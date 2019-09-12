# minitokio

A toy tokio-like runtime capable of executing async-await futures.
Starts with `libstd` and [mio](https://github.com/tokio-rs/mio) and provides a reactor,
a multithreaded futures executor and async `TcpStream`/`TcpListener` wrappers.

The purpose of this repo is to understand how all pieces fit together in an async runtime.
Don't use for anything serious, use [tokio](https://tokio.rs/) instead.

## Examples

See `examples/http_server.rs` for a simple HTTP server. Run with `cargo run --release --example http_server`
and point your browser or your favorite load testing tool at `127.0.0.1:8000`.
