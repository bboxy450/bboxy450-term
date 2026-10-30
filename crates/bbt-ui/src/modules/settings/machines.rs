// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Settings-owned machine choices, also used to prefill the launcher.
use serde::{Deserialize, Serialize};

pub const FIELDS: [(&str, &str); 6] = [
    ("via", "Via"),
    ("connect", "Connect"),
    ("port", "Port"),
    ("key", "Key"),
    ("directory", "Directory"),
    ("routes", "Routes (one per line)"),
];
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Machine {
    pub via: String,
    pub connect: String,
    pub port: String,
    pub key: String,
    pub directory: String,
    pub routes: Vec<String>,
    pub recent_directories: Vec<String>,
}
impl Default for Machine {
    fn default() -> Self {
        Self {
            via: "ssh".into(),
            connect: String::new(),
            port: "22".into(),
            key: String::new(),
            directory: "~".into(),
            routes: vec![],
            recent_directories: vec![],
        }
    }
}
impl Machine {
    pub fn value(&self, key: &str) -> String {
        match key {
            "via" => self.via.clone(),
            "connect" => self.connect.clone(),
            "port" => self.port.clone(),
            "key" => self.key.clone(),
            "directory" => self.directory.clone(),
            "routes" => self.routes.join("\n"),
            _ => String::new(),
        }
    }
    pub fn set(&mut self, key: &str, value: String) {
        match key {
            "via" => self.via = value,
            "connect" => {
                if !self.connect.is_empty() && self.connect != value && !self.routes.contains(&self.connect) {
                    self.routes.insert(0, self.connect.clone());
                }
                self.connect = value;
                if !self.connect.is_empty() && !self.routes.contains(&self.connect) { self.routes.insert(0, self.connect.clone()); }
                self.routes.truncate(64);
            },
            "port" => self.port = value,
            "key" => self.key = value,
            "directory" => self.directory = value,
            "routes" => {
                self.routes = value
                    .lines()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect()
            }
            _ => {}
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        let port = if self.port.is_empty() {
            None
        } else {
            Some(
                self.port
                    .parse::<u16>()
                    .map_err(|_| "Port must be between 1 and 65535")?,
            )
        };
        let transport = |route| {
            bbt_ui_model::transport::Transport {
                kind: &self.via,
                route,
                port,
                key: Some(&self.key),
            }
            .validate()
        };
        transport(&self.connect)?;
        if self.routes.len() > 64 || self.recent_directories.len() > 6 {
            return Err("Too many saved machine choices".into());
        }
        for route in &self.routes {
            transport(route)?;
        }
        bbt_ui_model::transport::remote_directory(&self.directory)?;
        for directory in &self.recent_directories {
            bbt_ui_model::transport::remote_directory(directory)?;
        }
        Ok(())
    }
    pub fn fields(&self) -> Vec<bbt_ui_model::SpawnField> {
        use bbt_ui_model::{FieldKey, Source, SpawnField, ValueChoice};
        let choices = |values: Vec<String>| {
            Some(
                values
                    .into_iter()
                    .map(|value| ValueChoice {
                        label: value.clone(),
                        value,
                        purpose: String::new(),
                    })
                    .collect(),
            )
        };
        let mut routes = vec![self.connect.clone()];
        for route in &self.routes {
            if !routes.contains(route) {
                routes.push(route.clone());
            }
        }
        let mut dirs = vec![self.directory.clone()];
        for dir in &self.recent_directories {
            if !dirs.contains(dir) {
                dirs.push(dir.clone());
            }
        }
        [
            (
                FieldKey::Via,
                self.via.clone(),
                choices(["ssh", "mosh", "et"].map(str::to_owned).to_vec()),
            ),
            (FieldKey::Connect, self.connect.clone(), choices(routes)),
            (FieldKey::Port, self.port.clone(), None),
            (FieldKey::Key, self.key.clone(), None),
            (FieldKey::Directory, self.directory.clone(), choices(dirs)),
        ]
        .into_iter()
        .map(|(key, value, choices)| SpawnField {
            key,
            value,
            source: Source::Settings,
            choices,
            fixed: false,
        })
        .collect()
    }
}

pub fn region(id: &str, field: &str) -> String {
    format!(
        "machine/{}/{field}",
        id.replace('%', "%25").replace('/', "%2F")
    )
}
