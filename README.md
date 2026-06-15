# Webhook CLI

A token-authenticated, background-capable Rust CLI tool to run local shell scripts on OS bash via HTTP webhook requests.

## Features

- **Bearer Token & Custom Header Auth**: Restricts webhook access to configured bearer tokens or custom headers (e.g., `X-Gitlab-Token`, `X-My-Header`).
- **Dynamic Configuration**: Adding/deleting tokens or custom headers takes effect immediately without restarting the server.
- **Background Daemon**: Run the server silently as a background process decoupled from the terminal session.
- **Automatic Lifecycle Control**: Starting a new background server automatically shuts down any existing running instance.
- **Script Piping**: Webhook request body is piped directly to the target script's standard input (`stdin`).
- **Rich JSON Responses**: Returns the script's `stdout`, `stderr`, and `exit_code` as structured JSON (when running in blocking mode).
- **No-wait Mode (Async Execution)**: Global `-w` / `--no-wait` option to start the server in non-blocking mode. Callers can also control this on a per-request basis using query parameters (`?no_wait=true` or `?wait=false`), allowing immediate response returning `{"message": "Script started in background"}`.
- **Silent Mode**: Global `-n` / `--no-log` option to completely disable console/file output logs.

---

## Installation & Build

Build the project in release mode and copy the compiled binary to the root directory by running:

```bash
./build_release.sh
```

This will produce the `./webhook` executable.

---

## Command Reference

Run `./webhook --help` for the full instruction manual.

### 1. Token Management
Tokens are saved securely in `~/.config/webhook-daemon/config.json`.

- **Add a token**:
  ```bash
  ./webhook add <your-token>
  ```
- **List tokens**:
  ```bash
  ./webhook list
  ```
- **Delete a token**:
  ```bash
  ./webhook delete <your-token>
  ```

### 2. Custom Header Management
Configure custom headers to authenticate requests with a raw token (e.g., without the `Bearer` prefix).

- **Add a token for a custom header**:
  ```bash
  ./webhook add-header <HEADER> <your-token>
  ```
  *Example*: `./webhook add-header X-Gitlab-Token gitlab-secret-abc`
- **List custom headers**:
  ```bash
  ./webhook list-header
  ```
- **Delete a custom header or token**:
  ```bash
  ./webhook delete-header <HEADER> [your-token]
  ```
  *Note*: If `[your-token]` is omitted, the entire header and all its configured tokens will be deleted.

### 3. Server Control
- **Start in Background (Daemon)**:
  ```bash
  ./webhook [-p <PORT>] [-n / --no-log] [-w / --no-wait] background
  ```
- **Start in Foreground**:
  ```bash
  ./webhook [-p <PORT>] [-n / --no-log] [-w / --no-wait] start --foreground
  ```
- **Stop Daemon**:
  ```bash
  ./webhook stop
  ```
- **Check Status**:
  ```bash
  ./webhook status
  ```

### 4. Quick Start Helper Scripts
For convenience, helper runner scripts are provided in the root directory. They all support passing an optional port number as the first argument (e.g. `./run_fg_log.sh 8080`), defaulting to `9090`. You can also pass extra arguments to the CLI (e.g. `./run_fg_log.sh 9090 --no-wait`).

**Default (Blocking) Helper Scripts:**
- **Foreground with Logs**: `./run_fg_log.sh [PORT] [EXTRA_ARGS]`
- **Foreground without Logs**: `./run_fg_nolog.sh [PORT] [EXTRA_ARGS]`
- **Background with Logs**: `./run_bg_log.sh [PORT] [EXTRA_ARGS]`
- **Background without Logs**: `./run_bg_nolog.sh [PORT] [EXTRA_ARGS]`

**Async (No-wait) Helper Scripts:**
- **Foreground with Logs**: `./run_fg_log_nowait.sh [PORT] [EXTRA_ARGS]`
- **Foreground without Logs**: `./run_fg_nolog_nowait.sh [PORT] [EXTRA_ARGS]`
- **Background with Logs**: `./run_bg_log_nowait.sh [PORT] [EXTRA_ARGS]`
- **Background without Logs**: `./run_bg_nolog_nowait.sh [PORT] [EXTRA_ARGS]`

---

## Webhook API Usage

- **Endpoint**: `POST http://localhost:<PORT>/webhook/{path_to_script.sh}` (default port is `9090`)
- **Authentication Headers**:
  - `Authorization: Bearer <TOKEN>` (Standard bearer token)
  - **OR** custom configured headers (e.g., `X-Gitlab-Token: <TOKEN>`, `X-My-Header: <TOKEN>`).
- **Query Parameters (Execution Mode Override)**:
  - `?no_wait=true` or `?wait=false`: Enables No-wait Mode (Async Execution) for this request. The server returns immediately without blocking.
  - `?no_wait=false` or `?wait=true`: Disables No-wait Mode (Blocking Execution) for this request. The server waits for the script to finish and returns stdout/stderr/exit_code.
- **Request Body**: Automatically piped to the target script's `stdin`.

### Example Requests (curl)

Using standard Bearer token in default blocking mode:
```bash
curl -X POST \
  -H "Authorization: Bearer my-secret-token" \
  -d "Hello script!" \
  http://localhost:9090/webhook/path/to/script.sh
```

Response for blocking execution:
```json
{
  "exit_code": 0,
  "stderr": "",
  "stdout": "Hello script received: Hello script!\n"
}
```

Calling webhook in No-wait (Async) mode:
```bash
curl -X POST \
  -H "Authorization: Bearer my-secret-token" \
  -d "Hello script!" \
  "http://localhost:9090/webhook/path/to/script.sh?no_wait=true"
```

Response for async execution:
```json
{
  "message": "Script started in background"
}
```
