//! Package-time plugin selection.
//!
//! `SCRIPT/PLUGINS/PLUGINS.TXT` names the plugins that the installer installs,
//! separated by ASCII whitespace. The installer parses it with the same rules
//! (`runtime/selection.h`). A package without the file installs every plugin,
//! and an empty file installs none. The installer changes no firmware for a
//! plugin that is not installed. Image Control also needs effects: without
//! them, the installer treats it as not installed.

use anyhow::{bail, ensure, Result};

/// The path of the file in the SD tree.
pub const PATH: &str = "SCRIPT/PLUGINS/PLUGINS.TXT";
/// The largest file that the installer reads.
pub const MAX_FILE_SIZE: usize = 256;
/// The plugins in menu order (`plugins::PLUGINS`).
pub use crate::plugins::NAMES;

/// A set of plugins. Bit n is plugin n of `NAMES`, as the installer stores it
/// in the private header (`GRKIT_HOST_PLUGINS`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection(u32);

impl Default for Selection {
    fn default() -> Self {
        Self::ALL
    }
}

impl Selection {
    pub const ALL: Self = Self((1 << NAMES.len()) - 1);
    pub const NONE: Self = Self(0);

    /// Selects the named plugins. Unknown and repeated names are errors.
    pub fn from_names<'a>(names: impl IntoIterator<Item = &'a str>) -> Result<Self> {
        let mut mask = 0;
        for name in names {
            let Some(index) = NAMES.iter().position(|known| *known == name) else {
                bail!(
                    "Unknown plugin {name:?}. Expected one of: {}",
                    NAMES.join(", ")
                );
            };
            let bit = 1 << index;
            ensure!(mask & bit == 0, "Plugin listed twice: {name}");
            mask |= bit;
        }
        Ok(Self(mask))
    }

    /// Reads the contents of `PLUGINS.TXT`.
    pub fn parse(file: &[u8]) -> Result<Self> {
        ensure!(
            file.len() <= MAX_FILE_SIZE,
            "PLUGINS.TXT exceeds {MAX_FILE_SIZE} bytes"
        );
        let names = file
            .split(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
            .filter(|token| !token.is_empty())
            .map(|token| std::str::from_utf8(token).unwrap_or("\u{fffd}"));
        Self::from_names(names)
    }

    /// The contents of `PLUGINS.TXT`, one name for each line. Each package has
    /// the file, also with every plugin selected, so a copy of a new package
    /// over an old one always replaces the old selection.
    pub fn file(self) -> Vec<u8> {
        self.names()
            .iter()
            .map(|name| format!("{name}\n"))
            .collect::<String>()
            .into_bytes()
    }

    /// The selected plugin names, in menu order.
    pub fn names(self) -> Vec<&'static str> {
        NAMES
            .iter()
            .enumerate()
            .filter(|(index, _)| self.0 & 1 << index != 0)
            .map(|(_, name)| *name)
            .collect()
    }

    /// The selection without the named plugin. Unknown names are errors.
    pub fn without(self, name: &str) -> Result<Self> {
        let Self(bit) = Self::from_names([name])?;
        Ok(Self(self.0 & !bit))
    }

    pub fn mask(self) -> u32 {
        self.0
    }

    /// True when the camera menu would show no plugin for this many effects.
    /// `available` names the plugins that the runtime has.
    pub fn shows_nothing(self, effects: usize, available: &[&str]) -> bool {
        NAMES
            .iter()
            .enumerate()
            .filter(|(index, name)| self.0 & 1 << index != 0 && available.contains(name))
            .all(|(index, _)| crate::plugins::NEEDS_EFFECTS & 1 << index != 0 && effects == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_file_selects_everything_and_an_empty_file_nothing() {
        assert_eq!(Selection::default(), Selection::ALL);
        let all = Selection::ALL.file();
        assert_eq!(Selection::parse(&all).unwrap(), Selection::ALL);
        assert!(all.ends_with(b"\n"));
        assert_eq!(Selection::parse(b"").unwrap(), Selection::NONE);
        assert_eq!(Selection::NONE.file(), Vec::<u8>::new());
        let selection = Selection::parse(b" grain\r\nframing\t").unwrap();
        assert_eq!(selection.names(), ["framing", "grain"]);
        assert_eq!(selection.file(), b"framing\ngrain\n");
        assert_eq!(selection.mask(), 0b10010);
        assert!(Selection::parse(b"framing framing").is_err());
        assert!(Selection::parse(b"hdr").is_err());
        assert!(Selection::parse(&[b' '; MAX_FILE_SIZE + 1]).is_err());
        assert_eq!(
            Selection::ALL.without("guides").unwrap().names(),
            ["image-control", "framing", "date-stamp", "grain"]
        );
    }

    #[test]
    fn image_control_alone_shows_nothing_without_effects() {
        let image_control = Selection::from_names(["image-control"]).unwrap();
        assert!(image_control.shows_nothing(0, &NAMES));
        assert!(!image_control.shows_nothing(1, &NAMES));
        assert!(Selection::NONE.shows_nothing(3, &NAMES));
        assert!(!Selection::ALL.shows_nothing(0, &NAMES));
        let grain = Selection::from_names(["grain"]).unwrap();
        assert!(grain.shows_nothing(0, &NAMES[..4]));
    }
}
