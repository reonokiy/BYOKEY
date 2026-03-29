.PHONY: build build-release desktop clean openapi

# Build the byokey binary (debug)
build:
	cargo build

# Build the byokey binary (release)
build-release:
	cargo build --release

# Regenerate OpenAPI spec and Swift client from Rust utoipa annotations
openapi: build
	cargo run -- openapi 2>/dev/null | python3 -m json.tool > desktop/Byokey/openapi.json
	cd desktop/Byokey && swift-openapi-generator generate openapi.json \
		--config openapi-generator-config.yaml \
		--output-directory Generated/

# Build the binary then open the Xcode project
desktop: build openapi
	open desktop/Byokey.xcodeproj

clean:
	cargo clean
