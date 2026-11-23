# Server mode

Server mode runs Escouade without a window on a Linux machine that stays on, and opens the same interface in a browser on your computer. This page explains how to install it, pair a browser with it, and what to do when the tunnel doesn’t open.

> [!WARNING]
> This is a first version. No release of Escouade ships the `escouade-server` program yet: you build it from source. It is meant for one user per server, on Linux x64 or arm64.

## What it is

On the server, the `escouade-server` program runs everything the app does: agents, Git, the board, terminals, the MCP server. It has no window and needs no desktop.

On your computer, a browser shows the interface. It is only a view: closing it stops nothing. You can open several, on several devices, and find the same agents.

The server never listens on the Internet. You reach it through an SSH tunnel, with the keys you already use to manage the machine, or through an HTTPS proxy you run yourself.

The desktop app doesn’t change, and it can’t connect to a server yet: in server mode, the client is a browser.

## Before you start

You need:

- a Linux machine (Ubuntu 22.04, Debian 12, or later) you reach by SSH;
- on that machine, your agents’ harness ([Claude Code](harness) for example) and `git`, installed for the account that will run the server;
- on your computer, a browser and the `ssh` command (present on Windows 10 and later, macOS and Linux).

### An account of their own for the agents

Agents have the rights of the account that runs the server. If that account has `sudo` or belongs to the `docker` group, they have the whole machine. On a server that hosts anything else, create a dedicated account with neither. From your administrator account:

```
sudo adduser --disabled-password --gecos "" escouade
sudo install -d -m 700 -o escouade -g escouade /home/escouade/.ssh
sudo install -m 600 -o escouade -g escouade ~/.ssh/authorized_keys /home/escouade/.ssh/
```

The last line copies your SSH keys: `ssh escouade@<host>` then signs in to that account. Do every following step from it, never as root.

## Build the server

On a machine of the same architecture as the server, with rustup, Node 22, git and a C compiler:

```
rustup target add wasm32-wasip1
npm ci && npm run build:web
sh scripts/escouade-js.sh
npx --yes tsx@4.23.15 store/scripts/seed.ts fetch
cd src-tauri && cargo build --release --locked --no-default-features --features server --bin escouade-server
```

You get the program `src-tauri/target/release/escouade-server` and the folder `dist-web/`, the page the server serves. Copy both to the server, for example to `~/.local/bin/` and `~/.local/share/escouade-web/`.

## Install the service

On the server, signed in by `ssh` with the agents’ account:

```
escouade-server install --web ~/.local/share/escouade-web
systemctl --user daemon-reload
systemctl --user enable --now escouade-server
loginctl enable-linger
```

`install` writes the service and prints the commands to run next; it runs nothing itself. `loginctl enable-linger` keeps the service running with no session open, and starts it again when the machine boots. If it is refused, run `sudo loginctl enable-linger <account>` from an administrator account.

The data is the app’s, in `~/.escouade`. One server runs per data folder: a second one refuses to start.

## Open the tunnel and pair your browser

1. On the server, run `escouade-server pair`. It prints the `ssh` line to run, the address to open and a code.
2. On your computer, run that `ssh -N -L 8765:… <host>` line and leave it running.
3. Open the address `pair` gave, `http://127.0.0.1:8765/`. The page shows “Pair this browser”.
4. Type the code and a name for your device.

The code works once and lasts 10 minutes. Five wrong codes cancel it: run `escouade-server pair` again.

> [!NOTE]
> To your browser, `http://127.0.0.1:8765/` and `http://localhost:8765/` are two different sites: a browser paired on one is not paired on the other. Always use the same address.

Keep that local port for Escouade alone. Your browser stores the device’s token there, and any other program served later on that port could read it.

## First steps in the browser

- **Add an account**: Settings › Accounts and harnesses › “Add an account…”. Claude Code starts in a terminal of the page, on the server. It shows an address: click it, sign in in your browser, then paste into the terminal the code the page gives you. See [accounts](accounts).
- **Create a project**: “New project” › “Browse…” shows the server’s folders, under the account’s home folder. “Clone here” clones a repository there.
- **Start an agent**, close the tab, open it again: the agent kept going.

