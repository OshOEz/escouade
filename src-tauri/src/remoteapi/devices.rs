//! The devices paired with `escouade-server` (docs/REMOTE.md, « Le mode serveur »): each its own token, 32 random bytes
//! only the device holds, kept here hashed (SHA-256) in `devices.json`, readable by its owner alone and written by
//! replacement, compared in constant time. A pairing code (`escouade-server pair`) serves once, for 10 minutes, one
//! alive at a time; a wrong try does not cancel it. A device revoked is refused at once (the server closes its connections).

use crate::paths;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const FILE: &str = "devices.json";
/// How long a pairing code lives.
pub const CODE_LIFE: Duration = Duration::from_secs(10 * 60);
/// The longest name kept, in characters.
pub const NAME_MAX: usize = 64;
/// A code's characters: Crockford's base32 (no I, L, O, U), 5 bits each, 40 bits in 8.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// How often a device's last visit is written down at most.
const SEEN_EVERY: chrono::TimeDelta = chrono::TimeDelta::seconds(60);

/// A device as `escouade-server devices` shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub paired_at: DateTime<Utc>,
    pub last_seen: Option<DateTime<Utc>>,
}

/// A device in the file: its token's hash, never the token.
#[derive(Serialize, Deserialize)]
struct Kept {
    #[serde(flatten)]
    device: Device,
    hash: String,
}

/// The code to type in the browser, `XXXX-XXXX`, and how long it lives.
#[derive(Debug)]
pub struct PairCode {
    pub code: String,
    pub expires_in: Duration,
}

/// A code refused: wrong, expired, used, cancelled or none asked; the browser is told no more.
#[derive(Debug, PartialEq, Eq)]
pub struct Refused;

struct Pending {
    /// Without its dash.
    code: String,
    until: Instant,
}

#[derive(Default)]
struct State {
    kept: Vec<Kept>,
    pending: Option<Pending>,
}

pub struct Devices {
    file: PathBuf,
    state: Mutex<State>,
    /// Why the file could not be read at the start, if it exists: it is then never written over.
    unreadable: Option<String>,
}

