# Distributed WASM - Web View

The frontend to the distributed WASM demo in this repository.

The frontend is built almost 100% in Rust, using WASM code and accessing web browser APIs through it.

For DOM manipulation and WebGL based rendering inside a canvas, I am re-using code I wrote a few years back for another project. It's an (incomplete!) game engine library called [paddle](https://github.com/jakmeier/paddle/).

For the WebRTC and WebSocket API, I use [web-sys](https://github.com/rustwasm/wasm-bindgen/tree/main/crates/web-sys) directly.

## Quick start

Requirements: [Rust](https://www.rust-lang.org/tools/install), [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/), [node](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm)

1. Build the WASM module and prepare it to be displayed on a website.

```bash
cd www
npm install
npm run release
```

2. Serve the website locally
```bash
npm run start
```

3. Open the link in the npm logs (e.g. http://localhost:8080/)