use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::any,
    Router,
};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;

#[derive(Parser, Debug)]
#[command(name = "webhook-cli")]
#[command(author = "Antigravity")]
#[command(version = "0.1.0")]
#[command(
    about = "Rust CLI tool to run shell scripts via Webhook with Bearer token or custom header authentication",
    long_about = "PROGRAM USAGE GUIDE:\n\n\
    * Tokens storage path: ~/.config/webhook-daemon/config.json\n\n\
    1. Authenticated Token Management:\n\
       - Add token:       webhook add <TOKEN>\n\
       - List tokens:     webhook list\n\
       - Delete token:    webhook delete <TOKEN>\n\n\
    2. Custom Header Management:\n\
       - Add header:      webhook add-header <HEADER> <TOKEN>\n\
       - List headers:    webhook list-header\n\
       - Delete header:   webhook delete-header <HEADER> [TOKEN]\n\n\
    3. Launch Webhook Server:\n\
       - Run in Background:  webhook [-p <PORT>] background [-n / --no-log]\n\
       - Run in Foreground:  webhook [-p <PORT>] start --foreground [-n / --no-log]\n\n\
       * Note: The 'background' command will automatically stop the running daemon (if any) before starting a new one.\n\n\
    4. Stop and Status Check:\n\
       - Check status:       webhook status\n\
       - Stop daemon:        webhook stop\n\n\
    5. Call Webhook:\n\
       - Endpoint: POST http://localhost:<PORT>/webhook/{path_to_script.sh}\n\
       - Required Header: Authorization: Bearer <TOKEN>\n\
         OR custom headers (e.g. X-Gitlab-Token: <TOKEN>, X-My-Header: <TOKEN>)\n\
       - Request body will be piped into the standard input (stdin) of the shell script."
)]
struct Cli {
    /// Port to listen on (overrides config file value)
    #[arg(short = 'p', long = "port", global = true)]
    port: Option<u16>,

    /// Disable writing logs (overrides config file value)
    #[arg(short = 'n', long = "no-log", global = true)]
    no_log: bool,

    /// Execute shell scripts in the background without waiting for results (overrides config file value)
    #[arg(short = 'w', long = "no-wait", global = true)]
    no_wait: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Add a new bearer token to the configuration
    Add {
        /// The bearer token to add
        token: String,
    },
    /// List all configured bearer tokens
    List,
    /// Delete a bearer token from the configuration
    Delete {
        /// The bearer token to delete
        token: String,
    },
    /// Add a custom header token to the configuration
    AddHeader {
        /// The custom header name (e.g. X-My-Header)
        header: String,
        /// The token value to add
        token: String,
    },
    /// List all configured custom headers and their tokens
    ListHeader,
    /// Delete a custom header token or the entire header from the configuration
    DeleteHeader {
        /// The custom header name
        header: String,
        /// The token value to delete (if omitted, the entire header and all its tokens will be deleted)
        token: Option<String>,
    },
    /// Start the webhook server in the background
    Background,
    /// Start the webhook server (foreground or background)
    Start {
        /// Run in the foreground instead of background
        #[arg(short, long)]
        foreground: bool,
    },
    /// Stop the background webhook server daemon
    Stop,
    /// Show the status of the background webhook server daemon
    Status,
}

fn default_port() -> u16 {
    9090
}

fn default_no_log() -> bool {
    false
}

fn default_no_wait() -> bool {
    false
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Config {
    #[serde(default)]
    tokens: Vec<String>,
    #[serde(default)]
    headers: std::collections::HashMap<String, Vec<String>>,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_no_log")]
    no_log: bool,
    #[serde(default = "default_no_wait")]
    no_wait: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tokens: Vec::new(),
            headers: std::collections::HashMap::new(),
            port: default_port(),
            no_log: default_no_log(),
            no_wait: default_no_wait(),
        }
    }
}

fn get_config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".config").join("webhook-daemon")
}

fn get_pid_file_path() -> PathBuf {
    get_config_dir().join("daemon.pid")
}

fn get_log_file_path() -> PathBuf {
    get_config_dir().join("daemon.log")
}

