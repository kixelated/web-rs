# web-rs
Rust bindings to make WASM more tolerable.

## web-codecs
[web-codecs](./web-codecs) provides a wrapper around the [WebCodecs API](https://developer.mozilla.org/en-US/docs/Web/API/WebCodecs_API).

The callbacks and Streams have been replaced with a channel-like API.
For example, the `VideoEncoder` is split into a `VideoEncoder` for input and a `VideoEncoded` for output.

## web-streams
[web-streams](./web-streams) provides a wrapper around the [Streams API](https://developer.mozilla.org/en-US/docs/Web/API/Streams_API).

This API is annoyingly untyped when using web_sys.
This library handles the casting for you as well as providing guard-rails around the API (ex. closing on Drop).