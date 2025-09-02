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
//!
//! NOTE: `addPolicyExceptionForURL:error:` probably isn't relevant for us,
//! that is more used for e.g. allowing running a recently downloaded binary
//! (and requires that you already have developer tool authorization).

use std::cell::Cell;
use std::ffi::{CStr, c_void};
use std::marker::PhantomData;
use std::rc::Rc;

use block2::{DynBlock, RcBlock};
use objc2::ffi::NSInteger;
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, Bool, NSObject};

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
            // SAFETY: `dlerror` is safe to call.
            let err = unsafe { libc::dlerror() };

            let err = if err.is_null() {
                None
            } else {
                // SAFETY: The error is a valid C string.
                Some(unsafe { CStr::from_ptr(err) })
            };
            tracing::debug!(?err, "failed loading ExecutionPolicy.framework");
            return None;
        }

        Some(Self(handle))
    }
}

impl Drop for ExecutionPolicyHandle {
    fn drop(&mut self) {
        // SAFETY: The handle is valid.
        let _ = unsafe { libc::dlclose(self.0) };
        // Ignore errors when closing. This is also what `libloading` does:
        // https://docs.rs/libloading/0.8.6/src/libloading/os/unix/mod.rs.html#374
    }
}

/// A  that interacts with the system via XPC.
///
/// See [`objc2_execution_policy::EPDeveloperTool`] for details.
///
/// [`objc2_execution_policy::EPDeveloperTool`]: https://docs.rs/objc2-execution-policy/0.3.1/objc2_execution_policy/struct.EPDeveloperTool.html
#[derive(Debug)]
pub struct EPDeveloperTool<'handle> {
    _handle: PhantomData<&'handle ExecutionPolicyHandle>,
    obj: Retained<NSObject>,
}

impl<'handle> EPDeveloperTool<'handle> {
    /// Call `+[EPDeveloperTool new]` to get a new handle.
    pub fn new(_handle: &'handle ExecutionPolicyHandle) -> Option<Self> {
        // Dynamically query the class (loading the framework with dlopen
        // above should have made this available).
        let Some(cls) = AnyClass::get(c"EPDeveloperTool") else {
            tracing::error!("failed finding `EPDeveloperTool` class");
            return None;
        };

        // SAFETY: The signature of +[EPDeveloperTool new] is correct and
        // the method is safe to call.
        let obj: Option<Retained<NSObject>> = unsafe { msg_send![cls, new] };

        let Some(obj) = obj else {
            tracing::error!("failed creating `EPDeveloperTool` instance");
            return None;
        };

        let _handle = PhantomData;
        Some(Self { _handle, obj })
    }

    /// Call `-[EPDeveloperTool authorizationStatus]`.
    pub fn authorization_status(&self) -> EPDeveloperToolStatus {
        // SAFETY: -[EPDeveloperTool authorizationStatus] correctly
        // returns EPDeveloperToolStatus and the method is safe to call.
        let status: NSInteger = unsafe { msg_send![&*self.obj, authorizationStatus] };
        EPDeveloperToolStatus(status)
    }

    /// Call `requestDeveloperToolAccessWithCompletionHandler:` and get the
    /// result.
    ////
    /// This allows the user to more easily see which application needs to be
    /// allowed (but _is_ also requesting higher privileges, so we need to be
    /// clear in messaging around that).
    pub fn request_access(&self) -> bool {
        // Wrapper to make the signature easier to write.
        fn inner(obj: &NSObject, block: &DynBlock<dyn Fn(Bool) + 'static>) {
            // SAFETY:
            // - The method is safe to call, and we provide a correctly typed
            //   block, and constrain the signature to be void / unit return.
            // - No Send/Sync requirements are needed, because the block is
            //   not marked @Sendable in Swift.
            // - The 'static requirement on the block is needed because the
            //   block is marked as @escaping in Swift. Note that the fact
            //   that the API is annotated as such is kind of weird, there
            //   isn't really a way that it could call this block on the
            //   current thread later (which is what a lone @escaping means).
            unsafe { msg_send![obj, requestDeveloperToolAccessWithCompletionHandler: block] }
        }

        let result = Rc::new(Cell::new(None));
        let result_clone = result.clone();
        let block = RcBlock::new(move |granted: Bool| result_clone.set(Some(granted.as_bool())));
        inner(&self.obj, &block);
        result.get().unwrap_or_else(|| {
            tracing::error!("failed getting result of -[EPDeveloperTool requestDeveloperToolAccessWithCompletionHandler:]");
            false
        })
    }
}

/// The Developer Tool status of the process.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EPDeveloperToolStatus(pub NSInteger);

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
