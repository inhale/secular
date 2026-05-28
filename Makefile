# Secular Core — Makefile
# Helper commands for building the Rust core library

.PHONY: all build test clean bindgen

all: build

# Build core with all features
build:
	cd secular-core && cargo build --all-features

# Build universal2 (macOS only)
build-mac-universal:
	cd secular-core && \
		cargo build --release --target aarch64-apple-darwin --all-features && \
		cargo build --release --target x86_64-apple-darwin --all-features && \
		mkdir -p target/universal && \
		lipo -create \
			target/aarch64-apple-darwin/release/libsecular_core.a \
			target/x86_64-apple-darwin/release/libsecular_core.a \
			-output target/universal/libsecular_core.a

# Run unit tests
test:
	cd secular-core && cargo test --all-features

# Generate Swift/Kotlin bindings
bindgen:
	cd secular-core && cargo build --features uniffi

# Verify no arm64-only files (universal2 check)
verify-universal:
	@find secular-core/target -name "*.a" -o -name "*.dylib" | while read f; do
		echo "=== $$f ==="
		file "$$f"
	done

# Clean build artifacts
clean:
	cd secular-core && cargo clean
