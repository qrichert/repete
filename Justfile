lint:
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings -W clippy::all -W clippy::pedantic

test:
    cargo test --all-targets
