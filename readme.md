# gentle

The low configuration build system that gets out of your way, like a gentleman.

## Draft

This project is *NOT* stable and should not be relied on for critical systems.
It exists, at the moment, to suit the needs of the WalletOS project.
Features will be added according to that project's needs.

## Model

Gentle aims to build, test, lint, format, etc. your project with no configuration.
It infers the targets that exist at various directories using marker files for the appropriate language.
Ex: `Cargo.toml` for Rust, `go.mod` for Go.

It currently uses the system's version of tools, but we have plans to have Gentle install specific versions of tools.

## Installation

Copy the `./gtl` file to the root of your repository.

## Usage

All commands should be run through the `./gtl` script at the root of your repository.
You can run `./gtl test` to test all the targets in your repo.

### Caching

Gentle uses the caches from the various build tools.
So `target/` for Rust, and `GOCACHE` for Go.

You can save these cached files to a single directory suitable for CI caching with the `./gtl cache-save <out-dir>` command.
You can then load these files in future CI runs with `./gtl cache-load <out-dir>`.
For example, your basic CI setup can look like the following:

```sh
load_cache_from_CI_tool()

./gtl cache-load ~/.gentle_cache # Command will succeed, even if the directory does not exist.
./gtl test
./gtl cache-save ~/.gentle_cache

save_cache_to_CI_tool()
```
