This is a Rust based repository to manage multiple Python virtual environments with ease, inspired by conda's CLI interface. Please follow these guidelines when contributing:

## Code Standards

### Required Before Each Commit

-  Run `cargo +stable fmt --all` to format the code
-  Run `cargo +stable clippy --all-targets --all-features -- -D warnings` to lint the code
-  Ensure all tests pass with `cargo test`

> [!NOTE]
>
> The commands above maybe outdated. Always check the latest workflow in the repository. e.g. [`lint-and-fmt.yml`](.github/workflows/lint-and-fmt.yml) and [`test.yml`](.github/workflows/test.yml).

### Development Flow

-  Please refer to the [README](./README.md) for user guidelines.
-  For development, ensure you have the latest version of Rust installed. All commands available in [`copilot-setup-steps.yml`](.github/workflows/copilot-setup-steps.yml) should be run before starting development.
-  Use `cargo run` to run the application locally.

For example, use these commands to create, activate, and install packages in a virtual environment:

```bash
cargo run -- init init.sh # Initialize the shell script, only once
source init.sh
cargo run -- create meow-env -p 3.14
cargo run -- activate meow-env
cargo run -- install ruff
cargo run -- env list  # should show global env meow-env
cargo run -- deactivate
cargo run -- env remove meow-env
cargo run -- env list  # should show no envs
```

## Repository Structure

-  `src/cli`: Contains the command-line interface logic.
-  `src/store`: Contains the storage logic for virtual environments.
-  `src/backends.rs`: Contains the backend logic for managing virtual environments. Currently, it supports `uv` backend only.
-  `src/envs.rs`: Contains the environment variables used in meowda.
-  `src/main.rs`: The entry point of the application.

## Key Guidelines

1. Follow Rust best practices and idiomatic patterns
2. Maintain existing code structure and organization
3. Use dependency injection patterns where appropriate
4. Write unit tests for new functionality. Use table-driven unit tests when possible.
5. Document public APIs and complex logic.
