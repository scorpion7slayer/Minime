use std::{env, fs, io, path::PathBuf};

use minime::{CompressionEffort, OutputFormat};

use crate::localization::Language;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemePreference {
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    pub const fn id(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "system" => Self::System,
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => return None,
        })
    }

    pub const fn label(self, language: Language) -> &'static str {
        match (language, self) {
            (Language::French, Self::System) => "Système",
            (Language::French, Self::Light) => "Clair",
            (Language::French, Self::Dark) => "Sombre",
            (Language::English, Self::System) => "System",
            (Language::English, Self::Light) => "Light",
            (Language::English, Self::Dark) => "Dark",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preferences {
    pub language: Language,
    pub theme: ThemePreference,
    pub output_format: OutputFormat,
    pub output_dir: Option<PathBuf>,
    pub reject_larger: bool,
    pub effort: CompressionEffort,
    pub show_preview: bool,
    pub reveal_after_compression: bool,
    pub intro_seen: bool,
    pub automatic_update_checks: Option<bool>,
    pub last_update_check_unix: Option<u64>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            language: Language::default(),
            theme: ThemePreference::System,
            output_format: OutputFormat::Auto,
            output_dir: None,
            reject_larger: true,
            effort: CompressionEffort::Balanced,
            show_preview: true,
            reveal_after_compression: false,
            intro_seen: false,
            automatic_update_checks: None,
            last_update_check_unix: None,
        }
    }
}

impl Preferences {
    pub fn load() -> Self {
        let Some(path) = preferences_path() else {
            return Self::default();
        };
        let Ok(source) = fs::read_to_string(path) else {
            return Self::default();
        };
        Self::decode(&source)
    }

    pub fn save(&self) -> io::Result<()> {
        let Some(path) = preferences_path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.encode())
    }

    fn encode(&self) -> String {
        let output_dir = self
            .output_dir
            .as_ref()
            .map(|path| path.to_string_lossy().replace(['\r', '\n'], ""))
            .unwrap_or_default();
        let automatic_update_checks = match self.automatic_update_checks {
            Some(true) => "true",
            Some(false) => "false",
            None => "ask",
        };
        let last_update_check_unix = self
            .last_update_check_unix
            .map(|timestamp| timestamp.to_string())
            .unwrap_or_default();
        format!(
            "version=3\nlanguage={}\ntheme={}\noutput_format={}\noutput_dir={}\nreject_larger={}\neffort={}\nshow_preview={}\nreveal_after_compression={}\nintro_seen={}\nautomatic_update_checks={}\nlast_update_check_unix={}\n",
            self.language.id(),
            self.theme.id(),
            self.output_format.id(),
            output_dir,
            self.reject_larger,
            self.effort.id(),
            self.show_preview,
            self.reveal_after_compression,
            self.intro_seen,
            automatic_update_checks,
            last_update_check_unix,
        )
    }

    fn decode(source: &str) -> Self {
        let mut preferences = Self::default();
        for line in source.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "language" => {
                    if let Some(language) = Language::from_id(value) {
                        preferences.language = language;
                    }
                }
                "theme" => {
                    if let Some(theme) = ThemePreference::from_id(value) {
                        preferences.theme = theme;
                    }
                }
                "output_format" => {
                    if let Some(format) = OutputFormat::from_id(value) {
                        preferences.output_format = format;
                    }
                }
                "output_dir" if !value.is_empty() => {
                    preferences.output_dir = Some(PathBuf::from(value));
                }
                "reject_larger" => preferences.reject_larger = value == "true",
                "effort" => {
                    if let Some(effort) = CompressionEffort::from_id(value) {
                        preferences.effort = effort;
                    }
                }
                "show_preview" => preferences.show_preview = value == "true",
                "reveal_after_compression" => {
                    preferences.reveal_after_compression = value == "true";
                }
                "intro_seen" => preferences.intro_seen = value == "true",
                "automatic_update_checks" => {
                    preferences.automatic_update_checks = match value {
                        "true" => Some(true),
                        "false" => Some(false),
                        _ => None,
                    };
                }
                "last_update_check_unix" => {
                    preferences.last_update_check_unix = value.parse().ok();
                }
                _ => {}
            }
        }
        if preferences
            .output_dir
            .as_ref()
            .is_some_and(|path| !path.is_dir())
        {
            preferences.output_dir = None;
        }
        preferences
    }
}

fn preferences_path() -> Option<PathBuf> {
    if let Some(directory) = env::var_os("MINIME_CONFIG_DIR") {
        return Some(PathBuf::from(directory).join("preferences.conf"));
    }

    #[cfg(target_os = "macos")]
    {
        env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join("Library/Application Support/Minime")
                .join("preferences.conf")
        })
    }
    #[cfg(target_os = "windows")]
    {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("Minime/preferences.conf"))
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .map(|path| path.join("minime/preferences.conf"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_round_trip() {
        let preferences = Preferences {
            language: Language::English,
            theme: ThemePreference::Dark,
            output_format: OutputFormat::Tiff,
            output_dir: None,
            reject_larger: false,
            effort: CompressionEffort::Maximum,
            show_preview: false,
            reveal_after_compression: true,
            intro_seen: true,
            automatic_update_checks: Some(true),
            last_update_check_unix: Some(1_725_000_000),
        };

        assert_eq!(Preferences::decode(&preferences.encode()), preferences);
    }

    #[test]
    fn invalid_values_fall_back_to_safe_defaults() {
        let preferences = Preferences::decode(
            "language=de\noutput_format=jpeg\neffort=slow\nreject_larger=true\n",
        );

        assert_eq!(preferences.language, Language::English);
        assert_eq!(preferences.output_format, OutputFormat::Auto);
        assert_eq!(preferences.effort, CompressionEffort::Balanced);
        assert_eq!(preferences.theme, ThemePreference::System);
        assert!(preferences.reject_larger);
        assert_eq!(preferences.automatic_update_checks, None);
    }

    #[test]
    fn legacy_preferences_default_to_system_theme() {
        let preferences = Preferences::decode("version=1\nlanguage=en\nintro_seen=true\n");

        assert_eq!(preferences.language, Language::English);
        assert_eq!(preferences.theme, ThemePreference::System);
        assert!(preferences.intro_seen);
        assert_eq!(preferences.automatic_update_checks, None);
    }

    #[test]
    fn fresh_preferences_start_in_english() {
        assert_eq!(Preferences::default().language, Language::English);
        assert_eq!(
            Preferences::decode("version=2\n").language,
            Language::English
        );
    }

    #[test]
    fn update_consent_is_preserved_and_invalid_timestamps_are_ignored() {
        let enabled = Preferences::decode(
            "version=3\nautomatic_update_checks=true\nlast_update_check_unix=1725000000\n",
        );
        assert_eq!(enabled.automatic_update_checks, Some(true));
        assert_eq!(enabled.last_update_check_unix, Some(1_725_000_000));

        let manual = Preferences::decode(
            "version=3\nautomatic_update_checks=false\nlast_update_check_unix=not-a-number\n",
        );
        assert_eq!(manual.automatic_update_checks, Some(false));
        assert_eq!(manual.last_update_check_unix, None);
    }
}
