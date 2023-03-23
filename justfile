clippy:
    cargo +$(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "scoped-trace").rust_version') clippy --all-targets

fmt:
    cargo +nightly fmt

readme:
    cargo doc2readme
