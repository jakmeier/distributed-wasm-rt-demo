# Distributed WASM Demo

Blog post at: *TODO: add link once it's published*

Demo hosted on https://demos.jakobmeier.ch/distributed_wasm/.

Components:
- [Web-View](./web-view/)
- [Ray-Tracer](./clumsy-rt/)
- [Spin Component](./spin-component/)
- [Web-RTC signaling server](./webrtc-signaling-server/) and [shared lib with client](./ntmy/)

## Quick Start

Try it locally (requires [rust](https://www.rust-lang.org/tools/install), [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) and [npm](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm)):

```bash
make release
make start
```