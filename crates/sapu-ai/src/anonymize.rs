//! Strips personal segments from strings before they leave the device.

use std::path::Path;

#[derive(Debug, Clone)]
pub struct Anonymizer {
    home: Option<String>,
    user: Option<String>,
    enabled: bool,
}

impl Anonymizer {
    pub fn new(enabled: bool) -> Self {
        let home = dirs::home_dir().map(|h| h.to_string_lossy().into_owned());
        let user = home
            .as_deref()
            .and_then(|h| Path::new(h).file_name())
            .map(|n| n.to_string_lossy().into_owned());
        Self::with(home, user, enabled)
    }

    pub fn with(home: Option<String>, user: Option<String>, enabled: bool) -> Self {
        Self {
            home,
            user,
            enabled,
        }
    }

    pub fn text(&self, s: &str) -> String {
        if !self.enabled {
            return s.to_string();
        }
        let mut out = s.to_string();
        if let Some(h) = &self.home {
            out = out.replace(h.as_str(), "~");
            out = out.replace(&h.replace('/', "\\"), "~");
        }
        if let Some(u) = &self.user
            && u.len() >= 3
        {
            out = out.replace(u.as_str(), "<user>");
        }
        out
    }

    pub fn path(&self, p: &Path) -> String {
        self.text(&p.to_string_lossy())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_home_and_user() {
        let a = Anonymizer::with(Some("/Users/budi".into()), Some("budi".into()), true);
        assert_eq!(
            a.path(Path::new("/Users/budi/Downloads/x")),
            "~/Downloads/x"
        );
        assert_eq!(a.text("backup-budi-2024"), "backup-<user>-2024");
        let off = Anonymizer::with(Some("/Users/budi".into()), Some("budi".into()), false);
        assert_eq!(off.path(Path::new("/Users/budi/x")), "/Users/budi/x");
    }
}
