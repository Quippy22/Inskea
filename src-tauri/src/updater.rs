//! GitHub release checks and in-place self-update helpers.
//!
//! Queries the latest GitHub release for `Quippy22/Inskea`, compares its tag
//! against the running crate version, and either installs the update in place
//! (ready for relaunch) or opens the release page in the system browser.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};

const GITHUB_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/Quippy22/Inskea/releases/latest";
const GITHUB_RELEASES_PAGE: &str = "https://github.com/Quippy22/Inskea/releases/latest";
const ALLOWED_GITHUB_PREFIX: &str = "https://github.com/Quippy22/Inskea/";

/// Path to a downloaded Windows NSIS installer waiting to run on relaunch.
static STAGED_WINDOWS_INSTALLER: Mutex<Option<PathBuf>> = Mutex::new(None);

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GithubAsset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct UpdateCheck {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub release_url: String,
    pub asset_name: Option<String>,
    pub asset_url: Option<String>,
    pub can_install: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateOutcome {
    ReadyToRelaunch,
    OpenedPage,
}

/// Strip a leading `v`/`V` prefix and surrounding whitespace from a tag name.
pub fn normalize_version(raw: &str) -> &str {
    let trimmed = raw.trim();
    trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed)
}

fn parse_semver(version: &str) -> Option<(u64, u64, u64, Option<&str>)> {
    let normalized = normalize_version(version);
    let without_build = normalized.split('+').next()?;
    let (core, prerelease) = match without_build.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (without_build, None),
    };
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch, prerelease))
}

/// Return `true` when `candidate` is strictly newer than `current`.
pub fn is_newer_version(current: &str, candidate: &str) -> bool {
    let Some((c_maj, c_min, c_pat, c_pre)) = parse_semver(current) else {
        return false;
    };
    let Some((n_maj, n_min, n_pat, n_pre)) = parse_semver(candidate) else {
        return false;
    };
    match (n_maj, n_min, n_pat).cmp(&(c_maj, c_min, c_pat)) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => match (c_pre, n_pre) {
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (Some(c), Some(n)) => n > c,
            (None, None) => false,
        },
    }
}

fn is_valid_github_url(url: &str) -> bool {
    url.starts_with(ALLOWED_GITHUB_PREFIX)
}

fn is_safe_asset_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
}

fn command_exists(program: &str) -> bool {
    let Some(path_var) = env::var_os("PATH") else {
        return false;
    };
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(program);
        if candidate.is_file() {
            return true;
        }
        #[cfg(windows)]
        if dir.join(format!("{program}.exe")).is_file() {
            return true;
        }
    }
    false
}

fn is_arch_linux() -> bool {
    Path::new("/etc/arch-release").exists() || command_exists("pacman")
}

fn is_debian_linux() -> bool {
    Path::new("/etc/debian_version").exists() || command_exists("dpkg")
}

/// Whether the running executable lives in a directory writable by the current
/// user (so its binary can be atomically replaced in place without root).
fn can_overwrite_current_exe() -> bool {
    let Ok(exe) = env::current_exe().and_then(fs::canonicalize) else {
        return false;
    };
    let Some(parent) = exe.parent() else {
        return false;
    };
    let probe = parent.join(format!(".inskea-write-probe-{}", std::process::id()));
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// Pick the release asset that matches the given platform.
pub fn select_asset_for_platform<'a>(
    assets: &'a [GithubAsset],
    os: &str,
    arch_linux: bool,
    debian_linux: bool,
) -> Option<&'a GithubAsset> {
    let valid_assets: Vec<&GithubAsset> = assets
        .iter()
        .filter(|a| is_safe_asset_name(&a.name) && is_valid_github_url(&a.browser_download_url))
        .collect();

    match os {
        "windows" => {
            let mut fallback = None;
            for asset in valid_assets {
                let lower = asset.name.to_ascii_lowercase();
                if lower.ends_with("-setup.exe") {
                    return Some(asset);
                }
                if lower.ends_with(".exe") && fallback.is_none() {
                    fallback = Some(asset);
                }
            }
            fallback
        }
        "linux" => {
            if arch_linux {
                for asset in &valid_assets {
                    let lower = asset.name.to_ascii_lowercase();
                    if lower.ends_with(".pkg.tar.zst") && !lower.contains("-debug-") {
                        return Some(asset);
                    }
                }
            }
            if debian_linux {
                for asset in &valid_assets {
                    if asset.name.to_ascii_lowercase().ends_with(".deb") {
                        return Some(asset);
                    }
                }
            }
            // Fallback to the Arch bundle if `tar` is available for in-place binary extraction.
            for asset in valid_assets {
                let lower = asset.name.to_ascii_lowercase();
                if lower.ends_with(".pkg.tar.zst") && !lower.contains("-debug-") {
                    return Some(asset);
                }
            }
            None
        }
        _ => None,
    }
}

