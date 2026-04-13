use std::process::Command;
use std::thread;
use std::time::Duration;

const MAX_RETRIES: u32 = 5;
const RETRY_INTERVAL: Duration = Duration::from_secs(10);

struct Output {
    stdout: String,
    stderr: String,
    code: i32,
}

fn run(program: &str, args: &[&str]) -> Result<Output, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("failed to run {program}: {e}"))?;

    Ok(Output {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        code: output.status.code().unwrap_or(1),
    })
}

fn main() {
    // Tell Windows the hardware clock (RTC) stores UTC, not local time
    match run("reg", &["add", r"HKLM\SYSTEM\CurrentControlSet\Control\TimeZoneInformation", "/v", "RealTimeIsUniversal", "/t", "REG_DWORD", "/d", "1", "/f"]) {
        Ok(o) if o.code == 0 => println!("[rtc] RealTimeIsUniversal = 1"),
        Ok(o) => eprintln!("[rtc] failed to set RealTimeIsUniversal (exit {}): {}{}", o.code, o.stdout, o.stderr),
        Err(e) => eprintln!("[rtc] {e}"),
    }

    // Ensure the Windows Time service is running
    match run("net", &["start", "w32time"]) {
        Ok(o) if o.code == 0 => println!("[w32time] {}", o.stdout.trim()),
        Ok(o) if o.code == 2 => println!("[w32time] service already running"),
        Ok(o) => {
            eprintln!("[w32time] failed to start service (exit {}): {}{}", o.code, o.stdout, o.stderr);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("[w32time] {e}");
            std::process::exit(1);
        }
    }

    // Force NTP resync — retry until network is ready
    for attempt in 1..=MAX_RETRIES {
        match run("w32tm", &["/resync", "/force"]) {
            Ok(o) if o.code == 0 => {
                println!("[resync] {}", o.stdout.trim());
                return;
            }
            Ok(o) if attempt < MAX_RETRIES => {
                eprintln!("[resync] attempt {attempt}/{MAX_RETRIES} failed (exit {}): {}{}", o.code, o.stdout, o.stderr);
                thread::sleep(RETRY_INTERVAL);
            }
            Ok(o) => {
                eprintln!("[resync] all {MAX_RETRIES} attempts failed (exit {}): {}{}", o.code, o.stdout, o.stderr);
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("[resync] {e}");
                std::process::exit(1);
            }
        }
    }
}