## What differs from the app

| In the app | In the browser |
|---|---|
| Choosing a folder with the system’s picker | A browser of the server’s folders |
| Attaching a file by its path | The file is sent to the server |
| “Open” on a file an agent sent | The file downloads to your computer |
| System notifications | The browser’s notifications |
| Built-in update | None: you replace the program and the page, then restart the service |
| Dictation and reading aloud | None for now |
| Plugins “From a folder…” | None: the store and Git repositories remain |

After a network cut, the page reconnects by itself and finds your agents and terminals again, with what they wrote in the meantime. If the server changed version, it asks you to reload.

## Manage devices

Each paired browser has its own token. On the server:

- `escouade-server devices` lists the devices, with their name and when they were last seen;
- `escouade-server devices revoke <id>` cuts one off at once, open connections included.

> [!WARNING]
> A device’s token is worth a terminal on the server, under the service’s account. Revoke a lost device or a browser you no longer use.

If the command answers that the revocation could not be written, the device stays cut off while the server runs, but would come back when it restarts: free some space or repair the disk, then revoke again.

## Behind an HTTPS proxy

Instead of the tunnel, you can put the server behind your own proxy (Caddy, nginx). The service then listens on a port, and you declare the public address to it:

```
escouade-server install --port 8765 --origin https://escouade.example.com --web ~/.local/share/escouade-web
```

With Caddy:

```
escouade.example.com {
	reverse_proxy 127.0.0.1:8765
}
```

`escouade-server pair` then gives the address `https://escouade.example.com/`. On a port, any account of the machine can reach the server: keep this setup for a machine you alone use.

## What lives on the server

All of Escouade’s state is in `~/.escouade`: your projects, your conversations, and **your accounts’ credentials**, in plain text in files only the service’s account can read. A server has no keychain. Whoever reads that disk, one of its backups or that account’s SSH session has your accounts. See also [data](data).

## If the tunnel doesn’t open

**VS Code is connected to the same server (Remote-SSH).** VS Code forwards by itself the port of every `http://localhost:…` or `http://127.0.0.1:…` address shown in one of its terminals, including the one `pair` prints. The local port is then taken, and your `ssh -L` fails. Two ways out:

- tell VS Code to ignore that port, in its settings: `"remote.portsAttributes": { "8765": { "onAutoForward": "ignore" } }`, then stop the forward already open in its “Ports” panel;
- or install the service on a port (`escouade-server install --port 8765 --web …`) and let VS Code forward it.

**On Windows, `bind [127.0.0.1]:8765: Permission denied`.** This message often means the port is already taken. `Get-NetTCPConnection -State Listen -LocalPort 8765` shows by whom. Windows also reserves port ranges for Hyper-V, WSL 2 and Docker: `netsh interface ipv4 show excludedportrange protocol=tcp` lists them. Pick a local port outside those ranges, for example `ssh -N -L 18765:… <host>`, then open `http://127.0.0.1:18765/`.

**In PowerShell, `hostname contains invalid characters`.** A key path (`-i`) that contains spaces goes between quotes.

**The server refuses forwarding to a socket.** An SSH server set with `AllowStreamLocalForwarding no` refuses the tunnel to the service’s socket. Install the service on a port instead, and open the tunnel to that port: `ssh -N -L 8765:127.0.0.1:8765 <host>`.

**The service doesn’t start.** `journalctl --user -u escouade-server -n 50` shows why. See also [troubleshooting](troubleshooting).

## Remove everything

```
systemctl --user disable --now escouade-server
escouade-server uninstall
```

Then delete the program, the page’s folder and, to leave nothing behind, `~/.escouade`. If you created a dedicated account: `sudo loginctl disable-linger escouade && sudo deluser --remove-home escouade`.

## Known limits

- No desktop client that connects to a server yet, and no layout for phones.
- The test servers your agents start are not relayed to your computer.
- The interface’s state is shared between your devices: the last one to write wins.
- The server holds 16 connections at once; a very slow or dead connection can keep one for a while.
- Don’t open the desktop app on the data folder of a running server.