fn can_install_asset(asset_name: &str) -> bool {
    if !is_safe_asset_name(asset_name) {
        return false;
    }
    let lower = asset_name.to_ascii_lowercase();
    if cfg!(target_os = "windows") {
        lower.ends_with(".exe")
    } else if cfg!(target_os = "linux") {
        if can_overwrite_current_exe() && command_exists("tar") {
            if lower.ends_with(".pkg.tar.zst") {
                return true;
            }
            if lower.ends_with(".deb") && command_exists("dpkg-deb") {
                return true;
            }
        }
        if !command_exists("pkexec") {
            return false;
        }
        if lower.ends_with(".pkg.tar.zst") {
            command_exists("pacman")
        } else if lower.ends_with(".deb") {
            command_exists("apt-get") || command_exists("dpkg")
        } else {
            false
        }
    } else {
        false
    }
}

/// Check whether a file path points to an npm installation directory.
pub fn is_npm_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.contains("node_modules") || s.contains(".npm")
}

/// Check if the currently running executable lives inside an npm installation
/// (e.g., node_modules or global npm directory).
pub fn is_npm_installation() -> bool {
    let Ok(exe) = env::current_exe().and_then(fs::canonicalize) else {
        return false;
    };
    is_npm_path(&exe)
}

fn try_npm_update() -> Result<UpdateOutcome, String> {
    if !command_exists("npm") {
        return Err("npm command not found in PATH".into());
    }

    #[cfg(windows)]
    {
        let status = Command::new("cmd")
            .args(["/C", "npm", "update", "-g", "inskea"])
            .status()
            .map_err(|e| format!("failed to run npm update: {e}"))?;
        if status.success() {
            return Ok(UpdateOutcome::ReadyToRelaunch);
        }
        return Err(format!("npm update exited with status {status}"));
    }

    #[cfg(target_os = "linux")]
    {
        if can_overwrite_current_exe() {
            let status = Command::new("npm")
                .args(["update", "-g", "inskea"])
                .status()
                .map_err(|e| format!("failed to run npm update: {e}"))?;
            if status.success() {
                return Ok(UpdateOutcome::ReadyToRelaunch);
            }
            log::warn!("direct npm update failed, checking pkexec: {status}");
        }

        if command_exists("pkexec") {
            let status = Command::new("pkexec")
                .args(["npm", "update", "-g", "inskea"])
                .status()
                .map_err(|e| format!("failed to run pkexec npm update: {e}"))?;
            if status.success() {
                return Ok(UpdateOutcome::ReadyToRelaunch);
            }
            return Err(format!("pkexec npm update exited with status {status}"));
        }

        Err("cannot update npm package: directory not writable and pkexec unavailable".into())
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Err("npm update is not supported on this platform".into())
    }
}

/// Fetch the latest GitHub release and compare it with the running version.
pub fn check_for_update() -> Result<UpdateCheck, String> {
    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(15)))
            .build(),
    );
    let mut resp = agent
        .get(GITHUB_LATEST_RELEASE_URL)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", concat!("Inskea/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|e| format!("request failed: {e}"))?;

    let body = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("failed to read response: {e}"))?;

    let release: GithubRelease =
        serde_json::from_str(&body).map_err(|e| format!("invalid release JSON: {e}"))?;

    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let latest_version = normalize_version(&release.tag_name).to_string();
    let update_available = is_newer_version(&current_version, &latest_version);

    let release_url = if is_valid_github_url(&release.html_url) {
        release.html_url.clone()
    } else {
        GITHUB_RELEASES_PAGE.to_string()
    };

    let selected = select_asset_for_platform(
        &release.assets,
        env::consts::OS,
        is_arch_linux(),
        is_debian_linux(),
    );

    let can_install = if is_npm_installation() {
        command_exists("npm")
    } else {
        selected
            .map(|asset| can_install_asset(&asset.name))
            .unwrap_or(false)
    };

    Ok(UpdateCheck {
        current_version,
        latest_version,
        update_available,
        release_url,
        asset_name: selected.map(|a| a.name.clone()),
        asset_url: selected.map(|a| a.browser_download_url.clone()),
        can_install,
    })
}

