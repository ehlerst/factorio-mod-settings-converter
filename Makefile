.PHONY: all build check test clean integration

# Default target
all: build

# Build the project in release mode
build:
	cargo build --release

# Run cargo check
check:
	cargo check

# Run tests
test:
	cargo test

# Clean build artifacts
clean:
	cargo clean
	rm -f integration.json integration.dat

# Integration test: DAT -> JSON -> DAT and verify MD5
integration: build
	@echo "Converting mod-settings.dat to integration.json..."
	./target/release/factorio-mod-settings-converter mod-settings.dat integration.json
	@echo "Converting integration.json to integration.dat..."
	./target/release/factorio-mod-settings-converter integration.json integration.dat
	@echo "Verifying MD5 sums..."
	@ORIGINAL=$$(md5sum mod-settings.dat | cut -d' ' -f1); \
	RESULT=$$(md5sum integration.dat | cut -d' ' -f1); \
	if [ "$$ORIGINAL" = "$$RESULT" ]; then \
		echo "Success: MD5 sums match ($$ORIGINAL)"; \
	else \
		echo "Failure: MD5 sums do not match!"; \
		echo "Original: $$ORIGINAL"; \
		echo "Result:   $$RESULT"; \
		exit 1; \
	fi
