//! Detect if we're running as or are spawned by a process with
//! "Developer Tool" privileges.
//!
//! As configured in System Preferences > Security & Privacy > Developer Tools.

mod execution_policy_dynamic;

use execution_policy_dynamic::{EPDeveloperToolStatus, ExecutionPolicyHandle};

fn main() {
    tracing_subscriber::fmt::init();

    if let Some(handle) = ExecutionPolicyHandle::open() {
        let status = handle.check_status();
        match status {
            EPDeveloperToolStatus::NOT_DETERMINED => println!("not determined"),
            EPDeveloperToolStatus::RESTRICTED => println!("restricted"),
            EPDeveloperToolStatus::DENIED => println!("denied"),
            EPDeveloperToolStatus::AUTHORIZED => println!("authorized"),
            _ => println!("unknown"),
        }
    } else {
        println!("ExecutionPolicy framework not available");
    }
}