fn download_asset(url: &str, name: &str) -> Result<PathBuf, String> {
    if !is_valid_github_url(url) {
        return Err("refusing to download from an untrusted URL".into());
    }
    if !is_safe_asset_name(name) {
        return Err("invalid asset file name".into());
    }

    let dir = env::temp_dir().join("inskea-update");
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create temp dir: {e}"))?;
    let dest = dir.join(name);
    let tmp = dir.join(format!(".{name}.{}.part", std::process::id()));

    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(120)))
            .build(),
    );
    let mut resp = agent
        .get(url)
        .header("User-Agent", concat!("Inskea/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|e| format!("download failed: {e}"))?;

    let mut out = fs::File::create(&tmp).map_err(|e| format!("failed to create temp file: {e}"))?;
    std::io::copy(&mut resp.body_mut().as_reader(), &mut out)
        .map_err(|e| format!("failed to write update package: {e}"))?;
    drop(out);

    if dest.exists() {
        let _ = fs::remove_file(&dest);
    }
    fs::rename(&tmp, &dest).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("failed to finalize update package: {e}")
    })?;
    Ok(dest)
}

/// Extract `usr/bin/inskea` from a downloaded Linux package and atomically
/// replace the currently running executable on disk.
#[cfg(target_os = "linux")]
fn replace_current_exe_from_package(package_path: &Path, asset_name: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let current_exe = env::current_exe()
        .and_then(fs::canonicalize)
        .map_err(|e| format!("failed to resolve current executable: {e}"))?;
    let parent = current_exe
        .parent()
        .ok_or_else(|| "executable has no parent directory".to_string())?;

    let lower = asset_name.to_ascii_lowercase();
    let bytes = if lower.ends_with(".pkg.tar.zst") {
        let output = Command::new("tar")
            .args(["--zstd", "-xOf"])
            .arg(package_path)
            .arg("usr/bin/inskea")
            .output()
            .map_err(|e| format!("failed to run tar: {e}"))?;
        if !output.status.success() || output.stdout.is_empty() {
            return Err("failed to extract binary from Arch package".into());
        }
        output.stdout
    } else if lower.ends_with(".deb") {
        let tar_stream = Command::new("dpkg-deb")
            .arg("--fsys-tarfile")
            .arg(package_path)
            .output()
            .map_err(|e| format!("failed to read deb archive: {e}"))?;
        if !tar_stream.status.success() {
            return Err("dpkg-deb failed".into());
        }
        let mut child = Command::new("tar")
            .args(["-xOf", "-", "./usr/bin/inskea"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn tar: {e}"))?;
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(&tar_stream.stdout);
        }
        let output = child
            .wait_with_output()
            .map_err(|e| format!("failed to extract binary from deb: {e}"))?;
        if !output.status.success() || output.stdout.is_empty() {
            return Err("failed to extract binary from Debian package".into());
        }
        output.stdout
    } else {
        return Err("unsupported package archive".into());
    };

    let tmp_exe = parent.join(format!(".inskea-update-{}.tmp", std::process::id()));
    fs::write(&tmp_exe, &bytes).map_err(|e| format!("failed to write updated binary: {e}"))?;
    fs::set_permissions(&tmp_exe, fs::Permissions::from_mode(0o755)).map_err(|e| {
        let _ = fs::remove_file(&tmp_exe);
        format!("failed to set executable permissions: {e}")
    })?;
    fs::rename(&tmp_exe, &current_exe).map_err(|e| {
        let _ = fs::remove_file(&tmp_exe);
        format!("failed to replace executable in place: {e}")
    })
}

fn try_install_package(asset_name: &str, asset_url: &str) -> Result<UpdateOutcome, String> {
    let path = download_asset(asset_url, asset_name)?;
    let lower = asset_name.to_ascii_lowercase();

    if cfg!(target_os = "windows") && lower.ends_with(".exe") {
        if let Ok(mut staged) = STAGED_WINDOWS_INSTALLER.lock() {
            *staged = Some(path);
        }
        return Ok(UpdateOutcome::ReadyToRelaunch);
    }

    #[cfg(target_os = "linux")]
    {
        if can_overwrite_current_exe() {
            match replace_current_exe_from_package(&path, asset_name) {
                Ok(()) => {
                    let _ = fs::remove_file(&path);
                    return Ok(UpdateOutcome::ReadyToRelaunch);
                }
                Err(err) => {
                    log::warn!(
                        "in-place executable replacement failed, trying package manager: {err}"
                    );
                }
            }
        }

        let status = if lower.ends_with(".pkg.tar.zst") {
            Command::new("pkexec")
                .args(["pacman", "-U", "--noconfirm"])
                .arg(&path)
                .status()
        } else if lower.ends_with(".deb") {
            if command_exists("apt-get") {
                Command::new("pkexec")
                    .args(["apt-get", "install", "-y", "--reinstall"])
                    .arg(&path)
                    .status()
            } else {
                Command::new("pkexec")
                    .args(["dpkg", "-i"])
                    .arg(&path)
                    .status()
            }
        } else {
            let _ = fs::remove_file(&path);
            return Err("unsupported Linux package format".into());
        };

        let _ = fs::remove_file(&path);
        let status = status.map_err(|e| format!("failed to run package installer: {e}"))?;
        if status.success() {
            return Ok(UpdateOutcome::ReadyToRelaunch);
        }
        Err(format!("installer exited with status {status}"))
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = fs::remove_file(&path);
        Err("direct installation is not supported on this platform".into())
    }
}

/// Open a GitHub release page in the system browser.
pub fn open_release_url(url: &str) -> Result<(), String> {
    let target = if is_valid_github_url(url) {
        url
    } else {
        GITHUB_RELEASES_PAGE
    };

    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("rundll32");
        c.args(["url.dll,FileProtocolHandler", target]);
        c
    };

    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = Command::new("open");
        c.arg(target);
        c
    };

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let mut cmd = {
        let mut c = Command::new("xdg-open");
        c.arg(target);
        c
    };

    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("failed to open release page: {e}"))
}

