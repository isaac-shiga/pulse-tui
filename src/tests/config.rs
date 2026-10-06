use std::path::PathBuf;

use crate::config::config_dir;

#[test]
fn windows_without_home_keeps_settings_in_appdata() {
    let appdata = PathBuf::from(r"C:\Users\ada\AppData\Roaming");

    assert_eq!(config_dir(None, None, Some(appdata.clone())), appdata);
}

#[test]
fn xdg_config_home_wins_over_home() {
    let dir = config_dir(
        Some(PathBuf::from("/xdg")),
        Some(PathBuf::from("/home/ada")),
        None,
    );

    assert_eq!(dir, PathBuf::from("/xdg"));
}
