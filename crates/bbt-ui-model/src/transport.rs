// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Pure configured transport argv. No network, process, credential or filesystem IO.

pub struct Transport<'a> {
    pub kind: &'a str,
    pub route: &'a str,
    /// SSH bootstrap port, including for mosh and Eternal Terminal.
    pub port: Option<u16>,
    pub key: Option<&'a str>,
}

pub fn quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_@%+=:,./-".contains(&c))
    {
        value.into()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}
pub fn join(argv: &[String]) -> String {
    argv.iter().map(|s| quote(s)).collect::<Vec<_>>().join(" ")
}
pub fn remote_directory(value: &str) -> Result<String, String> {
    if value.contains('\0') || value.contains(['\r', '\n']) {
        return Err("Remote Directory contains a control character".into());
    }
    if value == "~" {
        Ok("\"$HOME\"".into())
    } else if let Some(rest) = value.strip_prefix("~/") {
        Ok(format!("\"$HOME\"/{}", quote(rest)))
    } else if value.starts_with('/') {
        Ok(quote(value))
    } else {
        Err("Remote Directory must be absolute or start with ~/".into())
    }
}
pub fn remote_command(
    cwd: &str,
    argv: &[String],
    env: &[(String, String)],
) -> Result<String, String> {
    if argv.first().is_none_or(String::is_empty) || argv.iter().any(|s| s.contains('\0')) {
        return Err("Command must name an executable without NUL".into());
    }
    let mut command = vec![];
    if !env.is_empty() {
        command.push("env".into());
        for (key, value) in env {
            if key.is_empty()
                || key.starts_with("BBOXY_")
                || key.starts_with(|c: char| c.is_ascii_digit())
                || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                || value.contains('\0')
            {
                return Err("Environment uses KEY=value".into());
            }
            command.push(format!("{key}={value}"));
        }
    }
    command.extend_from_slice(argv);
    Ok(format!(
        "cd {} && exec {}",
        remote_directory(cwd)?,
        join(&command)
    ))
}
impl Transport<'_> {
    pub fn validate(&self) -> Result<(), String> {
        if !["ssh", "mosh", "et"].contains(&self.kind) {
            return Err("Via must be ssh, mosh or et".into());
        }
        if self.route.is_empty()
            || self.route.starts_with('-')
            || self.route.len() > 512
            || !self
                .route
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_.-@:[]%".contains(&c))
        {
            return Err("Connect must name a host or user@host route".into());
        }
        if self.port == Some(0) {
            return Err("Port must be between 1 and 65535".into());
        }
        if self
            .key
            .is_some_and(|s| s.len() > 4096 || s.chars().any(char::is_control))
        {
            return Err("Key must name an identity file".into());
        }
        Ok(())
    }
    pub fn wrap(&self, inner: &str) -> Result<Vec<String>, String> {
        self.validate()?;
        if inner.contains('\0') {
            return Err("Remote command contains NUL".into());
        }
        let mut ssh = vec!["ssh".into()];
        if let Some(port) = self.port {
            ssh.extend(["-p".into(), port.to_string()]);
        }
        if let Some(key) = self.key.filter(|s| !s.is_empty()) {
            ssh.extend(["-i".into(), key.into()]);
        }
        match self.kind {
            "ssh" => {
                ssh.insert(1, "-t".into());
                ssh.extend(["--".into(), self.route.into(), inner.into()]);
                Ok(ssh)
            }
            // mosh's -p selects UDP; --ssh controls its SSH bootstrap instead.
            // CLI: https://github.com/mobile-shell/mosh/blob/master/scripts/mosh.pl
            "mosh" => Ok(vec![
                "mosh".into(),
                format!("--ssh={}", join(&ssh)),
                "--".into(),
                self.route.into(),
                "sh".into(),
                "-lc".into(),
                inner.into(),
            ]),
            // Long flags avoid ET 7's reassignment of short -c/-p flags.
            // CLI: https://github.com/MisterTea/EternalTerminal/blob/master/src/terminal/TerminalClientMain.cpp
            "et" => {
                let mut out = vec!["et".into()];
                if let Some(port) = self.port {
                    out.push(format!("--ssh-option=Port={port}"));
                }
                if let Some(key) = self.key.filter(|s| !s.is_empty()) {
                    out.push(format!("--ssh-option=IdentityFile={}", quote(key)));
                }
                out.extend([format!("--command={inner}"), "--".into(), self.route.into()]);
                Ok(out)
            }
            _ => unreachable!("validated transport"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transport_table_preserves_configured_argv_without_network() {
        let inner = remote_command(
            "~/src space",
            &["codex".into(), "$(literal)".into(), "".into()],
            &[("A".into(), "two words".into())],
        )
        .unwrap();
        assert_eq!(
            inner,
            "cd \"$HOME\"/'src space' && exec env 'A=two words' codex '$(literal)' ''"
        );
        for (kind, expected) in [
            (
                "ssh",
                vec![
                    "ssh",
                    "-t",
                    "-p",
                    "2222",
                    "-i",
                    "/keys/a key",
                    "--",
                    "user@host",
                    &inner,
                ],
            ),
            (
                "mosh",
                vec![
                    "mosh",
                    "--ssh=ssh -p 2222 -i '/keys/a key'",
                    "--",
                    "user@host",
                    "sh",
                    "-lc",
                    &inner,
                ],
            ),
            (
                "et",
                vec![
                    "et",
                    "--ssh-option=Port=2222",
                    "--ssh-option=IdentityFile='/keys/a key'",
                    "COMMAND",
                    "--",
                    "user@host",
                ],
            ),
        ] {
            let mut expected: Vec<String> = expected.into_iter().map(str::to_owned).collect();
            if kind == "et" {
                expected[3] = format!("--command={inner}");
            }
            assert_eq!(
                Transport {
                    kind,
                    route: "user@host",
                    port: Some(2222),
                    key: Some("/keys/a key")
                }
                .wrap(&inner)
                .unwrap(),
                expected
            );
        }
        assert_eq!(remote_directory("~").unwrap(), "\"$HOME\"");
        assert!(remote_directory("relative").is_err());
        for route in ["", "-oProxyCommand=bad", "a b", "a\n"] {
            assert!(
                Transport {
                    kind: "ssh",
                    route,
                    port: None,
                    key: None
                }
                .wrap("true")
                .is_err()
            );
        }
    }
}
