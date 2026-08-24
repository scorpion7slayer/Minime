use std::{env, path::PathBuf, process::Command, time::Duration};

#[cfg(unix)]
use std::fs;
#[cfg(any(unix, test))]
use std::path::Path;

use anyhow::{Context as _, Result, anyhow};
use axoupdater::{AxoUpdater, AxoupdateError, ReleaseSource, ReleaseSourceType, Version};

pub const AUTOMATIC_CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;

const APP_NAME: &str = "minime";
const REPOSITORY_OWNER: &str = "scorpion7slayer";
const REPOSITORY_NAME: &str = "Minime";
const OFFICIAL_APP_ID: &str = "io.github.scorpion7slayer.minime";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCheck {
    DevelopmentBuild,
    UpToDate,
    FeedUnavailable,
    Available { version: String },
    ManagedExternally { version: String },
}

#[derive(Debug, Clone)]
pub enum InstallOutcome {
    UpToDate,
    Updated {
        version: String,
        restart_target: RestartTarget,
    },
}

#[derive(Debug, Clone)]
pub struct RestartTarget {
    #[cfg(not(target_os = "macos"))]
    executable: PathBuf,
    #[cfg(target_os = "macos")]
    application_bundle: PathBuf,
}

pub fn should_check_automatically(last_check_unix: Option<u64>, now_unix: u64) -> bool {
    last_check_unix.is_none_or(|last_check| {
        now_unix.saturating_sub(last_check) >= AUTOMATIC_CHECK_INTERVAL_SECS
    })
}

pub fn check_for_update() -> Result<UpdateCheck> {
    if !is_official_build() {
        return Ok(UpdateCheck::DevelopmentBuild);
    }

    let current_version = current_version()?;
    let mut updater = configured_updater()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Unable to start the update client")?;
    let latest_version = match runtime.block_on(async {
        updater
            .query_new_version()
            .await
            .map(|version| version.cloned())
    }) {
        Ok(Some(version)) => version,
        Ok(None) => return Ok(UpdateCheck::UpToDate),
        Err(AxoupdateError::NoStableReleases { .. }) => {
            return Ok(UpdateCheck::FeedUnavailable);
        }
        Err(error) => {
            return Err(error).context("Unable to read the latest Minime release");
        }
    };

    if latest_version > current_version {
        let version = latest_version.to_string();
        if is_flatpak() {
            Ok(UpdateCheck::ManagedExternally { version })
        } else {
            Ok(UpdateCheck::Available { version })
        }
    } else {
        Ok(UpdateCheck::UpToDate)
    }
}

pub fn install_update() -> Result<InstallOutcome> {
    if !is_official_build() {
        return Err(anyhow!(
            "Updates can only be installed from an official Minime build"
        ));
    }

    if is_flatpak() {
        return Err(anyhow!(
            "This copy is managed by Flatpak. Install the update through Minime's Flatpak source."
        ));
    }

    let restart_target = restart_target()?;
    let mut updater = configured_updater()?;
    let result = updater
        .run_sync()
        .context("The Minime update installer failed")?;

    let Some(result) = result else {
        return Ok(InstallOutcome::UpToDate);
    };

    Ok(InstallOutcome::Updated {
        version: result.new_version.to_string(),
        restart_target,
    })
}

pub fn restart_application(target: &RestartTarget) -> Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg("-n").arg(&target.application_bundle);
        command
    };

    #[cfg(not(target_os = "macos"))]
    let mut command = Command::new(&target.executable);

    command
        .spawn()
        .context("Unable to restart the updated Minime application")?;
    Ok(())
}

pub fn cleanup_update_backup() {
    if !is_official_build() {
        return;
    }

    if let Err(error) = cleanup_update_backup_inner() {
        log::warn!("Unable to remove the previous Minime update backup: {error}");
    }
}

