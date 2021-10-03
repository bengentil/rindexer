all: build

docker:
	podman build -t docker.io/rustembedded/cross:armv7-unknown-linux-musleabihf-0.2.1-sqlite docker

arm: docker
	cross build --target=armv7-unknown-linux-musleabihf -v --release

build: arm
	cargo build --release