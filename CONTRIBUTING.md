# Contributing to obzenflow-topology

Thanks for your interest in contributing!

By participating, you agree to follow the Code of Conduct (`CODE_OF_CONDUCT.md`).

## Sign-off (DCO)

We use the **Developer Certificate of Origin (DCO)** instead of a Contributor License Agreement (CLA).

- All commits in a PR must be signed off.
- Sign off your commits with: `git commit -s`
- The sign-off line looks like: `Signed-off-by: Your Name <your.email@example.com>`

The full text is in `DCO.md`.

### Fixing missing sign-offs

- Amend the most recent commit: `git commit --amend -s`
- Sign off all commits on your branch (interactive): `git rebase -i --signoff main`

## Contribution provenance

If you are employed, you are responsible for ensuring your employer's intellectual property policies permit your contribution. Many employment contracts include IP assignment clauses that may cover work done outside of office hours or on personal equipment.

If your employer requires a corporate sign-off or approval for open source contributions, please obtain it before submitting a pull request.
By signing off your commits (DCO), you attest you have the right to contribute the work under the project's license terms.

## Development Setup

### Prerequisites

- Rust toolchain (see `.github/workflows/ci.yml` for the pinned version used in CI)
- Cargo

### Building

```bash
# Native build
cargo build

# WASM build (compile check)
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown
```

### Testing

Run all tests:
```bash
cargo test
```

Run all tests with all features:
```bash
cargo test --all-features
```

Run a specific test:
```bash
cargo test test_name
```

Run tests with output:
```bash
cargo test -- --nocapture
```

## Code Style

- Follow standard Rust conventions
- Use `cargo fmt` to format your code
- Use `cargo clippy` to catch common mistakes
- Add documentation comments for public APIs
- Include examples in doc comments where appropriate

## Testing Guidelines

### Test Organization

- Unit tests go in the same file as the code they test (in `mod tests`)
- Integration tests go in the `tests/` directory
- Each test file should focus on a specific aspect of functionality
- Use descriptive test names that explain what is being tested

### Writing Tests

- Test both success and failure cases
- Test edge cases and boundary conditions
- Ensure tests are deterministic (avoid relying on timing unless necessary)

## Documentation

- All public APIs must have documentation comments
- Include examples in documentation where helpful
- Update the README if you add new features

## Pull Request Process

1. **Before submitting:**
   - Ensure all tests pass
   - Run `cargo fmt` and `cargo clippy`
   - Update documentation as needed

2. **PR Description:**
   - Clearly describe what the PR does
   - Reference any related issues
   - Include examples of usage if adding new features
   - List any breaking changes

3. **Review Process:**
   - PRs require at least one review before merging
   - Address reviewer feedback promptly
   - Keep PRs focused - one feature/fix per PR

## Types of Contributions

### Bug Reports

- Use the issue tracker to report bugs
- Include a minimal reproducible example
- Describe expected vs actual behavior
- Include environment details (OS, Rust version)

### Feature Requests

- Open an issue to discuss new features first
- Explain the use case and motivation

### Code Contributions

We especially welcome:
- Performance improvements
- Additional test coverage
- Documentation improvements
- Bug fixes
- New examples

## Questions?

Feel free to open an issue for any questions about contributing.

## License

By contributing, you agree that your contributions will be licensed under the project's dual license (MIT OR Apache-2.0).

## Source headers (SPDX)

All Rust source files (`*.rs`) must start with an SPDX header block.

Use:

```rust
// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev
```

Do not add individual names to per-file headers. Attribution lives in `LICENSE-MIT` and `LICENSE-APACHE`.

## Security

Please do not open public issues for security vulnerabilities. See `SECURITY.md`.
