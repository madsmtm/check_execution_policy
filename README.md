# Checking macOS execution policies

Motivated by [discussion on the Rust Zulip](https://rust-lang.zulipchat.com/#narrow/channel/246057-t-cargo/topic/build.20scripts.20slow.20on.20macOS.3F) to allow detecting when launching binaries for the first time will take ~200ms for Gatekeeper to do its thing.

The idea would be to someday warn against this in Cargo or similar.


## Debuggers

By default, LLDB resets TCC (Transparency, Consent, and Control) provenance, requiring the process being debugged itself to be marked as a Developer Tool. You can opt to instead forward the parent process' permissions with the `target.inherit-tcc` setting:

```sh
lldb -O 'settings set target.inherit-tcc true' target/debug/check_execution_policy
```
