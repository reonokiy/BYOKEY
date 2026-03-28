.PHONY: build build-release desktop clean

# Build the byokey binary (debug)
build:
	cargo build

# Build the byokey binary (release)
build-release:
	cargo build --release

# Build the binary then open the Xcode project
desktop: build
	open desktop/Byokey.xcodeproj

clean:
	cargo clean
