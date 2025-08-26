//! Detect if we're running as or are spawned by a process with
//! "Developer Tool" privileges.
//!
//! As configured in System Preferences > Security & Privacy > Developer Tools.

mod execution_policy_dynamic;
mod sip_detect_fs;

use execution_policy_dynamic::{EPDeveloperToolStatus, ExecutionPolicyHandle};

fn main() {
    tracing_subscriber::fmt::init();

    // If disabled, we don't actually care about the execution policy.
    println!(
        "SIP Filesystem Protections status: {:?}, {:?}, {:?}",
        sip_detect_fs::from_command(),
        sip_detect_fs::from_system_lib(),
        sip_detect_fs::from_fs_operation(),
    );

    if let Some(handle) = ExecutionPolicyHandle::open() {
        let status = handle.check_status();
        let status = match status {
            EPDeveloperToolStatus::NOT_DETERMINED => "not determined",
            EPDeveloperToolStatus::RESTRICTED => "restricted",
            EPDeveloperToolStatus::DENIED => "denied",
            EPDeveloperToolStatus::AUTHORIZED => "authorized",
            _ => "unknown",
        };
        println!("Execution policy status: {status}");
    } else {
        println!("ExecutionPolicy framework not available");
    }
}
