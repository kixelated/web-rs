# web-rs
Rust bindings to make WASM more tolerable.

Nothing runs on pull requests: run `just check` before every push (`just setup` installs the tools it needs).

## Unstable API
Some crates use unstable `web_sys` APIs and you may need to set `--cfg=web_sys_unstable_apis` when compiling.
For more information, see the [web-sys docs](https://rustwasm.github.io/wasm-bindgen/web-sys/unstable-apis.html).

There's a few ways to set this depending on the environment:
- [Cargo Config](./.cargo/config.toml) via `rustflags`
- [GitHub Action](.github/workflows/release.yml) via `GITHUB_ENV`
- [docs.rs](./web-codecs/Cargo.toml) via `package.metadata.docs.rs`

## web-codecs
[web-codecs](./web-codecs) provides a wrapper around the [WebCodecs API](https://developer.mozilla.org/en-US/docs/Web/API/WebCodecs_API).

The callbacks have been replaced with a channel-like API.
For example, the `VideoEncoder` is split into a `VideoEncoder` for input and a `VideoEncoded` for output.

## web-streams
[web-streams](./web-streams) provides a wrapper around the [Streams API](https://developer.mozilla.org/en-US/docs/Web/API/Streams_API).

This API is annoyingly untyped when using web_sys.
This library handles the casting for you as well as providing guard-rails around the API (ex. closing on Drop).