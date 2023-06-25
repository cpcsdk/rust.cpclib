RUSTFLAGS=

build:
	wasm-pack  build --target web  --out-dir www/pkg
	cd www; npm install

run:
	cd www; npm run start
	
test:
	wasm-pack test --firefox