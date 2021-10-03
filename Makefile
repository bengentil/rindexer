all: build

docker-image-arm:
	podman build -t ghcr.io/bengentil/rindexer:cross-armv7-unknown-linux-musleabihf-0.2.1-sqlite -f docker/Dockerfile.armv7-unknown-linux-musleabihf

docker-image-x86_64:
	podman build -t ghcr.io/bengentil/rindexer:cross-x86_64-unknown-linux-musl-0.2.1-sqlite -f docker/Dockerfile.x86_64-unknown-linux-musl

arm: docker-image-arm
	cross build --target=armv7-unknown-linux-musleabihf -v --release

x86_64: docker-image-x86_64
	cross build --target=x86_64-unknown-linux-musl -v --release

build: arm x86_64
	cargo build --release