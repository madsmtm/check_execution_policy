//! Utilities to detect whether SIP Filesystem Protections are enabled.

use std::ffi::{CStr, c_int, c_void};
use std::io::{Error, ErrorKind};
use std::process::Command;

/// Fastest (?) and maybe most robust implementation (?): Look up the private
/// symbol `csr_get_active_config`.
///
/// <https://github.com/apple-oss-distributions/xnu/blob/xnu-11417.121.6/libsyscall/wrappers/csr.c#L37-L40>
pub fn from_system_lib() -> Option<bool> {
    pub struct LibSystemHandle(*mut c_void);

    impl Drop for LibSystemHandle {
        fn drop(&mut self) {
            let _ = unsafe { libc::dlclose(self.0) };
            // Ignore errors when closing. This is also what `libloading` does:
            // https://docs.rs/libloading/0.8.6/src/libloading/os/unix/mod.rs.html#374
        }
    }

    let handle = unsafe {
        libc::dlopen(
            c"/usr/lib/libSystem.dylib".as_ptr(),
            libc::RTLD_LAZY | libc::RTLD_LOCAL,
        )
    };
    if handle.is_null() {
        let err = unsafe { CStr::from_ptr(libc::dlerror()) };
        tracing::error!(?err, "failed loading libSystem.dylib");
        return None;
    }
    let handle = LibSystemHandle(handle);

    let symbol = unsafe { libc::dlsym(handle.0, c"csr_get_active_config".as_ptr()) };
    if symbol.is_null() {
        let err = unsafe { CStr::from_ptr(libc::dlerror()) };
        tracing::warn!(?err, "failed to find csr_get_active_config");
        return None;
    }

    let csr_get_active_config = unsafe {
        std::mem::transmute::<*const c_void, unsafe extern "C" fn(*mut u32) -> c_int>(symbol)
    };

    let mut sip_value: u32 = 0;
    let err = unsafe { csr_get_active_config(&mut sip_value as *mut u32) };

    if err != 0 {
        tracing::warn!(?err, "failed to call csr_get_active_config");
        return None;
    }

    const CSR_ALLOW_UNRESTRICTED_FS: u32 = 1 << 1;

    Some((sip_value & CSR_ALLOW_UNRESTRICTED_FS) == 0)
}

/// Alternative implementation: Invoke `csrutil status`, and parse the output.
///
/// Might fail if a weird PATH is set. Maybe should use `/usr/bin/csrutil`?
pub fn from_command() -> Option<bool> {
    let res = Command::new("csrutil")
        .arg("status")
        .output()
        .inspect_err(|err| {
            tracing::error!(?err, "failed invoking `csrutil status`");
        })
        .ok()?;

    if !res.status.success() {
        tracing::error!(?res, "`csrutil status` failed");
        return None;
    }

    let res = String::from_utf8(res.stdout).unwrap();

    if res.contains("Filesystem Protections: enabled")
        || res.contains("System Integrity Protection status: enabled")
    {
        Some(true)
    } else if res.contains("Filesystem Protections: disabled")
        || res.contains("System Integrity Protection status: disabled")
    {
        Some(false)
    } else {
        tracing::warn!(?res, "could not part `csrutil status` output");
        None
    }
}

/// Hacky way: Query the file system for write access to `/System`.
pub fn from_fs_operation() -> Option<bool> {
    let res = unsafe { libc::access(c"/System".as_ptr(), libc::W_OK) };
    if res == 0 {
        Some(false)
    } else {
        let err = Error::last_os_error();
        if err.kind() == ErrorKind::PermissionDenied {
            Some(true)
        } else if err.kind() == ErrorKind::ReadOnlyFilesystem {
            Some(false)
        } else {
            None
        }
    }
}
