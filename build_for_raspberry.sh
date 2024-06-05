#export PKG_CONFIG_SYSROOT_DIR=/usr/arm-linux-gnueabihf/
#cargo build --release --target=armv7-unknown-linux-gnueabihf

# docker run --user "$(id -u)":"$(id -g)" -v "$PWD":/usr/src/sunny -w /usr/src/sunny rust:bullseye \
#     /bin/bash -c \
#     "rustup target add armv7-unknown-linux-gnueabihf;
#     sudo apt install arm-linux-gnueabihf-gcc;
#     cargo build --release --target=armv7-unknown-linux-gnueabihf"

docker build -f Dockerfile-build -t sunny-build-raspberry . && \
docker run --user "$(id -u)":"$(id -g)" -v "$PWD":/usr/src/sunny -w /usr/src/sunny sunny-build-raspberry cargo build --release --target=armv7-unknown-linux-gnueabihf

# build frontend
cd frontend/sunny-ui
npx vite build --base=/ .

cd ../../

rm -rf raspberry_build
mkdir -p raspberry_build

cp -r frontend/sunny-ui/dist/ raspberry_build/
cp frontend/sunny-ui/dist/index.html raspberry_build/
