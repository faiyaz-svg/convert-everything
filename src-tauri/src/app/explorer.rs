//! The "Convert with Convert Everything" entry in the File Explorer right-click menu.
//! Written to HKEY_CURRENT_USER, so no administrator rights are needed. On Windows 11
//! it appears under "Show more options".

#[cfg(windows)]
mod imp {
    use anyhow::Result;
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    const KEYS: [&str; 2] = [r"Software\Classes\*\shell\ConvertEverything", r"Software\Classes\Directory\shell\ConvertEverything"];

    pub fn set_enabled(on: bool) -> Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if on {
            let exe = std::env::current_exe()?;
            let exe = exe.display();
            for key in KEYS {
                let (k, _) = hkcu.create_subkey(key)?;
                k.set_value("MUIVerb", &"Convert with Convert Everything")?;
                k.set_value("Icon", &format!("\"{exe}\",0"))?;
                k.set_value("MultiSelectModel", &"Player")?;
                let (c, _) = hkcu.create_subkey(format!(r"{key}\command"))?;
                c.set_value("", &format!("\"{exe}\" \"%1\""))?;
            }
        } else {
            for key in KEYS {
                let _ = hkcu.delete_subkey_all(key);
            }
        }
        Ok(())
    }

    pub fn is_enabled() -> bool {
        RegKey::predef(HKEY_CURRENT_USER).open_subkey(KEYS[0]).is_ok()
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn set_enabled(_on: bool) -> anyhow::Result<()> {
        Ok(())
    }
    pub fn is_enabled() -> bool {
        false
    }
}

pub use imp::{is_enabled, set_enabled};
