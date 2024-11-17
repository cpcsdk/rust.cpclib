RUSTFLAGS=

# Build the versio nthat runs within firefox
build:
	wasm-pack  build --target bundler  --out-dir www/pkg
	cd www; npm install

run:
	cd www; npm run start
	
test:
	wasm-pack test --firefox

clean:
	-rm -rf www/pkg