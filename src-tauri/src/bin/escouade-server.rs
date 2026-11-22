//! `escouade-server`: the core without a window, its commands over a WebSocket and the web build (`--web <folder>`) on
//! a Unix socket (`--socket`) or 127.0.0.1's port (`--port`, `ESCOUADE_REMOTE_PORT`), for the devices paired by
//! `escouade-server pair`, one per data folder; `install` writes its systemd unit (docs/REMOTE.md, « Le mode serveur »).
//! Stops on SIGINT or SIGTERM.

// `SEED` and `SEED_STAMP`: the seed of the plugins, in this program alone (build.rs).
include!(concat!(env!("OUT_DIR"), "/seed.rs"));

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seed = escouade_lib::Seed {
        files: SEED,
        stamp: SEED_STAMP,
    };
    std::process::exit(escouade_lib::server_cli(args, seed));
}
