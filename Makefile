release:
	cd clumsy-rt; wasm-pack build --release --features=web --target no-modules --out-dir wasm
	cp ./clumsy-rt/wasm/clumsy_rt_bg.wasm ./clumsy-rt/wasm/clumsy_rt.js ./web-view/www/
	cd web-view/www; npm run release

debug:
	cd clumsy-rt; wasm-pack build --dev --features=web --target no-modules --out-dir wasm
	cp ./clumsy-rt/wasm/clumsy_rt_bg.wasm ./clumsy-rt/wasm/clumsy_rt.js ./web-view/www/
	cd web-view/www; npm run build

start:
	cd web-view/www; npm run start

