# Checking macOS execution policies

Motivated by [discussion on the Rust Zulip](https://rust-lang.zulipchat.com/#narrow/channel/246057-t-cargo/topic/build.20scripts.20slow.20on.20macOS.3F) to allow detecting when launching binaries for the first time will take ~200ms for Gatekeeper to do its thing.

The idea would be to someday warn against this in Cargo or similar.

NOTE: It seems that **adding Terminal to Developer Tools and then removing it again doesn't take effect immediately**. So when testing the slowdown, you need to wait a little bit before relaunching Terminal, until XProtect (or whatever?) has cleared its cache.


## Xcode

Xcode avoids all this by having the `com.apple.private.tcc.allow` entitlement with the `kTCCServiceDeveloperTool` value set.

TODO: Could Cargo be signed in a way that it gets this entilement too? Probably not, right?

And even if it could, it wouldn't matter, since seems to only be "top-level" processes that matter?


## Debuggers

By default, LLDB resets TCC (Transparency, Consent, and Control) provenance, requiring the process being debugged itself to be marked as a Developer Tool. You can opt to instead forward the parent process' permissions with the `target.inherit-tcc` setting:

```sh
lldb -O 'settings set target.inherit-tcc true' target/debug/check_execution_policy
```

## Resources

Various resources:
- `man DevToolsSecurity`.
- `man spctl`.
- `man csrutil`.
- `man tccutil`.
- `man xprotect`.
- <https://support.apple.com/en-gb/guide/security/sec469d47bd8/web>
- <https://zeroclick.sh/blog/macos-tcc/>
- <https://book.hacktricks.wiki/en/macos-hardening/macos-security-and-privilege-escalation/macos-security-protections/macos-tcc/index.html>
- <https://newosxbook.com/ent.php> (with com.apple.private.tcc.allow)
