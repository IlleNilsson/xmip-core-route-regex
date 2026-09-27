# xmip-core-route-regex

Regex route technology: `regex:<property>:<pattern>` applies a pattern, compiled once as xmip-core-path-regex's Pattern, to another property's text and reads the first capture, the whole match or a #name group. A technology of [xmip-core-route](https://github.com/IlleNilsson/xmip-core-route).

## Toolchain

`rust-toolchain.toml` pins the toolchain for the whole estate. Do not change it
here.

## Verification

The included workflow is manual-only and calls the versioned shared workflow at
`IlleNilsson/.github@v1`.
