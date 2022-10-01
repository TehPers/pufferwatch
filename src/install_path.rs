use serde::Deserialize;
use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};
use tracing::{instrument, trace};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TargetsFile {
    pub property_group: PropertyGroup,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PropertyGroup {
    pub game_path: PathBuf,
}

/// Gets the possible installation paths for Stardew Valley.
#[instrument(level = "trace")]
pub fn get_install_paths() -> impl IntoIterator<Item = PathBuf> {
    let home = dirs::home_dir();
    let custom_paths = home.as_ref().and_then(|home| get_custom_install_path(home));
    let default_paths = get_default_install_paths(home.as_ref().map(AsRef::as_ref));
    custom_paths
        .into_iter()
        .chain(default_paths)
        .filter_map(|path| path.canonicalize().ok())
        .inspect(|path| trace!(?path, "possible SDV path"))
        .filter(|path| path.join("Stardew Valley.dll").is_file())
        .inspect(|path| trace!(?path, "looks like SDV path"))
}

fn get_custom_install_path(home: &Path) -> Option<PathBuf> {
    let targets_file = home.join("stardewvalley.targets");
    let targets_file = File::open(targets_file).ok()?;
    let targets: TargetsFile = quick_xml::de::from_reader(BufReader::new(targets_file)).ok()?;
    Some(targets.property_group.game_path)
}

#[allow(clippy::used_underscore_binding)]
fn get_default_install_paths(_home: Option<&Path>) -> impl IntoIterator<Item = PathBuf> + 'static {
    #[cfg(unix)]
    fn unix_paths(home: Option<&Path>) -> impl IntoIterator<Item = PathBuf> + 'static {
        home.map(|dir| {
            [
                dir.join("GOG Games/Stardew Valley/game"),
                dir.join(".steam/steam/steamapps/common/Stardew Valley"),
                dir.join(".local/share/Steam/steamapps/common/Stardew Valley"),
            ]
        })
        .into_iter()
        .flatten()
    }

    #[cfg(target_os = "macos")]
    fn mac_paths(home: Option<&Path>) -> impl IntoIterator<Item = PathBuf> + 'static {
        std::iter::once(PathBuf::from(
            "/Applications/Stardew Valley.app/Contents/MacOS",
        ))
        .chain(home.map(|dir| {
            dir.join(
                "/Library/Application Support/Steam/steamapps/common/Stardew Valley/Contents/MacOS",
            )
        }))
    }

    #[cfg(windows)]
    fn windows_paths() -> impl IntoIterator<Item = PathBuf> {
        use std::ffi::OsString;
        use winreg::{
            enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
            RegKey,
        };

        // Get relevant registry values
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let paths = hklm
            .open_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Steam App 413150")
            .and_then(|key| key.get_value("InstallLocation"))
            .map(OsString::into)
            .into_iter();
        let paths = paths.chain(
            hklm.open_subkey(r"SOFTWARE\WOW6432Node\GOG.com\Games\1453375253")
                .and_then(|key| key.get_value("PATH"))
                .map(OsString::into),
        );
        let paths = paths.chain(
            hkcu.open_subkey(r"SOFTWARE\Valve\Steam")
                .and_then(|key| key.get_value("SteamPath"))
                .map(|path: OsString| Path::new(&path).join(r"steamapps\common\Stardew Valley")),
        );

        // Default GOG/Steam paths
        let paths = paths.chain(
            [
                Path::new(r"C:\Program Files"),
                Path::new(r"C:\Program Files (x86)"),
            ]
            .into_iter()
            .flat_map(|program_files| {
                [
                    program_files.join(r"GalaxyClient\Games\Stardew Valley"),
                    program_files.join(r"GOG Galaxy\Games\Stardew Valley"),
                    program_files.join(r"GOG Games\Stardew Valley"),
                    program_files.join(r"Steam\steamapps\common\Stardew Valley"),
                ]
            }),
        );

        // Xbox paths
        paths.chain(('C'..='H').into_iter().map(|drive| {
            PathBuf::from(format!(
                r"{drive}:\Program Files\ModifiableWindowsApps\Stardew Valley"
            ))
        }))
    }

    // Collect paths
    let paths = std::iter::empty();
    #[cfg(unix)]
    let paths = paths.chain(unix_paths(_home));
    #[cfg(target_os = "macos")]
    let paths = paths.chain(mac_paths(_home));
    #[cfg(windows)]
    let paths = paths.chain(windows_paths());

    paths
}
