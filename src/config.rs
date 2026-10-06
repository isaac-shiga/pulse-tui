use std::{fs, io, io::Write, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::api::Env;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    pub name: String,
    pub email: String,
    pub nin: String,
    pub bvn: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Beneficiary {
    pub person: Person,
    pub bank_code: String,
    pub bank_name: String,
    pub account_number: String,
    pub account_name: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub env: Env,
    pub test_key: String,
    pub live_key: String,
    pub payers: Vec<Person>,
    pub beneficiaries: Vec<Beneficiary>,
}

impl Config {
    pub fn path() -> PathBuf {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
            });
        base.join("pulse-tui").join("config.json")
    }

    pub fn load() -> Self {
        fs::read_to_string(Self::path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Writes the file with owner-only access, because it holds keys, NINs and BVNs.
    pub fn save(&self) -> io::Result<()> {
        let path = Self::path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            opts.mode(0o600);
            if path.exists() {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
            }
        }
        opts.open(&path)?
            .write_all(serde_json::to_string_pretty(self)?.as_bytes())
    }

    /// The saved key, or the `PULSE_TEST_KEY` / `PULSE_LIVE_KEY` variable.
    pub fn key(&self, env: Env) -> String {
        let (saved, var) = match env {
            Env::Test => (&self.test_key, "PULSE_TEST_KEY"),
            Env::Live => (&self.live_key, "PULSE_LIVE_KEY"),
        };
        if saved.is_empty() {
            std::env::var(var).unwrap_or_default()
        } else {
            saved.clone()
        }
    }
}
