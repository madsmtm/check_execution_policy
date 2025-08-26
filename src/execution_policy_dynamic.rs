//! Utilities for using a dynamically loaded ExecutionPolicy.framework.
//!
//! ExecutionPolicy is only available since macOS 10.15, while Rust's
//! minimum supported version for host tooling is macOS 10.12:
//! https://doc.rust-lang.org/rustc/platform-support/apple-darwin.html#host-tooling
//!
//! For this reason, we must load the framework dynamically instead of linking
//! it statically - which gets a bit more involved.
//!
//! See <https://docs.rs/objc2-execution-policy> for a safer interface that
//! can be used if support for lower macOS versions are dropped (or once Rust
//! gains better support for weak linking).

use std::ffi::{CStr, c_void};

use objc2::msg_send;
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::{AnyClass, NSObject};

/// A handle to the dynamically loaded ExecutionPolicy framework.
#[derive(Debug)]
pub struct ExecutionPolicyHandle(*mut c_void);

impl ExecutionPolicyHandle {
    /// Dynamically load the ExecutionPolicy framework, and return None if it
    /// isn't available.
    pub fn open() -> Option<Self> {
        let path = c"/System/Library/Frameworks/ExecutionPolicy.framework/ExecutionPolicy";

        let handle = unsafe { libc::dlopen(path.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL) };

        if handle.is_null() {
            let err = unsafe { CStr::from_ptr(libc::dlerror()) };
            tracing::debug!(?err, "failed loading ExecutionPolicy.framework");
            return None;
        }

        Some(Self(handle))
    }

    /// Call the equivalent of:
    /// ```objc
    /// [[EPDeveloperTool new] authorizationStatus]
    /// ```
    pub fn check_status(&self) -> EPDeveloperToolStatus {
        // Use an autoreleasepool to .
        autoreleasepool(|_| {
            // Dynamically query the class.
            let Some(cls) = AnyClass::get(c"EPDeveloperTool") else {
                tracing::error!("failed finding `EPDeveloperTool` class");
                return EPDeveloperToolStatus::NOT_DETERMINED;
            };

            // SAFETY: The signature of +[EPDeveloperTool new] is correct and
            // the method is safe to call.
            let obj: Option<Retained<NSObject>> = unsafe { msg_send![cls, new] };

            let Some(obj) = obj else {
                tracing::error!("failed creating `EPDeveloperTool` instance");
                return EPDeveloperToolStatus::NOT_DETERMINED;
            };

            // SAFETY: The signature of -[EPDeveloperTool authorizationStatus]
            // is correct and the method is safe to call.
            let status: isize = unsafe { msg_send![&*obj, authorizationStatus] };
            EPDeveloperToolStatus(status)
        })
    }

    // `requestDeveloperToolAccessWithCompletionHandler` might be useful at
    // some point, to allow the user to more easily see which application
    // needs to be allowed.
    //
    // addPolicyExceptionForURL:error: probably isn't relevant, that is more
    // used for e.g. allowing running a recently downloaded application.
}

impl Drop for ExecutionPolicyHandle {
    fn drop(&mut self) {
        // Ignore errors when closing. This is also what `libloading` does:
        // https://docs.rs/libloading/0.8.6/src/libloading/os/unix/mod.rs.html#374
        let _ = unsafe { libc::dlclose(self.0) };
    }
}

/// The Developer Tool status of the process.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EPDeveloperToolStatus(pub isize);

impl EPDeveloperToolStatus {
    #[doc(alias = "EPDeveloperToolStatusNotDetermined")]
    pub const NOT_DETERMINED: Self = Self(0);
    #[doc(alias = "EPDeveloperToolStatusRestricted")]
    pub const RESTRICTED: Self = Self(1);
    #[doc(alias = "EPDeveloperToolStatusDenied")]
    pub const DENIED: Self = Self(2);
    #[doc(alias = "EPDeveloperToolStatusAuthorized")]
    pub const AUTHORIZED: Self = Self(3);
}
