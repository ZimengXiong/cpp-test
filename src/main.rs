use clap::{Arg, Command};
use notify::{RecommendedWatcher, Watcher, RecursiveMode, EventKind};
use std::process::Command as ProcessCommand;
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::{Duration, SystemTime};
use std::fs;
use colored::*;

fn main() {
    // Parse command-line arguments
    let matches = Command::new("cpp-watcher")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("Watches a C++ file for changes, compiles it, and runs the executable.")
        .arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .value_name("FILE")
                .help("Sets the input C++ file to watch")
                .required(true)
                .value_parser(clap::value_parser!(String)),
        )
        .get_matches();

    // Get the input file path
    let input_file = matches.get_one::<String>("input").unwrap();
    let input_path = Path::new(input_file);

    if !input_path.exists() {
        eprintln!("{}", format!("Error: File '{}' does not exist.", input_file).red());
        return;
    }

    if input_path.extension().and_then(|ext| ext.to_str()) != Some("cpp") {
        eprintln!("{}", format!("Error: Input file must be a .cpp file.").red());
        return;
    }

    println!("{}", format!("Watching file: {}", input_file).green());

    // Set up file watcher
    let (tx, rx) = channel();
    let mut watcher = RecommendedWatcher::new(tx, notify::Config::default()).unwrap();
    watcher.watch(input_path, RecursiveMode::NonRecursive).unwrap();

    // Track the last modification time of the file
    let mut last_modified = get_file_modified_time(input_path);

    loop {
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(event) => {
                if let Ok(event) = event {
                    if let EventKind::Modify(_) = event.kind {
                        let current_modified = get_file_modified_time(input_path);
                        if current_modified > last_modified {
                            last_modified = current_modified;

                            println!(
                                "{}",
                                format!("\nFile changed at {}. Compiling and running...\n", timestamp())
                                    .yellow()
                            );
                            if compile_and_run(input_path) {
                                println!("{}", "\nExecution completed successfully.".green());
                            } else {
                                println!("{}", "\nCompilation or execution failed.".red());
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // Timeout occurred, continue watching
            }
        }
    }
}

/// Compiles the given C++ file and runs the resulting executable.
fn compile_and_run(input_path: &Path) -> bool {
    let output_executable = "output";

    // Compile the C++ file
    let compile_status = ProcessCommand::new("g++")
        .args([
            "-std=c++17",
            "-O2",
            "-lm",
            "-o",
            output_executable,
            input_path.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute g++");

    if !compile_status.success() {
        eprintln!("{}", "Compilation failed.".red());
        return false;
    }

    // Run the compiled executable
    let run_output = ProcessCommand::new("./output")
        .output()
        .expect("Failed to execute the compiled program");

    if !run_output.status.success() {
        eprintln!("{}", "Execution failed.".red());
        return false;
    }

    // Print the output of the compiled program
    println!("\n{}{}", "Program Output:".bold(), "\n-------------------".dimmed());
    println!("{}", String::from_utf8_lossy(&run_output.stdout).trim().blue());
    println!("{}", "-------------------\n".dimmed());

    true
}

/// Returns the last modified time of a file.
fn get_file_modified_time(path: &Path) -> SystemTime {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// Returns the current timestamp as a formatted string.
fn timestamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}