impl Devices {
    /// The devices of `dir`'s `devices.json`: none when it is missing, or damaged (said in one line of the log). Unread
    /// for another reason (its rights, say), none either, and the file is left as it is: no pairing writes over it.
    pub fn open(dir: &Path) -> Devices {
        let file = dir.join(FILE);
        let mut unreadable = None;
        let kept = match std::fs::read(&file) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => {
                log::warn!(
                    "remote API: {} cannot be read ({e}): no device paired, and none can be until it is fixed",
                    file.display()
                );
                unreadable = Some(e.to_string());
                Vec::new()
            }
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                log::warn!(
                    "remote API: {} is unreadable ({e}): no device paired",
                    file.display()
                );
                Vec::new()
            }),
        };
        Devices {
            file,
            state: Mutex::new(State {
                kept,
                pending: None,
            }),
            unreadable,
        }
    }

    /// The folder of the file.
    pub fn dir(&self) -> &Path {
        self.file.parent().expect("a file in a folder")
    }

    /// A new code, in place of the one under way.
    pub fn pair_code(&self) -> PairCode {
        let mut bytes = [0u8; 8];
        getrandom::fill(&mut bytes).expect("the system's random generator");
        let code: String = bytes
            .iter()
            .map(|b| ALPHABET[(b & 31) as usize] as char)
            .collect();
        let shown = format!("{}-{}", &code[..4], &code[4..]);
        self.state.lock().pending = Some(Pending {
            code,
            until: Instant::now() + CODE_LIFE,
        });
        PairCode {
            code: shown,
            expires_in: CODE_LIFE,
        }
    }

    /// The device `name` paired with `code`: its token, which only it will hold.
    pub fn redeem(&self, code: &str, name: &str) -> Result<String, Refused> {
        let typed: String = code
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-')
            .collect::<String>()
            .to_ascii_uppercase()
            // Crockford's base32 reads O as 0, I and L as 1.
            .replace('O', "0")
            .replace(['I', 'L'], "1");
        let mut state = self.state.lock();
        let pending = state.pending.as_mut().ok_or(Refused)?;
        if Instant::now() >= pending.until {
            state.pending = None;
            return Err(Refused);
        }
        // A wrong code does not cancel the pairing: `POST /pair` is open to anyone reaching the server, who could
        // otherwise kill every code minted. 40 bits outlast the 10 minutes of a code at any rate a server answers.
        if !same(typed.as_bytes(), pending.code.as_bytes()) {
            return Err(Refused);
        }
        state.pending = None;
        let token = escouade_mcp::new_token();
        let now = Utc::now();
        let device = Device {
            id: new_id(),
            name: clean(name),
            paired_at: now,
            last_seen: Some(now),
        };
        log::info!("remote API: device {} ({}) paired", device.id, device.name);
        state.kept.push(Kept {
            device,
            hash: hash(&token),
        });
        if let Err(e) = self.save(&state) {
            // Not written, not paired: it would be lost at the next start.
            state.kept.pop();
            log::error!("remote API: {} not written: {e}", self.file.display());
            return Err(Refused);
        }
        Ok(token)
    }

    /// The id of the device whose token is `token`; every device compared, each in constant time.
    pub fn check(&self, token: &str) -> Option<String> {
        let presented = hash(token);
        let mut state = self.state.lock();
        let mut found = None;
        for (i, kept) in state.kept.iter().enumerate() {
            if same(kept.hash.as_bytes(), presented.as_bytes()) {
                found = Some(i);
            }
        }
        let kept = &mut state.kept[found?];
        let now = Utc::now();
        let stale = kept.device.last_seen.is_none_or(|t| now - t >= SEEN_EVERY);
        kept.device.last_seen = Some(now);
        let id = kept.device.id.clone();
        if stale {
            if let Err(e) = self.save(&state) {
                log::warn!("remote API: {} not written: {e}", self.file.display());
            }
        }
        Some(id)
    }

    /// Whether `id` is still paired.
    pub fn known(&self, id: &str) -> bool {
        self.state.lock().kept.iter().any(|k| k.device.id == id)
    }

    pub fn list(&self) -> Vec<Device> {
        let state = self.state.lock();
        state.kept.iter().map(|k| k.device.clone()).collect()
    }

    /// Forgets `id`: false if it was not paired. An error when the file could not be written: `id` is refused until the
    /// server restarts, then read again from the file, and the caller must say so.
    pub fn revoke(&self, id: &str) -> Result<bool, String> {
        let mut state = self.state.lock();
        let before = state.kept.len();
        state.kept.retain(|k| k.device.id != id);
        if state.kept.len() == before {
            return Ok(false);
        }
        log::info!("remote API: device {id} revoked");
        self.save(&state).map_err(|e| {
            let why = format!("{} not written: {e}", self.file.display());
            log::error!("remote API: {why}");
            why
        })?;
        Ok(true)
    }

    fn save(&self, state: &State) -> std::io::Result<()> {
        if let Some(why) = &self.unreadable {
            return Err(std::io::Error::other(format!(
                "it could not be read at the start ({why}): not written over"
            )));
        }
        let bytes = serde_json::to_vec_pretty(&state.kept).expect("plain data");
        paths::write_private(&self.file, &bytes)
    }
}

/// SHA-256 of `token`, in hex.
fn hash(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Whether `a` and `b` are the same, in a time that does not depend on where they differ.
fn same(a: &[u8], b: &[u8]) -> bool {
    let differ = a.iter().zip(b).fold(0u8, |d, (x, y)| d | (x ^ y));
    std::hint::black_box(differ) == 0 && a.len() == b.len()
}

/// 8 hex characters, what `escouade-server devices revoke` takes.
fn new_id() -> String {
    let mut bytes = [0u8; 4];
    getrandom::fill(&mut bytes).expect("the system's random generator");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A name of one line, `NAME_MAX` characters at most, without the control or format characters (U+202E would turn
/// `escouade-server devices`'s line around). Empty: no name, which `POST /pair` refuses.
pub fn clean(name: &str) -> String {
    let kept: String = name
        .chars()
        .filter(|c| !c.is_control() && !format_char(*c))
        .collect();
    kept.trim().chars().take(NAME_MAX).collect()
}

/// Whether `c` is of Unicode's category Cf (format), as of Unicode 15.
fn format_char(c: char) -> bool {
    matches!(c as u32,
        0xAD | 0x600..=0x605 | 0x61C | 0x6DD | 0x70F | 0x890..=0x891 | 0x8E2 | 0x180E | 0x200B..=0x200F
        | 0x202A..=0x202E | 0x2060..=0x2064 | 0x2066..=0x206F | 0xFEFF | 0xFFF9..=0xFFFB | 0x110BD | 0x110CD
        | 0x13430..=0x1343F | 0x1BCA0..=0x1BCA3 | 0x1D173..=0x1D17A | 0xE0001 | 0xE0020..=0xE007F)
}

#[cfg(test)]
#[path = "devices_tests.rs"]
mod tests;
