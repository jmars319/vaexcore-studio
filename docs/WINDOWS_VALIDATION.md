# Windows Validation

Run this from the `vaexcore-studio` repo root on a real Windows machine:

```powershell
npm run validate:windows
```

The runner executes:

- `npm run test:scripts`
- `npm run typecheck --workspaces --if-present`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --workspace`
- `npm run app:build:windows`

If Windows application control blocks Rust test binary execution, the runner
falls back to `cargo test --workspace --no-run` to confirm the tests still
compile, then exits nonzero so the result is reported as partial rather than
fully validated.

No PowerShell-specific validation is faked by the script; it is a Node runner
that works from PowerShell, Windows Terminal, or a standard command prompt.
