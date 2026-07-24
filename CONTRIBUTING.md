# Contributing to Chiaroscuro

Thanks for contributing to Chiaroscuro. The project is an iRacing-only telemetry viewer; support for other simulators is out of scope.

## Development setup

Install the stable Rust toolchain with the components declared in `rust-toolchain.toml`. Windows is the primary development platform because the application integrates with iRacing.

Run the project checks before opening a pull request:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For changes that affect the desktop application, also launch it on Windows and manually verify the affected behavior.

## Pull requests

- Keep each pull request focused on one change.
- Explain the motivation and implementation in the pull request description.
- Add or update tests when behavior changes.
- Include screenshots or recordings for user-interface changes.
- Describe any impact on telemetry, IBT files, or iRacing SDK integration.

## Issues

Use the provided issue forms for bugs and feature requests. Before opening an issue, search existing issues for duplicates and remove sensitive information from logs, screenshots, and telemetry data.
