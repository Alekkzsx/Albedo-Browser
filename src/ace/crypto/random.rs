use std::fs::File;
use std::io::Read;

pub fn get_random_bytes(dest: &mut [u8]) {
    if dest.is_empty() {
        return;
    }

    platform_get_random_bytes(dest)
        .unwrap_or_else(|e| panic!("ACE-Crypto: failed to get random bytes: {}", e));
}

#[cfg(target_os = "windows")]
fn platform_get_random_bytes(dest: &mut [u8]) -> Result<(), String> {
    #[link(name = "bcrypt")]
    extern "system" {
        fn BCryptGenRandom(
            halgorithm: *mut core::ffi::c_void,
            pb_buffer: *mut u8,
            cb_buffer: u32,
            flags: u32,
        ) -> i32;
    }

    const BCRYPT_USE_SYSTEM_PREFERRED_RNG: u32 = 0x0000_0002;
    let status = unsafe {
        BCryptGenRandom(
            core::ptr::null_mut(),
            dest.as_mut_ptr(),
            dest.len() as u32,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };

    if status == 0 {
        Ok(())
    } else {
        Err(format!("BCryptGenRandom failed with NTSTATUS {}", status))
    }
}

#[cfg(target_os = "linux")]
fn platform_get_random_bytes(dest: &mut [u8]) -> Result<(), String> {
    extern "C" {
        fn getrandom(buf: *mut core::ffi::c_void, buflen: usize, flags: u32) -> isize;
    }

    let mut filled = 0usize;
    while filled < dest.len() {
        let rc = unsafe {
            getrandom(
                dest[filled..].as_mut_ptr() as *mut core::ffi::c_void,
                dest.len() - filled,
                0,
            )
        };

        if rc > 0 {
            filled += rc as usize;
            continue;
        }

        if rc == -1 {
            return read_urandom(dest);
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn platform_get_random_bytes(dest: &mut [u8]) -> Result<(), String> {
    #[link(name = "Security", kind = "framework")]
    extern "C" {
        fn SecRandomCopyBytes(rnd: *const core::ffi::c_void, count: usize, bytes: *mut u8) -> i32;
    }

    let status = unsafe { SecRandomCopyBytes(core::ptr::null(), dest.len(), dest.as_mut_ptr()) };
    if status == 0 {
        Ok(())
    } else {
        Err(format!("SecRandomCopyBytes failed with status {}", status))
    }
}

#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
fn platform_get_random_bytes(dest: &mut [u8]) -> Result<(), String> {
    read_urandom(dest)
}

#[cfg(not(any(unix, target_os = "windows")))]
fn platform_get_random_bytes(dest: &mut [u8]) -> Result<(), String> {
    let _ = dest;
    Err("unsupported platform".to_string())
}

fn read_urandom(dest: &mut [u8]) -> Result<(), String> {
    let mut file = File::open("/dev/urandom").map_err(|e| e.to_string())?;
    file.read_exact(dest).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::get_random_bytes;

    #[test]
    fn fills_requested_buffer() {
        let mut buf = [0u8; 32];
        get_random_bytes(&mut buf);
        assert!(buf.iter().any(|&b| b != 0));
    }

    #[test]
    fn independent_calls_vary() {
        let mut a = [0u8; 16];
        let mut b = [0u8; 16];
        get_random_bytes(&mut a);
        get_random_bytes(&mut b);
        assert_ne!(a, b);
    }
}
