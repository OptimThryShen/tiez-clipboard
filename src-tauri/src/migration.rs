use std::path::PathBuf;

/// v0.2.8 Rename Migration: 贴汁 -> TieZ
pub fn perform_migration_v028(default_app_dir: &PathBuf) {
    let mut old_app_dirs_to_check = Vec::new();

    if let Some(parent) = default_app_dir.parent() {
        old_app_dirs_to_check.push(parent.join("贴汁"));
        #[cfg(target_os = "macos")]
        {
        old_app_dirs_to_check.push(parent.join("tie-z"));
        old_app_dirs_to_check.push(parent.join("com.tiez.app"));
        }
    }

    #[cfg(target_os = "windows")]
    for variable in ["LOCALAPPDATA", "APPDATA"] {
        if let Ok(base) = std::env::var(variable) {
            let candidate = PathBuf::from(base).join("贴汁");
            if !old_app_dirs_to_check.contains(&candidate) {
                old_app_dirs_to_check.push(candidate);
            }
        }
    }

    // Try each possible location
    for old_app_dir in old_app_dirs_to_check {
        if old_app_dir.exists() && old_app_dir.is_dir() {
            println!(
                ">>> [MIGRATION] Found old data folder at: {:?}",
                old_app_dir
            );
            let new_db = default_app_dir.join("clipboard.db");
            let old_db = old_app_dir.join("clipboard.db");

            let mut success = false;

            // Older Windows builds supported a custom data path through datapath.txt.
            // Migrate the redirect first so the subsequent startup opens the same database.
            let old_redirect = old_app_dir.join("datapath.txt");
            if old_redirect.exists() {
                let _ = std::fs::create_dir_all(default_app_dir);
                if std::fs::copy(&old_redirect, default_app_dir.join("datapath.txt")).is_ok() {
                    success = true;
                }
            }

            // 1. Data Migration Logic
            if !default_app_dir.exists() {
                println!(
                    ">>> [MIGRATION] Renaming old data folder {:?} to 'TieZ'...",
                    old_app_dir
                );
                success = std::fs::rename(&old_app_dir, &default_app_dir).is_ok();
            } else if old_db.exists() && !new_db.exists() {
                println!(
                    ">>> [MIGRATION] Pulling old data from {:?} to 'TieZ'...",
                    old_app_dir
                );
                let _ = std::fs::create_dir_all(&default_app_dir);
                if std::fs::copy(&old_db, &new_db).is_ok() {
                    success = true;
                    let old_log = old_app_dir.join("tiez.log");
                    if old_log.exists() {
                        let _ = std::fs::copy(&old_log, default_app_dir.join("tiez.log"));
                    }
                }
            } else if old_db.exists() && new_db.exists() {
                let old_size = std::fs::metadata(&old_db).map(|m| m.len()).unwrap_or(0);
                let new_size = std::fs::metadata(&new_db).map(|m| m.len()).unwrap_or(0);

                if old_size > new_size && new_size < 50_000 {
                    println!(">>> [MIGRATION] Old database ({} bytes) has more data than new ({} bytes). Replacing...", old_size, new_size);
                    let backup_db = default_app_dir.join("clipboard.db.backup");
                    let _ = std::fs::rename(&new_db, &backup_db);

                    if std::fs::copy(&old_db, &new_db).is_ok() {
                        success = true;
                        println!(">>> [MIGRATION] Successfully migrated old database to TieZ.");
                    } else {
                        let _ = std::fs::rename(&backup_db, &new_db);
                    }
                } else {
                    success = true;
                }
            } else {
                success = true;
            }

            if success {
                println!(">>> [CLEANUP] Cleaning up residues of old version...");
                if old_app_dir.exists() {
                    let _ = std::fs::remove_dir_all(&old_app_dir);
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let custom_path = cleanup_old_install_registry();
        cleanup_old_start_menu();
        cleanup_old_install_folder(custom_path);
    }
}

pub fn cleanup_old_install_registry() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
        use winreg::RegKey;

        let uninstall_path = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu
            .open_subkey_with_flags(uninstall_path, KEY_READ | KEY_WRITE)
            .ok()?;
        let mut install_location = None;

        for subkey_name in key.enum_keys().filter_map(Result::ok) {
            let Ok(subkey) = key.open_subkey(&subkey_name) else {
                continue;
            };
            let display_name: String = subkey.get_value("DisplayName").unwrap_or_default();
            if !display_name.contains("贴汁") {
                continue;
            }

            install_location = subkey
                .get_value::<String, _>("InstallLocation")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .map(PathBuf::from)
                .or_else(|| {
                    subkey
                        .get_value::<String, _>("UninstallString")
                        .ok()
                        .and_then(|value| {
                            let executable = value.trim().trim_matches('"');
                            std::path::Path::new(executable)
                                .parent()
                                .map(std::path::Path::to_path_buf)
                        })
                });
            let _ = key.delete_subkey_all(&subkey_name);
        }
        install_location
    }

    #[cfg(not(target_os = "windows"))]
    None
}

pub fn cleanup_old_start_menu() {
    #[cfg(target_os = "windows")]
    {
        if let Ok(app_data) = std::env::var("APPDATA") {
            let programs = PathBuf::from(app_data)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs");
            let shortcut = programs.join("贴汁.lnk");
            if shortcut.is_file() {
                let _ = std::fs::remove_file(shortcut);
            }
            let folder = programs.join("贴汁");
            if folder.is_dir() {
                let _ = std::fs::remove_dir_all(folder);
            }
        }

        if let Ok(profile) = std::env::var("USERPROFILE") {
            let shortcut = PathBuf::from(profile).join("Desktop").join("贴汁.lnk");
            if shortcut.is_file() {
                let _ = std::fs::remove_file(shortcut);
            }
        }
    }
}

pub fn enable_autostart_manually(app_name: &str, exe_path: &str) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) =
            hkcu.create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")?;
        return key.set_value(app_name, &exe_path);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app_name, exe_path);
        Ok(())
    }
}

pub fn cleanup_old_install_folder(custom_path: Option<PathBuf>) {
    #[cfg(target_os = "windows")]
    {
        let mut candidates = vec![
            std::env::var("LOCALAPPDATA")
                .ok()
                .map(|base| PathBuf::from(base).join("Programs").join("贴汁")),
            std::env::var("ProgramFiles")
                .ok()
                .map(|base| PathBuf::from(base).join("贴汁")),
            std::env::var("ProgramFiles(x86)")
                .ok()
                .map(|base| PathBuf::from(base).join("贴汁")),
        ];
        candidates.push(custom_path);

        let current_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(std::path::Path::to_path_buf));
        for candidate in candidates.into_iter().flatten() {
            if !candidate.is_dir() || current_dir.as_ref() == Some(&candidate) {
                continue;
            }

            // Registry values can be malformed. Only delete a directory whose final
            // component is the exact legacy product name.
            if candidate.file_name().and_then(|name| name.to_str()) != Some("贴汁") {
                continue;
            }
            let _ = std::fs::remove_dir_all(candidate);
        }
    }

    #[cfg(not(target_os = "windows"))]
    let _ = custom_path;
}
