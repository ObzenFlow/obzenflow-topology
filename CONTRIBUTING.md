# Contributing to obzenflow-topology

Thank you for your interest in contributing to obzenflow-topology! We welcome contributions from the community.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/obzenflow-topology.git`
3. Create a feature branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Run tests: `cargo test`
6. Commit your changes: `git commit -am 'Add some feature'`
7. Push to the branch: `git push origin feature/your-feature-name`
8. Submit a pull request

## Development Setup

### Prerequisites

- Rust 1.70 or higher
- Cargo

### Building

```bash
cargo build
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` to check for common issues
- Follow Rust naming conventions
- Add documentation comments for public APIs

## Pull Request Guidelines

1. **Keep changes focused**: One feature or fix per PR
2. **Write tests**: Add tests for new functionality
3. **Update documentation**: Keep README and docs current
4. **Follow existing patterns**: Match the codebase style
5. **Write clear commit messages**: Explain what and why

## Testing

- Unit tests go in the same file as the code (`#[cfg(test)]` module)
- Integration tests go in the `tests/` directory
- Aim for high test coverage of critical paths

## Documentation

- All public APIs should have doc comments
- Include examples in doc comments where helpful
- Update README.md if adding new features

## Code of Conduct

Please be respectful and constructive in all interactions. We aim to maintain a welcoming and inclusive environment.

## Questions?

If you have questions, feel free to:
- Open an issue for discussion
- Ask in the pull request

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (MIT OR Apache-2.0).