fn load_config() -> Config {
    let path = get_config_dir().join("config.json");
    if !path.exists() {
        return Config::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

fn save_config(config: &Config) -> std::io::Result<()> {
    let dir = get_config_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("config.json");
    let data = serde_json::to_string_pretty(config)?;
    std::fs::write(path, data)
}

fn handle_add(token: String) {
    let mut config = load_config();
    let path = get_config_dir().join("config.json");
    if config.tokens.contains(&token) {
        println!("Token already exists!");
        return;
    }
    config.tokens.push(token);
    if let Err(e) = save_config(&config) {
        eprintln!("Error saving config: {}", e);
    } else {
        println!("Token added successfully.");
        println!("Tokens storage path: {}", path.display());
    }
}

fn handle_list() {
    let config = load_config();
    let path = get_config_dir().join("config.json");
    println!("Tokens storage path: {}", path.display());
    if config.tokens.is_empty() {
        println!("No bearer tokens configured. Use 'add <token>' to add one.");
        return;
    }
    println!("Configured Bearer Tokens:");
    for (i, token) in config.tokens.iter().enumerate() {
        println!("{}. {}", i + 1, token);
    }
}

fn handle_delete(token: String) {
    let mut config = load_config();
    let path = get_config_dir().join("config.json");
    if let Some(pos) = config.tokens.iter().position(|t| t == &token) {
        config.tokens.remove(pos);
        if let Err(e) = save_config(&config) {
            eprintln!("Error saving config: {}", e);
        } else {
            println!("Token deleted successfully.");
            println!("Tokens storage path: {}", path.display());
        }
    } else {
        println!("Token not found in configuration.");
    }
}

fn handle_add_header(header: String, token: String) {
    let mut config = load_config();
    let path = get_config_dir().join("config.json");
    
    let normalized_header = header.to_lowercase();
    let tokens = config.headers.entry(normalized_header.clone()).or_insert_with(Vec::new);
    
    if tokens.contains(&token) {
        println!("Token already exists for header '{}'!", header);
        return;
    }
    tokens.push(token);
    if let Err(e) = save_config(&config) {
        eprintln!("Error saving config: {}", e);
    } else {
        println!("Token added successfully for header '{}'.", header);
        println!("Tokens storage path: {}", path.display());
    }
}

fn handle_list_header() {
    let config = load_config();
    let path = get_config_dir().join("config.json");
    println!("Tokens storage path: {}", path.display());
    if config.headers.is_empty() {
        println!("No custom headers configured. Use 'add-header <header> <token>' to add one.");
        return;
    }
    println!("Configured Custom Headers:");
    let mut sorted_keys: Vec<&String> = config.headers.keys().collect();
    sorted_keys.sort();
    
    for header in sorted_keys {
        if let Some(tokens) = config.headers.get(header) {
            println!("  {}:", header);
            for (i, token) in tokens.iter().enumerate() {
                println!("    {}. {}", i + 1, token);
            }
        }
    }
}

fn handle_delete_header(header: String, token: Option<String>) {
    let mut config = load_config();
    let path = get_config_dir().join("config.json");
    let normalized_header = header.to_lowercase();
    
    if !config.headers.contains_key(&normalized_header) {
        println!("Header '{}' not found in configuration.", header);
        return;
    }
    
    if let Some(tok) = token {
        let mut remove_header = false;
        if let Some(tokens) = config.headers.get_mut(&normalized_header) {
            if let Some(pos) = tokens.iter().position(|t| t == &tok) {
                tokens.remove(pos);
                println!("Token deleted successfully from header '{}'.", header);
                if tokens.is_empty() {
                    remove_header = true;
                }
            } else {
                println!("Token not found for header '{}' in configuration.", header);
                return;
            }
        }
        if remove_header {
            config.headers.remove(&normalized_header);
        }
    } else {
        config.headers.remove(&normalized_header);
        println!("Header '{}' and all its tokens deleted successfully.", header);
    }
    
    if let Err(e) = save_config(&config) {
        eprintln!("Error saving config: {}", e);
    } else {
        println!("Tokens storage path: {}", path.display());
    }
}

fn is_process_running(pid: i32) -> bool {
    unsafe {
        // kill(pid, 0) returns 0 if process exists
        libc::kill(pid, 0) == 0
    }
}

fn stop_daemon() {
    let pid_file = get_pid_file_path();
    if !pid_file.exists() {
        println!("No daemon PID file found. Daemon is likely not running.");
        return;
    }
    
    let content = match std::fs::read_to_string(&pid_file) {
        Ok(s) => s,
        Err(_) => {
            println!("Could not read PID file.");
            return;
        }
    };
    
    let parts: Vec<&str> = content.trim().split(':').collect();
    let pid = match parts.first().and_then(|p| p.parse::<i32>().ok()) {
        Some(p) => p,
        None => {
            println!("Invalid PID in PID file.");
            return;
        }
    };
    
    if !is_process_running(pid) {
        println!("Daemon (PID {}) is not running. Cleaning up stale PID file.", pid);
        let _ = std::fs::remove_file(&pid_file);
        return;
    }
    
    println!("Stopping daemon (PID {})...", pid);
    unsafe {
        libc::kill(pid, 15); // SIGTERM
    }
    
    // Wait for process to exit
    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if !is_process_running(pid) {
            println!("Daemon stopped successfully.");
            let _ = std::fs::remove_file(&pid_file);
            return;
        }
    }
    
    println!("Daemon did not respond to SIGTERM. Force killing (SIGKILL)...");
    unsafe {
        libc::kill(pid, 9); // SIGKILL
    }
    let _ = std::fs::remove_file(&pid_file);
}

fn show_status() {
    let pid_file = get_pid_file_path();
    let config_path = get_config_dir().join("config.json");
    if !pid_file.exists() {
        println!("Status: Stopped");
        println!("Tokens storage path: {}", config_path.display());
        return;
    }
    
    if let Ok(content) = std::fs::read_to_string(&pid_file) {
        let parts: Vec<&str> = content.trim().split(':').collect();
        if let Some(pid_str) = parts.first() {
            if let Ok(pid) = pid_str.parse::<i32>() {
                let port = parts.get(1).and_then(|p| p.parse::<u16>().ok()).unwrap_or(9090);
                let no_log = parts.get(2).and_then(|s| s.parse::<bool>().ok()).unwrap_or(false);
                let no_wait = parts.get(3).and_then(|s| s.parse::<bool>().ok()).unwrap_or(false);
                if is_process_running(pid) {
                    println!("Status: Running (PID: {})", pid);
                    println!("Listening on port {}", port);
                    if no_log {
                        println!("Logs: Disabled");
                    } else {
                        println!("Logs: {}", get_log_file_path().display());
                    }
                    if no_wait {
                        println!("No-wait Mode: Enabled");
                    } else {
                        println!("No-wait Mode: Disabled");
                    }
                    println!("Tokens storage path: {}", config_path.display());
                    return;
                }
            }
        }
    }
    println!("Status: Stopped (stale PID file found)");
    println!("Tokens storage path: {}", config_path.display());
}

fn spawn_background_process(no_log: bool, no_wait: bool, port: u16) -> std::io::Result<()> {
    use std::os::unix::process::CommandExt;
    
    let current_exe = std::env::current_exe()?;
    let log_path = get_log_file_path();
    
    std::fs::create_dir_all(get_config_dir())?;
    
    let (stdout_sink, stderr_sink) = if no_log {
        (std::process::Stdio::null(), std::process::Stdio::null())
    } else {
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_path)?;
        let err_file = log_file.try_clone()?;
        (std::process::Stdio::from(log_file), std::process::Stdio::from(err_file))
    };
        
    let mut cmd = std::process::Command::new(current_exe);
    cmd.arg("start");
    cmd.arg("--foreground");
    cmd.arg("--port");
    cmd.arg(port.to_string());
    if no_log {
        cmd.arg("--no-log");
    }
    if no_wait {
        cmd.arg("--no-wait");
    }
    
    cmd.stdout(stdout_sink);
    cmd.stderr(stderr_sink);
    cmd.stdin(std::process::Stdio::null());
    
    // Detach the child process by creating a new session
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    
    let mut child = cmd.spawn()?;
    let pid = child.id();
    
    // Wait a short moment to check if process exits immediately
    std::thread::sleep(std::time::Duration::from_millis(500));
    match child.try_wait() {
        Ok(Some(status)) => {
            eprintln!("Daemon process exited immediately with status: {}", status);
            if !no_log {
                if let Ok(logs) = std::fs::read_to_string(&log_path) {
                    eprintln!("Daemon output:\n{}", logs);
                }
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Daemon exited immediately with status {}", status),
            ));
        }
        Ok(None) => {}
        Err(e) => {
            eprintln!("Error checking daemon status: {}", e);
        }
    }
    
    let pid_file = get_pid_file_path();
    let content = format!("{}:{}:{}:{}", pid, port, no_log, no_wait);
    std::fs::write(pid_file, content)?;
    
    println!("Daemon started successfully in background.");
    println!("PID: {}", pid);
    if no_log {
        println!("Logs: Disabled");
    } else {
        println!("Logs: {}", log_path.display());
    }
    
    Ok(())
}

