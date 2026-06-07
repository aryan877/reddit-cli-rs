use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigFile {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub user_agent: Option<String>,
    pub scope: Option<String>,
}

pub struct Config {
    pub client_id: String,
    pub client_secret: Zeroizing<String>,
    pub username: String,
    pub password: Zeroizing<String>,
    pub user_agent: String,
    pub scope: String,
}

impl Config {
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let file = load_config_file(path)?;

        let client_id = env_or_file("REDDIT_CLIENT_ID", file.client_id)
            .context("missing REDDIT_CLIENT_ID or config client_id")?;
        let client_secret = env_or_file("REDDIT_CLIENT_SECRET", file.client_secret)
            .context("missing REDDIT_CLIENT_SECRET or config client_secret")?;
        let username = env_or_file("REDDIT_USERNAME", file.username)
            .context("missing REDDIT_USERNAME or config username")?;
        let password = env_or_file("REDDIT_PASSWORD", file.password)
            .context("missing REDDIT_PASSWORD or config password")?;
        let scope = env_or_file("REDDIT_SCOPE", file.scope)
            .unwrap_or_else(|| "identity read submit privatemessages".to_string());
        let user_agent = env_or_file("REDDIT_USER_AGENT", file.user_agent).unwrap_or_else(|| {
            format!(
                "macos:reddit-cli-rs:0.1.0 (by /u/{})",
                username.replace(['\n', '\r'], "")
            )
        });

        Ok(Self {
            client_id,
            client_secret: Zeroizing::new(client_secret),
            username,
            password: Zeroizing::new(password),
            user_agent,
            scope,
        })
    }

    pub fn default_path() -> Result<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .context("could not determine config directory")?;
        Ok(base.join("reddit-cli-rs").join("config.toml"))
    }

    pub fn write_template(force: bool) -> Result<PathBuf> {
        let path = Self::default_path()?;
        if path.exists() && !force {
            bail!(
                "config already exists at {}. Use --force to overwrite",
                path.display()
            );
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let template = ConfigFile {
            client_id: Some("your_client_id".to_string()),
            client_secret: Some("your_client_secret".to_string()),
            username: Some("your_username".to_string()),
            password: Some("your_password".to_string()),
            user_agent: Some("macos:reddit-cli-rs:0.1.0 (by /u/your_username)".to_string()),
            scope: Some("identity read submit privatemessages".to_string()),
        };
        let text = toml::to_string_pretty(&template)?;
        std::fs::write(&path, text)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(path)
    }
}

fn load_config_file(path: Option<&Path>) -> Result<ConfigFile> {
    let path = match path {
        Some(path) => path.to_path_buf(),
        None => Config::default_path()?,
    };
    if !path.exists() {
        return Ok(ConfigFile {
            client_id: None,
            client_secret: None,
            username: None,
            password: None,
            user_agent: None,
            scope: None,
        });
    }
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("failed reading {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("failed parsing {}", path.display()))
}

fn env_or_file(key: &str, file_value: Option<String>) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.is_empty())
        .or(file_value)
}
