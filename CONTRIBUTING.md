Contributing to wicket-rs

Thank you for your interest in contributing! To maintain a high-quality codebase and a consistent developer experience, please follow these guidelines.
🛠 Development Environment

    Rust Version: We target the latest stable toolchain. Ensure yours is up to date: rustup update stable.

    Hooks: We recommend running cargo fmt and cargo clippy before every commit.

🏗 Architectural Conventions
1. Module Naming (The "No-Stutter" Rule)

We avoid redundancy in module paths. Since this crate is named wicket-core, the context of "wicket" and "core" is already implied.

    ❌ Avoid: wicket_core::wicket_auth or wicket_core::core_utils

    ✅ Prefer: wicket_core::auth or wicket_core::utils

2. Module Structure

We use the Rust 2018+ file structure. Avoid using mod.rs files unless absolutely necessary for specific re-exporting patterns.

    Prefer src/auth.rs for a module's entry point.

    Sub-modules belong in src/auth/submodule.rs.

3. Dependency Management

To keep Cargo.toml readable and minimize merge conflicts, dependencies must be:

    Alphabetized: Sorted A-Z within their respective groups.

🧪 Testing & Quality

    Unit Tests: Place unit tests in the same file as the code being tested using a mod tests block at the bottom.

    Documentation: All public functions, traits, and structs must have doc comments (///). Provide examples for complex logic.

    Lints: We treat Clippy warnings as errors. Your PR will not pass CI if it contains warnings.
    Bash

    cargo clippy --all-targets -- -D warnings

🚀 Pull Request Process

    Branch Naming: Use descriptive names (e.g., feature/auth-provider or fix/connection-leak).

    Commit Messages: Follow Conventional Commits (e.g., feat: add oauth2 support).

    Refactoring: If you are performing a large rename (like fixing stuttered modules), do the move in a dedicated commit before changing any logic. This helps Git track file history correctly.
