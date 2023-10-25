# Distributed WASM - Spin Component

The ray tracer (code in [../clumsy-rt/](../clumsy-rt/README.md)) can be used as
a service that accepts rendering requests through HTTP GET calls. This allows to
collaboratively render an image in distributed fashion.

For this demo, Fermyon Spin is used.

> Spin is a framework for building and running event-driven microservice applications with WebAssembly (Wasm) components.

For more, see: https://developer.fermyon.com/spin/index

[Install Spin](https://developer.fermyon.com/spin/install) first and then run:

```bash
cd spin-component
spin build
spin up
```

This will start a service which listens to incoming requests on `127.0.0.1:3000`.

You can use [this hosted frontend](https://demos.jakobmeier.ch/distributed_wasm/)
and create "Localhost" workers to connect to send work to the service.

You may also run the frontend locally following the instructions in
[web-view](../web-view/README.md).