fn start_daemon(foreground: bool, no_log: bool, no_wait: bool, port: u16) {
    if foreground {
        let current_pid = std::process::id();
        let _ = std::fs::create_dir_all(get_config_dir());
        let content = format!("{}:{}:{}:{}", current_pid, port, no_log, no_wait);
        let _ = std::fs::write(get_pid_file_path(), content);
        
        if !no_log {
            println!("Starting webhook server in foreground on port {}...", port);
        }
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(run_server(no_log, no_wait, port));
    } else {
        // Kill existing daemon if running
        let pid_file = get_pid_file_path();
        if pid_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&pid_file) {
                let parts: Vec<&str> = content.trim().split(':').collect();
                if let Some(pid_str) = parts.first() {
                    if let Ok(pid) = pid_str.parse::<i32>() {
                        if is_process_running(pid) {
                            println!("Stopping old daemon with PID {}...", pid);
                            unsafe {
                                libc::kill(pid, 15);
                            }
                            for _ in 0..30 {
                                std::thread::sleep(std::time::Duration::from_millis(100));
                                if !is_process_running(pid) {
                                    break;
                                }
                            }
                            if is_process_running(pid) {
                                println!("Daemon did not exit. Force killing...");
                                  unsafe {
                                      libc::kill(pid, 9);
                                  }
                            }
                        }
                    }
                }
            }
            let _ = std::fs::remove_file(&pid_file);
        }
        
        if let Err(e) = spawn_background_process(no_log, no_wait, port) {
            eprintln!("Error starting daemon: {}", e);
        }
    }
}

