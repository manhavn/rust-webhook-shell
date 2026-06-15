use axum::{
    body::Bytes,
    extract::Path,
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
    about = "Rust CLI tool to run shell scripts via Webhook with Bearer token authentication",
    long_about = "PROGRAM USAGE GUIDE:\n\n\
    * Tokens storage path: ~/.config/webhook-daemon/config.json\n\n\
    1. Authenticated Token Management:\n\
       - Add token:       webhook add <TOKEN>\n\
       - List tokens:     webhook list\n\
       - Delete token:    webhook delete <TOKEN>\n\n\
    2. Launch Webhook Server:\n\
       - Run in Background:  webhook [-p <PORT>] background [-n / --no-log]\n\
       - Run in Foreground:  webhook [-p <PORT>] start --foreground [-n / --no-log]\n\n\
       * Note: The 'background' command will automatically stop the running daemon (if any) before starting a new one.\n\n\
    3. Stop and Status Check:\n\
       - Check status:       webhook status\n\
       - Stop daemon:        webhook stop\n\n\
    4. Call Webhook:\n\
       - Endpoint: POST http://localhost:<PORT>/webhook/{path_to_script.sh}\n\
       - Required Header: Authorization: Bearer <TOKEN>\n\
       - Request body will be piped into the standard input (stdin) of the shell script."
)]
struct Cli {
    /// Port to listen on [default: 9090]
    #[arg(short = 'p', long = "port", default_value = "9090", global = true)]
    port: u16,

    /// Disable writing logs
    #[arg(short = 'n', long = "no-log", global = true)]
    no_log: bool,

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

#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
struct Config {
    tokens: Vec<String>,
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
                if is_process_running(pid) {
                    println!("Status: Running (PID: {})", pid);
                    println!("Listening on port {}", port);
                    println!("Logs: {}", get_log_file_path().display());
                    println!("Tokens storage path: {}", config_path.display());
                    return;
                }
            }
        }
    }
    println!("Status: Stopped (stale PID file found)");
    println!("Tokens storage path: {}", config_path.display());
}

fn spawn_background_process(no_log: bool, port: u16) -> std::io::Result<()> {
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
    let content = format!("{}:{}", pid, port);
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

fn start_daemon(foreground: bool, no_log: bool, port: u16) {
    if foreground {
        let current_pid = std::process::id();
        let _ = std::fs::create_dir_all(get_config_dir());
        let content = format!("{}:{}", current_pid, port);
        let _ = std::fs::write(get_pid_file_path(), content);
        
        if !no_log {
            println!("Starting webhook server in foreground on port {}...", port);
        }
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(run_server(no_log, port));
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
        
        if let Err(e) = spawn_background_process(no_log, port) {
            eprintln!("Error starting daemon: {}", e);
        }
    }
}

async fn run_server(no_log: bool, port: u16) {
    let app = Router::new()
        .route("/webhook/*script_path", any(handle_webhook));
        
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

async fn handle_webhook(
    Path(script_path): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // 1. Authenticate request
    let config = load_config();
    if config.tokens.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "error": "Unauthorized: No tokens configured on server."
            }))
        ).into_response();
    }
    
    let is_authorized = if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                let request_token = auth_str.trim_start_matches("Bearer ").trim();
                config.tokens.iter().any(|t| t == request_token)
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };
    
    if !is_authorized {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "error": "Unauthorized: Invalid or missing bearer token."
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
    
    // 3. Execute script using bash
    let mut child = match tokio::process::Command::new("bash")
        .arg(&resolved_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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
    
    match cli.command {
        Commands::Add { token } => handle_add(token),
        Commands::List => handle_list(),
        Commands::Delete { token } => handle_delete(token),
        Commands::Background => start_daemon(false, cli.no_log, cli.port),
        Commands::Start { foreground } => start_daemon(foreground, cli.no_log, cli.port),
        Commands::Stop => stop_daemon(),
        Commands::Status => show_status(),
    }
}
