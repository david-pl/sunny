cargo build
rm -rf local_build
mkdir -p local_build
cp target/debug/sunny* local_build/

cd frontend/sunny-ui
npx vite build --base=/ .
cp -r dist/* ../../local_build/
cp dist/index.html ../../local_build