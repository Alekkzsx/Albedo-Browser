use std::path::PathBuf;

pub fn cache_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        return known_folder_roaming_appdata().unwrap_or_else(fallback_home).join("Cache");
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(v) = std::env::var("XDG_CACHE_HOME") {
            if !v.is_empty() {
                return PathBuf::from(v);
            }
        }
        return fallback_home().join(".cache");
    }

    #[cfg(target_os = "macos")]
    {
        return home_dir().join("Library").join("Caches");
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        return fallback_home().join(".cache");
    }
}

pub fn config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        return known_folder_roaming_appdata().unwrap_or_else(fallback_home);
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(v) = std::env::var("XDG_CONFIG_HOME") {
            if !v.is_empty() {
                return PathBuf::from(v);
            }
        }
        return fallback_home().join(".config");
    }

    #[cfg(target_os = "macos")]
    {
        return home_dir().join("Library").join("Application Support");
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        return fallback_home().join(".config");
    }
}

fn fallback_home() -> PathBuf {
    home_dir()
}

fn home_dir() -> PathBuf {
    if let Ok(v) = std::env::var("HOME") {
        if !v.is_empty() {
            return PathBuf::from(v);
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(v) = std::env::var("USERPROFILE") {
            if !v.is_empty() {
                return PathBuf::from(v);
            }
        }
    }
    PathBuf::from(".")
}

#[cfg(target_os = "windows")]
fn known_folder_roaming_appdata() -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    type HResult = i32;
    const S_OK: HResult = 0;
    const KF_FLAG_DEFAULT: u32 = 0;
    const FOLDERID_RoamingAppData: windows_guid::Guid = windows_guid::Guid {
        d1: 0x3EB685DB,
        d2: 0x65F9,
        d3: 0x4CF6,
        d4: [0xA0, 0x3A, 0xE3, 0xEF, 0x65, 0x72, 0x9F, 0x3D],
    };

    #[repr(C)]
    struct Guid {
        d1: u32,
        d2: u16,
        d3: u16,
        d4: [u8; 8],
    }

    #[link(name = "shell32")]
    extern "system" {
        fn SHGetKnownFolderPath(
            rfid: *const Guid,
            dwFlags: u32,
            hToken: isize,
            ppszPath: *mut *mut u16,
        ) -> HResult;
    }

    #[link(name = "ole32")]
    extern "system" {
        fn CoTaskMemFree(pv: *mut core::ffi::c_void);
    }

    let mut raw: *mut u16 = std::ptr::null_mut();
    let guid = Guid {
        d1: FOLDERID_RoamingAppData.d1,
        d2: FOLDERID_RoamingAppData.d2,
        d3: FOLDERID_RoamingAppData.d3,
        d4: FOLDERID_RoamingAppData.d4,
    };
    let hr = unsafe {
        SHGetKnownFolderPath(&guid, KF_FLAG_DEFAULT, 0, &mut raw as *mut *mut u16)
    };
    if hr != S_OK || raw.is_null() {
        return None;
    }

    let mut len = 0usize;
    unsafe {
        while *raw.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(raw, len);
        let os = OsString::from_wide(slice);
        CoTaskMemFree(raw as *mut core::ffi::c_void);
        Some(PathBuf::from(os))
    }
}

#[cfg(target_os = "windows")]
mod windows_guid {
    pub struct Guid {
        pub d1: u32,
        pub d2: u16,
        pub d3: u16,
        pub d4: [u8; 8],
    }
}
