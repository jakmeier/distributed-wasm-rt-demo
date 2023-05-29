importScripts('./clumsy_rt.js');

const { render } = wasm_bindgen;

async function run_in_worker() {
    // Load the wasm file by awaiting the Promise returned by `wasm_bindgen`.
    await wasm_bindgen('./clumsy_rt_bg.wasm');
    // Set callback to handle messages passed to the worker.
    self.onmessage = async event => {
        let job = event.data;
        let png = render(job);

        // Send response back to be handled by callback in main thread.
        self.postMessage(png);
    };    
    self.postMessage("ready");
}

run_in_worker();