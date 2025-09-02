//! Detect if we're running as or are spawned by a process with
//! "Developer Tool" privileges.
//!
//! As configured in System Preferences > Security & Privacy > Developer Tools.

mod execution_policy_dynamic;
mod sip_detect_fs;

use execution_policy_dynamic::{EPDeveloperTool, EPDeveloperToolStatus, ExecutionPolicyHandle};

fn main() {
    tracing_subscriber::fmt::init();

    // If disabled, we don't actually care about the execution policy.
    println!(
        "SIP Filesystem Protections status: {:?}, {:?}, {:?}",
        sip_detect_fs::from_command(),
        sip_detect_fs::from_system_lib(),
        sip_detect_fs::from_fs_operation(),
    );

    let Some(handle) = ExecutionPolicyHandle::open() else {
        println!("ExecutionPolicy framework not available");
        return;
    };

    let Some(developer_tool) = EPDeveloperTool::new(&handle) else {
        println!("Failed initializing EPDeveloperTool");
        return;
    };

    let status = developer_tool.authorization_status();
    let status_str = match status {
        EPDeveloperToolStatus::NOT_DETERMINED => "not determined",
        EPDeveloperToolStatus::RESTRICTED => "restricted",
        EPDeveloperToolStatus::DENIED => "denied",
        EPDeveloperToolStatus::AUTHORIZED => "authorized",
        _ => "unknown",
    };
    println!("Execution policy status: {status_str}");

    let res = developer_tool.request_access();
    println!("Requested access, result: {res}");
}