#[derive(Clone)]
struct AppState {
    no_wait: bool,
}

async fn run_server(no_log: bool, no_wait: bool, port: u16) {
    let app = Router::new()
        .route("/webhook/*script_path", any(handle_webhook))
        .with_state(AppState { no_wait });
        
    let addr = format!("0.0.0.0:{}", port);
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            if !no_log {
                eprintln!("Failed to bind to port {}: {}", port, e);
            }
            std::process::exit(1);
        }
    };
    
    if !no_log {
        println!("Listening on http://{}", addr);
    }
    if let Err(e) = axum::serve(listener, app).await {
        if !no_log {
            eprintln!("Server error: {}", e);
        }
    }
}

#[derive(serde::Deserialize)]
struct WebhookParams {
    no_wait: Option<bool>,
    wait: Option<bool>,
}

async fn handle_webhook(
    Path(script_path): Path<String>,
    Query(params): Query<WebhookParams>,
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // 1. Authenticate request
    let config = load_config();
    if config.tokens.is_empty() && config.headers.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "error": "Unauthorized: No tokens or headers configured on server."
            }))
        ).into_response();
    }
    
    let mut is_authorized = false;
    
    // Check Authorization header (Bearer token)
    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                let request_token = auth_str.trim_start_matches("Bearer ").trim();
                if config.tokens.iter().any(|t| t == request_token) {
                    is_authorized = true;
                }
            }
        }
    }
    
    // Check custom headers
    if !is_authorized {
        for (header_name, allowed_tokens) in &config.headers {
            if let Some(header_val) = headers.get(header_name) {
                if let Ok(val_str) = header_val.to_str() {
                    let trimmed_val = val_str.trim();
                    if allowed_tokens.iter().any(|t| t == trimmed_val) {
                        is_authorized = true;
                        break;
                    }
                }
            }
        }
    }
    
    if !is_authorized {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "error": "Unauthorized: Invalid or missing token/header."
            }))
        ).into_response();
    }
    
    // 2. Resolve script path
    let cleaned_script_path = script_path.trim_start_matches('/');
    
    let resolved_path = if cleaned_script_path.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "Bad Request: Script path is empty."
            }))
        ).into_response();
    } else {
        let path = std::path::PathBuf::from(cleaned_script_path);
        if path.exists() {
            path
        } else {
            let abs_path = std::path::PathBuf::from("/").join(cleaned_script_path);
            if abs_path.exists() {
                abs_path
            } else {
                return (
                    StatusCode::NOT_FOUND,
                    axum::Json(serde_json::json!({
                        "error": format!("Script file not found: {}", cleaned_script_path)
                    }))
                ).into_response();
            }
        }
    };
    
    // Determine wait behavior
    let request_no_wait = if let Some(wait) = params.wait {
        !wait
    } else if let Some(no_wait) = params.no_wait {
        no_wait
    } else {
        state.no_wait
    };
    
    // 3. Execute script using bash
    let mut child = match tokio::process::Command::new("bash")
        .arg(&resolved_path)
        .stdin(Stdio::piped())
        .stdout(if request_no_wait { Stdio::null() } else { Stdio::piped() })
        .stderr(if request_no_wait { Stdio::null() } else { Stdio::piped() })
        .spawn() {
            Ok(c) => c,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(serde_json::json!({
                        "error": format!("Failed to execute script: {}", e)
                    }))
                ).into_response();
            }
        };
        
    if request_no_wait {
        tokio::spawn(async move {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(&body).await;
            }
            let _ = child.wait().await;
        });
        
        return (
            StatusCode::OK,
            axum::Json(serde_json::json!({
                "message": "Script started in background"
            }))
        ).into_response();
    }
        
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(&body).await;
    }
    
    let output = match child.wait_with_output().await {
        Ok(o) => o,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({
                    "error": format!("Failed to wait for script completion: {}", e)
                }))
            ).into_response();
        }
    };
    
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let status_code = output.status.code().unwrap_or(-1);
    
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "exit_code": status_code,
            "stdout": stdout,
            "stderr": stderr
        }))
    ).into_response()
}

fn main() {
    let cli = Cli::parse();
    
    // Load config first to resolve port and log settings
    let mut config = load_config();
    let final_port = cli.port.unwrap_or(config.port);
    let final_no_log = cli.no_log || config.no_log;
    let final_no_wait = cli.no_wait || config.no_wait;
    
    match cli.command {
        Commands::Add { token } => handle_add(token),
        Commands::List => handle_list(),
        Commands::Delete { token } => handle_delete(token),
        Commands::AddHeader { header, token } => handle_add_header(header, token),
        Commands::ListHeader => handle_list_header(),
        Commands::DeleteHeader { header, token } => handle_delete_header(header, token),
        Commands::Background => {
            config.port = final_port;
            config.no_log = final_no_log;
            config.no_wait = final_no_wait;
            let _ = save_config(&config);
            start_daemon(false, final_no_log, final_no_wait, final_port);
        }
        Commands::Start { foreground } => {
            config.port = final_port;
            config.no_log = final_no_log;
            config.no_wait = final_no_wait;
            let _ = save_config(&config);
            start_daemon(foreground, final_no_log, final_no_wait, final_port);
        }
        Commands::Stop => stop_daemon(),
        Commands::Status => show_status(),
    }
}
