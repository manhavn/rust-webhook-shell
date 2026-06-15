# Webhook CLI

A token-authenticated, background-capable Rust CLI tool to run local shell scripts on OS bash via HTTP webhook requests.

## Features

- **Bearer Token Auth**: Restricts webhook access to configured tokens.
- **Dynamic Configuration**: Adding or deleting tokens takes effect immediately without restarting the server.
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

### 2. Server Control
- **Start in Background (Daemon)**:
  ```bash
  ./webhook background [-n / --no-log]
  ```
- **Start in Foreground**:
  ```bash
  ./webhook start --foreground [-n / --no-log]
  ```
- **Stop Daemon**:
  ```bash
  ./webhook stop
  ```
- **Check Status**:
  ```bash
  ./webhook status
  ```

---

## Webhook API Usage

- **Endpoint**: `POST http://localhost:9090/webhook/{path_to_script.sh}`
- **Required Header**: `Authorization: Bearer <TOKEN>`
- **Request Body**: Automatically piped to the target script's `stdin`.

### Example Request (curl)

Assuming you have configured `my-secret-token` and want to execute `/path/to/script.sh`:

```bash
curl -X POST \
  -H "Authorization: Bearer my-secret-token" \
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
