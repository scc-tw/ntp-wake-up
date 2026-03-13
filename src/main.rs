use std::process::Command;
use std::thread;
use std::time::Duration;

const MAX_RETRIES: u32 = 5;
const RETRY_INTERVAL: Duration = Duration::from_secs(10);

fn run(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("failed to run {program}: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        Ok(stdout.into_owned())
    } else {
        Err(format!("{stdout}{stderr}").trim().to_string())
    }
}

fn main() {
    // Ensure the Windows Time service is running
    match run("net", &["start", "w32time"]) {
        Ok(msg) => println!("[w32time] {}", msg.trim()),
        Err(e) if e.contains("already been started") => {
            println!("[w32time] service already running");
        }
        Err(e) => {
            eprintln!("[w32time] failed to start service: {e}");
            std::process::exit(1);
        }
    }

    // Force NTP resync — retry until network is ready
    for attempt in 1..=MAX_RETRIES {
        match run("w32tm", &["/resync", "/force"]) {
            Ok(msg) => {
                println!("[resync] {}", msg.trim());
                return;
            }
            Err(e) if attempt < MAX_RETRIES => {
                eprintln!("[resync] attempt {attempt}/{MAX_RETRIES} failed: {e}");
                thread::sleep(RETRY_INTERVAL);
            }
            Err(e) => {
                eprintln!("[resync] all {MAX_RETRIES} attempts failed: {e}");
                std::process::exit(1);
            }
        }
    }
}
