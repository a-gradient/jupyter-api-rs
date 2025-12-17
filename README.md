# jupyter_shell

jupyter_shell is a small CLI tool (Rust) with supporting Python helpers to interact with a running Jupyter server using scp/ftp-like subcommands. The project contains a Rust binary, API client code in `src/api`, CLI entrypoints under `cli/`, and sample Jupyter REST responses used for tests and offline development.

Quick overview

- Language: Rust (primary); Python helpers for local testing.
- Binary: `jupyter_shell` (built with `cargo`).
- Purpose: copy files to/from a Jupyter server, inspect kernels/sessions, and provide scripted access using a Jupyter token for authentication.

Build

- Prerequisites:
	- Install Rust (rustup) and `cargo` from https://rustup.rs.
	- Python 3.8+ if you plan to use the included helpers.

- Build the binary:

```bash
cargo build --release -F cli
```

- Output:

- `target/release/jupyter_shell` — optimized release build
- `target/debug/jupyter_shell` — debug build (default for `cargo run`)

Usage

The binary exposes subcommands implemented in the `cli/` folder. Replace `./target/release/jupyter_shell` with `cargo run -F cli --` if you prefer running from source.

- Copy a file to the Jupyter server (scp-like):

```bash
./target/release/jupyter_shell scp --token-file .secret -p 7021 http://localhost:8888 path/to/local.file remote:path/on/server
```

- Copy a file from the Jupyter server to local:

```bash
./target/release/jupyter_shell scp --token-file .secret -p 7021 http://localhost:8888 remote:path/on/server path/to/local.file
```

- Example: list kernels or sessions (API helper):

```bash
./target/release/jupyter_shell api list-kernels --token-file .secret http://localhost:8888
```

- Run via `cargo run` (debug):

```bash
cargo run -- scp --token-file .secret -p 7021 http://localhost:8888 path/to/local.file remote/path/on/server
```

Configuration & Authentication

- Token file: the CLI supports `--token-file <path>` which should contain only the Jupyter token string. Example `.secret` contents:

```
0123456789abcdef0123456789abcdef
```

- Environment variable: you can also set `JUPYTER_TOKEN` in your shell and adapt the CLI or wrapper scripts to use it. See `src/api/client.rs` for how the client loads tokens in this repository.

- `spec.yaml`: This project references the Jupyter Server API specification. A useful upstream reference is:

https://github.com/jupyter-server/jupyter_server/blob/main/jupyter_server/services/api/api.yaml

Samples & tests

- `samples/` contains JSON responses captured from a Jupyter server for endpoints such as `/api/sessions`, `/api/kernels`, `/api/contents`, etc. These are used for tests and offline development.
- Run Rust tests:

```bash
uv run python start_jupyterlab.py &
cargo test
kill % # kill the background Jupyter server
```

Acknowledgements

- API spec reference: the Jupyter Server API definition at the link above.
