# Webhook CLI

A token-authenticated, background-capable Rust CLI tool to run local shell scripts on OS bash via HTTP webhook requests.

## Features

- **Bearer Token & Custom Header Auth**: Restricts webhook access to configured bearer tokens or custom headers (e.g., `X-Gitlab-Token`, `X-My-Header`).
- **Dynamic Configuration**: Adding/deleting tokens or custom headers takes effect immediately without restarting the server.
- **Background Daemon**: Run the server silently as a background process decoupled from the terminal session.
- **Automatic Lifecycle Control**: Starting a new background server automatically shuts down any existing running instance.
- **Script Piping**: Webhook request body is piped directly to the target script's standard input (`stdin`).
- **Rich JSON Responses**: Returns the script's `stdout`, `stderr`, and `exit_code` as structured JSON.
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
  ./webhook [-p <PORT>] background [-n / --no-log]
  ```
- **Start in Foreground**:
  ```bash
  ./webhook [-p <PORT>] start --foreground [-n / --no-log]
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
For convenience, four quick start runner scripts are provided in the root directory. They all support passing an optional port number as the first argument (e.g. `./run_fg_log.sh 8080`), defaulting to `9090`.

- **Foreground with Logs**: `./run_fg_log.sh [PORT]`
- **Foreground without Logs**: `./run_fg_nolog.sh [PORT]`
- **Background with Logs**: `./run_bg_log.sh [PORT]`
- **Background without Logs**: `./run_bg_nolog.sh [PORT]`

---

## Webhook API Usage

- **Endpoint**: `POST http://localhost:<PORT>/webhook/{path_to_script.sh}` (default port is `9090`)
- **Authentication Headers**:
  - `Authorization: Bearer <TOKEN>` (Standard bearer token)
  - **OR** custom configured headers (e.g., `X-Gitlab-Token: <TOKEN>`, `X-My-Header: <TOKEN>`). If both `Authorization` and custom headers are sent, or only custom headers are sent, as long as any of the provided and configured headers are correct, the request is authorized. Custom headers do not require the `Bearer ` prefix.
- **Request Body**: Automatically piped to the target script's `stdin`.

### Example Requests (curl)

Using standard Bearer token:
```bash
curl -X POST \
  -H "Authorization: Bearer my-secret-token" \
  -d "Hello script!" \
  http://localhost:9090/webhook/path/to/script.sh
```

Using custom headers (e.g., `X-Gitlab-Token`):
```bash
curl -X POST \
  -H "X-Gitlab-Token: gitlab-secret-abc" \
  -d "Hello script!" \
  http://localhost:9090/webhook/path/to/script.sh
```

### JSON Response

```json
{
  "exit_code": 0,
  "stderr": "",
  "stdout": "Hello script received: Hello script!\n"
}
```
