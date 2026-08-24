use minime::{CompressionEffort, OutputFormat, format_bytes};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Language {
    French,
    #[default]
    English,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::English, Self::French];

    pub const fn id(self) -> &'static str {
        match self {
            Self::French => "fr",
            Self::English => "en",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "fr" => Self::French,
            "en" => Self::English,
            _ => return None,
        })
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::French => "FR",
            Self::English => "EN",
        }
    }

    pub const fn text<'a>(self, french: &'a str, english: &'a str) -> &'a str {
        match self {
            Self::French => french,
            Self::English => english,
        }
    }

    pub fn format_bytes(self, bytes: u64) -> String {
        if self == Self::French {
            return format_bytes(bytes);
        }

        const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
        let mut value = bytes as f64;
        let mut unit = 0;
        while value >= 1024.0 && unit < UNITS.len() - 1 {
            value /= 1024.0;
            unit += 1;
        }
        if unit == 0 {
            format!("{bytes} {}", UNITS[unit])
        } else if value >= 10.0 {
            format!("{value:.0} {}", UNITS[unit])
        } else {
            format!("{value:.1} {}", UNITS[unit])
        }
    }

    pub const fn format_description(self, format: OutputFormat) -> &'static str {
        match self {
            Self::French => format.description(),
            Self::English => format.description_en(),
        }
    }

    pub const fn effort_label(self, effort: CompressionEffort) -> &'static str {
        match (self, effort) {
            (Self::French, CompressionEffort::Fast) => "Rapide",
            (Self::French, CompressionEffort::Balanced) => "Équilibré",
            (Self::French, CompressionEffort::Maximum) => "Maximum",
            (Self::English, CompressionEffort::Fast) => "Fast",
            (Self::English, CompressionEffort::Balanced) => "Balanced",
            (Self::English, CompressionEffort::Maximum) => "Maximum",
        }
    }

    pub fn engine_error(self, message: &str) -> String {
        if self == Self::French {
            return message.to_string();
        }
        if message.contains("profondeur de couleur") {
            "This format would lose some color information, so Minime left the image alone.".into()
        } else if message.contains("profil colorimétrique") {
            "This format can’t keep the image’s color profile, so no copy was made.".into()
        } else if message.contains("images animées") {
            "Animated images aren’t flattened. The original is untouched.".into()
        } else if message.contains("512 Mio") {
            "This image is over Minime’s 512 MiB safety limit.".into()
        } else if message.contains("ne peut pas ouvrir ce format") {
            "Minime can’t open this format, or the file has moved.".into()
        } else if message.contains("vérification pixel par pixel") {
            "The pixels didn’t match after writing, so Minime discarded the new file.".into()
        } else {
            "Minime couldn’t finish this image. The original is untouched.".into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_bytes_in_both_languages() {
        assert_eq!(Language::French.format_bytes(1_536), "1.5 Kio");
        assert_eq!(Language::English.format_bytes(1_536), "1.5 KiB");
    }
}