/// Install the update in place if supported, otherwise open the release page
/// in the user's browser.
pub fn install_or_open(
    release_url: &str,
    asset_name: Option<&str>,
    asset_url: Option<&str>,
) -> Result<UpdateOutcome, String> {
    if is_npm_installation() {
        match try_npm_update() {
            Ok(outcome) => return Ok(outcome),
            Err(err) => {
                log::warn!("npm update failed, opening release page instead: {err}");
            }
        }
    } else if let (Some(name), Some(url)) = (asset_name, asset_url) {
        if can_install_asset(name) {
            match try_install_package(name, url) {
                Ok(outcome) => return Ok(outcome),
                Err(err) => {
                    log::warn!("direct update install failed, opening release page instead: {err}");
                }
            }
        }
    }

    open_release_url(release_url)?;
    Ok(UpdateOutcome::OpenedPage)
}

/// Restart into the newly installed version. On Windows, runs the staged NSIS
/// installer in passive update-and-relaunch mode (`/P /UPDATE /R`) and exits so
/// the existing installation is overwritten cleanly in place.
pub fn relaunch(app_handle: &tauri::AppHandle) {
    if let Ok(mut guard) = STAGED_WINDOWS_INSTALLER.lock() {
        if let Some(installer) = guard.take() {
            let _ = Command::new(&installer)
                .args(["/P", "/UPDATE", "/R"])
                .spawn();
            app_handle.exit(0);
            return;
        }
    }
    app_handle.restart();
}

#[cfg(test)]
mod tests {
    use super::{
        is_newer_version, is_npm_path, is_safe_asset_name, is_valid_github_url, normalize_version,
        select_asset_for_platform, GithubAsset,
    };
    use std::path::Path;