fn configured_updater() -> Result<AxoUpdater> {
    let install_dir = install_directory()?;
    let install_dir = install_dir
        .to_str()
        .context("The Minime install path is not valid UTF-8")?;
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(30))
        .user_agent(format!("Minime/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .context("Unable to configure the update client")?;
    let mut updater = AxoUpdater::new_for(APP_NAME);
    updater
        .set_release_source(ReleaseSource {
            release_type: ReleaseSourceType::GitHub,
            owner: REPOSITORY_OWNER.into(),
            name: REPOSITORY_NAME.into(),
            app_name: APP_NAME.into(),
        })
        .set_client(client)
        .set_install_dir(install_dir)
        .disable_installer_output();
    updater
        .set_current_version(current_version()?)
        .context("Unable to configure the installed Minime version")?;
    Ok(updater)
}

fn current_version() -> Result<Version> {
    env!("CARGO_PKG_VERSION")
        .parse::<Version>()
        .context("The installed Minime version is invalid")
}

fn is_official_build() -> bool {
    option_env!("MINIME_APP_ID") == Some(OFFICIAL_APP_ID)
}

fn is_flatpak() -> bool {
    env::var_os("FLATPAK_ID").is_some()
}

fn install_directory() -> Result<PathBuf> {
    env::current_exe()?
        .parent()
        .map(|path| path.to_path_buf())
        .context("Unable to locate the Minime install directory")
}

fn restart_target() -> Result<RestartTarget> {
    let executable = env::current_exe().context("Unable to locate the Minime executable")?;

    #[cfg(target_os = "macos")]
    {
        let application_bundle = application_bundle_from_executable(&executable)?;
        if executable
            .components()
            .any(|component| component.as_os_str() == "AppTranslocation")
        {
            return Err(anyhow!(
                "Move Minime to Applications before installing an update"
            ));
        }
        Ok(RestartTarget { application_bundle })
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(RestartTarget { executable })
    }
}

#[cfg(target_os = "macos")]
fn application_bundle_from_executable(executable: &Path) -> Result<PathBuf> {
    let macos_directory = executable
        .parent()
        .filter(|path| path.file_name().is_some_and(|name| name == "MacOS"))
        .context("Minime is not running from a macOS application bundle")?;
    let contents_directory = macos_directory
        .parent()
        .filter(|path| path.file_name().is_some_and(|name| name == "Contents"))
        .context("Minime is not running from a macOS application bundle")?;
    let bundle = contents_directory
        .parent()
        .filter(|path| path.extension().is_some_and(|extension| extension == "app"))
        .context("Minime is not running from a macOS application bundle")?;
    Ok(bundle.to_path_buf())
}

#[cfg(any(unix, test))]
fn backup_path(path: &Path) -> Result<PathBuf> {
    let file_name = path
        .file_name()
        .context("Unable to identify the Minime update backup")?;
    let mut backup_name = file_name.to_os_string();
    backup_name.push(".minime-backup");
    Ok(path.with_file_name(backup_name))
}

fn cleanup_update_backup_inner() -> Result<()> {
    #[cfg(any(unix, windows))]
    let executable = env::current_exe().context("Unable to locate the Minime executable")?;

    #[cfg(target_os = "macos")]
    {
        let bundle = application_bundle_from_executable(&executable)?;
        let backup = backup_path(&bundle)?;
        if backup.is_dir() {
            fs::remove_dir_all(backup)?;
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let backup = backup_path(&executable)?;
        if backup.is_file() {
            fs::remove_file(backup)?;
        }
    }

    #[cfg(windows)]
    {
        let mut backup_name = executable.as_os_str().to_os_string();
        backup_name.push(".previous.exe");
        let backup = PathBuf::from(backup_name);
        if backup.is_file() {
            std::fs::remove_file(backup)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_checks_are_limited_to_once_per_day() {
        assert!(should_check_automatically(None, 50));
        assert!(!should_check_automatically(Some(50), 51));
        assert!(!should_check_automatically(
            Some(50),
            50 + AUTOMATIC_CHECK_INTERVAL_SECS - 1
        ));
        assert!(should_check_automatically(
            Some(50),
            50 + AUTOMATIC_CHECK_INTERVAL_SECS
        ));
    }

    #[test]
    fn backup_paths_stay_next_to_the_installed_item() {
        assert_eq!(
            backup_path(Path::new("/Applications/Minime.app")).unwrap(),
            PathBuf::from("/Applications/Minime.app.minime-backup")
        );
        assert_eq!(
            backup_path(Path::new("/opt/minime")).unwrap(),
            PathBuf::from("/opt/minime.minime-backup")
        );
    }
}
