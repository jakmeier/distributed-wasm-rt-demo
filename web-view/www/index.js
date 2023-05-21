import "./styles.css";
let pkg = await import('./wasm/web_view.js');

pkg.start();