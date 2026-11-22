//! `escouade-server`: the core without a window, its commands over a WebSocket and the web build (`--web <folder>`) on
//! 127.0.0.1, at `ESCOUADE_REMOTE_PORT`, for those who bring `ESCOUADE_REMOTE_TOKEN` (docs/REMOTE.md, « Le mode
//! serveur »). Stops on SIGINT or SIGTERM.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(escouade_lib::server_cli(args));
}