    #[test]
    fn detects_npm_installation_paths() {
        assert!(is_npm_path(Path::new(
            "/usr/local/lib/node_modules/@quippy/inskea-linux-x64/bin/inskea"
        )));
        assert!(is_npm_path(Path::new(
            "/home/user/.nvm/versions/node/v20.0.0/lib/node_modules/inskea/bin/inskea"
        )));
        assert!(is_npm_path(Path::new(
            "C:\\Users\\User\\AppData\\Roaming\\npm\\node_modules\\@quippy\\inskea-win32-x64\\bin\\inskea.exe"
        )));
        assert!(!is_npm_path(Path::new("/usr/bin/inskea")));
        assert!(!is_npm_path(Path::new("C:\\Program Files\\Inskea\\Inskea.exe")));
    }

    #[test]
    fn normalizes_leading_v_prefix() {
        assert_eq!(normalize_version("v2.0.0"), "2.0.0");
        assert_eq!(normalize_version("V1.7.1"), "1.7.1");
        assert_eq!(normalize_version(" 2.1.0 "), "2.1.0");
    }

    #[test]
    fn compares_semantic_versions() {
        assert!(is_newer_version("2.0.0", "v2.0.1"));
        assert!(is_newer_version("2.0.0", "2.1.0"));
        assert!(is_newer_version("1.9.9", "2.0.0"));
        assert!(is_newer_version("2.0.0-alpha.1", "2.0.0"));
        assert!(!is_newer_version("2.0.0", "2.0.0"));
        assert!(!is_newer_version("2.0.0", "v1.7.1"));
        assert!(!is_newer_version("2.0.0", "2.0.0-rc.1"));
    }

    #[test]
    fn validates_asset_names_and_urls() {
        assert!(is_safe_asset_name("inskea-2.0.0-1-x86_64.pkg.tar.zst"));
        assert!(is_safe_asset_name("Inskea_2.0.0_x64-setup.exe"));
        assert!(!is_safe_asset_name("../escape.exe"));
        assert!(!is_safe_asset_name("bad/name.deb"));
        assert!(is_valid_github_url(
            "https://github.com/Quippy22/Inskea/releases/tag/v2.0.0"
        ));
        assert!(!is_valid_github_url("https://example.com/evil.exe"));
    }

    #[test]
    fn selects_matching_platform_assets_and_skips_debug_packages() {
        let assets = vec![
            GithubAsset {
                name: "inskea-debug-2.0.0-1-x86_64.pkg.tar.zst".into(),
                browser_download_url:
                    "https://github.com/Quippy22/Inskea/releases/download/v2.0.0/inskea-debug-2.0.0-1-x86_64.pkg.tar.zst".into(),
            },
            GithubAsset {
                name: "inskea-2.0.0-1-x86_64.pkg.tar.zst".into(),
                browser_download_url:
                    "https://github.com/Quippy22/Inskea/releases/download/v2.0.0/inskea-2.0.0-1-x86_64.pkg.tar.zst".into(),
            },
            GithubAsset {
                name: "Inskea_2.0.0_amd64.deb".into(),
                browser_download_url:
                    "https://github.com/Quippy22/Inskea/releases/download/v2.0.0/Inskea_2.0.0_amd64.deb".into(),
            },
            GithubAsset {
                name: "Inskea_2.0.0_x64-setup.exe".into(),
                browser_download_url:
                    "https://github.com/Quippy22/Inskea/releases/download/v2.0.0/Inskea_2.0.0_x64-setup.exe".into(),
            },
        ];

        assert_eq!(
            select_asset_for_platform(&assets, "linux", true, false).map(|a| a.name.as_str()),
            Some("inskea-2.0.0-1-x86_64.pkg.tar.zst")
        );
        assert_eq!(
            select_asset_for_platform(&assets, "linux", false, true).map(|a| a.name.as_str()),
            Some("Inskea_2.0.0_amd64.deb")
        );
        assert_eq!(
            select_asset_for_platform(&assets, "windows", false, false).map(|a| a.name.as_str()),
            Some("Inskea_2.0.0_x64-setup.exe")
        );
        assert_eq!(
            select_asset_for_platform(&assets, "macos", false, false),
            None
        );
    }
}
