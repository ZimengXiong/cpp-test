use clap::{Arg, Command}; 
use notify::{RecommendedWatcher, Watcher, RecursiveMode, Config as NotifyConfig};
use std::process::{Command as ProcessCommand, Stdio};
use std::path::{Path, PathBuf};  // Keep only one PathBuf import
use std::sync::mpsc::channel;
use std::time::{Duration, SystemTime};
use std::fs;
use std::io::{self, Write, Read};
use colored::*;
use chrono;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tempfile::{NamedTempFile, TempPath};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ctrlc;
use serde_yaml;
use notify::Error as NotifyError;

// --- Structs and Enums (TestCase, ParseError) remain the same ---
#[derive(Debug)]
struct TestCase {
    name: String,
    input: String,
    expected_output: String,
}
#[derive(Debug)]
enum ParseError {
    Io(io::Error),
    Format(String),
}
impl From<io::Error> for ParseError {
    fn from(err: io::Error) -> Self {
        ParseError::Io(err)
    }
}
// --- End Structs/Enums ---


// --- Constants for executable names ---
// Remove unused constants
// const OUTPUT_WATCH_EXECUTABLE: &str = "./output_watch_run";
// const OUTPUT_TEST_EXECUTABLE: &str = "./output_test_watch";
// --- End Constants ---

// Configuration file structures
#[derive(Debug, Deserialize, Serialize)]
struct TestCaseConfig {
    solution: String,
    testcases: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct StressConfig {
    solution: String,
    brute: String,
    generator: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]  // Add Clone here
struct CustomConfig {
    mode: String,
    #[serde(default)]
    solution: String,
    #[serde(default)]
    testcases: String,
    #[serde(default)]
    brute: String,
    #[serde(default)]
    generator: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct CppTestConfig {
    #[serde(default)]
    default_watcher: Option<String>,
    #[serde(default)]
    default_testcase: Option<TestCaseConfig>,
    #[serde(default)]
    default_stress: Option<StressConfig>,
    #[serde(flatten)]
    custom: HashMap<String, CustomConfig>,
}

fn main() {
    // Check for custom config name first
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && !args[1].starts_with('-') {
        let custom_name = &args[1];
        if let Some(custom_config) = load_config_custom(custom_name) {
            handle_custom_config(custom_name, &custom_config);
            return;
        }
    }

    // Normal command-line parsing
    let matches = Command::new("cpp-watcher")
        .version("0.1.3") // Incremented version
        .author("zxzimeng@gmail.com")
        .about("Watches/Tests C++ files with options for comparing multiple algorithms and testcases")
        .arg( // Input file (not always required now)
              Arg::new("input")
                  .short('i')
                  .long("input")
                  .value_name("MAIN_SRC")
                  .help("Sets the main C++ solution file to watch or test")
                  .required(false)
                  .value_parser(clap::value_parser!(String))
                  .conflicts_with_all(["auto-test", "auto-stress"]),
        )
        // --- Test Case File Mode ---
        .arg(
            Arg::new("test-cases")
                .short('c')
                .long("test-cases")
                .value_name("TEST_FILE")
                .help("Continuously runs tests from file, rerunning on changes")
                .required(false)
                .value_parser(clap::value_parser!(String))
                .conflicts_with_all(["generator", "brute", "auto-test", "auto-stress"]),
        )
        // --- Stress Test Mode Arguments ---
        .arg(
            Arg::new("generator")
                .short('g')
                .long("generator")
                .value_name("GEN_SRC")
                .help("Generator C++ file for stress testing (requires -b)")
                .required(false)
                .value_parser(clap::value_parser!(String))
                .requires("brute")
                .conflicts_with_all(["test-cases", "auto-test", "auto-stress"]),
        )
        .arg(
            Arg::new("brute")
                .short('b')
                .long("brute")
                .value_name("BRUTE_SRC")
                .help("Brute-force/correct C++ solution for stress testing (requires -g)")
                .required(false)
                .value_parser(clap::value_parser!(String))
                .requires("generator")
                .conflicts_with_all(["test-cases", "auto-test", "auto-stress"]),
        )
        // --- New Auto Modes with clearer help text ---
        .arg(
            Arg::new("auto-test")
                .short('t')
                .long("auto-test")
                .value_name("PATTERN")
                .help("Auto-find test files: looks for .cases and solution.cpp files (optional PATTERN)")
                .required(false)
                .num_args(0..=1)  // Makes the value truly optional
                .value_parser(clap::value_parser!(String))
                .conflicts_with_all(["input", "test-cases", "generator", "brute", "auto-stress"]),
        )
        .arg(
            Arg::new("auto-stress")
                .short('s')
                .long("auto-stress")
                .value_name("PATTERN")
                .help("Auto-find stress test files: looks for generator.cpp, brute.cpp, and solution.cpp (optional PATTERN)")
                .required(false)
                .num_args(0..=1)  // Makes the value truly optional
                .value_parser(clap::value_parser!(String))
                .conflicts_with_all(["input", "test-cases", "generator", "brute", "auto-test"]),
        )
        // --- End Args ---
        .get_matches();

    // --- Auto Modes ---
    if matches.contains_id("auto-test") {
        println!("{}", "Mode: Automatic Test Case Detection".cyan().bold());
        let pattern = matches.get_one::<String>("auto-test").map(|s| s.as_str());
        
        if let Some(p) = pattern {
            println!("{}", format!("Using search pattern: '{}'", p).dimmed());
        }
        
        if let Err(e) = auto_test_mode(pattern) {
            eprintln!("{}", e.red());
            std::process::exit(1);
        }
        return;
    }
    
    if matches.contains_id("auto-stress") {
        println!("{}", "Mode: Automatic Stress Testing".cyan().bold());
        let pattern = matches.get_one::<String>("auto-stress").map(|s| s.as_str());
        
        if let Some(p) = pattern {
            println!("{}", format!("Using search pattern: '{}'", p).dimmed());
        }
        
        if let Err(e) = auto_stress_mode(pattern) {
            eprintln!("{}", e.red());
            std::process::exit(1);
        }
        return;
    }
    
    // --- Original Modes (require -i) ---
    let input_file = match matches.get_one::<String>("input") {
        Some(file) => file,
        None => {
            eprintln!("{}", "Error: No input file specified. Use -i option or one of the auto modes (-t or -s).".red());
            std::process::exit(1);
        }
    };
    
    // Rest of the original implementation remains unchanged...
    let input_path = Path::new(input_file).to_path_buf();
    validate_cpp_file(&input_path, "Input");

    // --- Mode Selection Logic ---
    if matches.contains_id("generator") { // Stress test mode takes precedence if flags are present
        println!("{}", "Mode: Stress Testing".cyan());
        // We know 'brute' is also present due to 'requires' constraint
        let gen_file = matches.get_one::<String>("generator").unwrap();
        let brute_file = matches.get_one::<String>("brute").unwrap();

        let gen_path = Path::new(gen_file).to_path_buf();
        let brute_path = Path::new(brute_file).to_path_buf();

        validate_cpp_file(&gen_path, "Generator");
        validate_cpp_file(&brute_path, "Brute-force");

        run_stress_test(&input_path, &gen_path, &brute_path);

    } else if matches.contains_id("test-cases") { // Test case file mode
        println!("{}", "Mode: Continuous File Testing".cyan());
        let test_file = matches.get_one::<String>("test-cases").unwrap();
        let test_path = Path::new(test_file).to_path_buf();
        if !test_path.exists() {
            eprintln!("{}", format!("Error: Test case file '{}' does not exist.", test_path.display()).red());
            std::process::exit(1);
        }
        println!(
            "{}",
            format!(
                "Continuous test mode: Watching {} and {}",
                input_path.display(),
                test_path.display()
            )
                .dimmed()
        );
        watch_and_test(&input_path, &test_path);

    } else { // Default: Simple watch & run mode
        println!("{}", "Mode: Simple Watch & Run".cyan());
        println!("{}", format!("Watching file: {}", input_path.display()).dimmed());
        watch_and_run(&input_path);
    }
}

// --- Helper to Validate C++ Files ---
fn validate_cpp_file(path: &Path, label: &str) {
    if !path.exists() {
        eprintln!("{}", format!("Error: {} file '{}' does not exist.", label, path.display()).red());
        std::process::exit(1);
    }
    if path.extension().and_then(|ext| ext.to_str()) != Some("cpp") {
        eprintln!("{}", format!("Error: {} file '{}' must be a .cpp file.", label, path.display()).red());
        std::process::exit(1);
    }
}

// --- Helper to Create Temporary Executable Files ---
fn create_temp_executable() -> TempPath {
    NamedTempFile::new()
        .expect("Failed to create temporary file")
        .into_temp_path()
}

// --- Helper function to set up file watcher ---
fn setup_watcher(tx: std::sync::mpsc::Sender<Result<notify::Event, NotifyError>>, paths_to_watch: &[&Path]) -> Result<RecommendedWatcher, NotifyError> {
    let mut watcher = RecommendedWatcher::new(tx, NotifyConfig::default())?;
    for path in paths_to_watch {
        if let Err(e) = watcher.watch(path, RecursiveMode::NonRecursive) {
            eprintln!("{} {} {}", "Failed to watch file:".red(), path.display(), e);
            // Return the error to indicate failure
            return Err(e);
        }
    }
    Ok(watcher)
}

// --- Helper function to set up Ctrl+C handler ---
fn setup_ctrlc_handler() -> Arc<AtomicBool> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        if r.load(Ordering::SeqCst) {
            println!("\n{}", "(Ctrl+C detected, stopping...)".yellow());
            r.store(false, Ordering::SeqCst);
        }
    })
    .expect("Error setting Ctrl+C handler");
    running
}

// --- Updated Stress Test Function ---
fn run_stress_test(input_path: &Path, gen_path: &Path, brute_path: &Path) {
    let (tx, rx) = channel();
    // Use helper function for watcher setup
    let _watcher = match setup_watcher(tx, &[input_path, gen_path, brute_path]) {
        Ok(w) => w,
        Err(_) => return, // Error already printed in helper
    };

    let mut last_input_modified = get_file_modified_time(input_path);
    let mut last_gen_modified = get_file_modified_time(gen_path);
    let mut last_brute_modified = get_file_modified_time(brute_path);

    // Use helper function for Ctrl+C handler
    let running = setup_ctrlc_handler();

    println!("{}", "\nStarting stress test loop. Watching for file changes...".green());

    'main_loop: while running.load(Ordering::SeqCst) {
        let main_exec_path = create_temp_executable();
        let gen_exec_path = create_temp_executable();
        let brute_exec_path = create_temp_executable();

        println!("{}", "\nCompiling solutions...".yellow());

        let compiled_main = compile(input_path, &main_exec_path);
        let compiled_gen = compile(gen_path, &gen_exec_path);
        let compiled_brute = compile(brute_path, &brute_exec_path);

        if !compiled_main || !compiled_gen || !compiled_brute {
            eprintln!("{}", "Compilation failed. Cannot start stress test iteration.".red());
        } else {
            println!("{}", "Starting stress test with sequential seeds...".green());
            let mut seed = 1u64; // Starting with seed 1

            'seed_loop: while running.load(Ordering::SeqCst) {
                print!("\rTesting seed: {} ", seed);
                io::stdout().flush().unwrap_or_default();

                let seed_str = seed.to_string();
                let test_case: String;
                let expected_answer: String;
                let actual_answer: String;

                if !running.load(Ordering::SeqCst) {
                    break 'seed_loop;
                }

                // Run generator with current seed
                match run_with_input(&gen_exec_path, &seed_str) {
                    Ok(output) => test_case = output,
                    Err(e) => {
                        eprintln!("\nError running generator (seed {}): {}", seed, e);
                        seed += 1;
                        continue 'seed_loop;
                    }
                }

                if !running.load(Ordering::SeqCst) {
                    break 'seed_loop;
                }

                // Run brute force solution with generated test case
                match run_with_input(&brute_exec_path, &test_case) {
                    Ok(output) => expected_answer = output,
                    Err(e) => {
                        eprintln!("\nError running brute-force (seed {}): {}", seed, e);
                        seed += 1;
                        continue 'seed_loop;
                    }
                }

                if !running.load(Ordering::SeqCst) {
                    break 'seed_loop;
                }

                // Run main solution with generated test case
                match run_with_input(&main_exec_path, &test_case) {
                    Ok(output) => actual_answer = output,
                    Err(e) => {
                        eprintln!("\nError running main solution (seed {}): {}", seed, e);
                        seed += 1;
                        continue 'seed_loop;
                    }
                }

                // Compare outputs
                if expected_answer.trim() != actual_answer.trim() {
                    println!("\n\n{}", "=== MISMATCH FOUND! ===".bright_red().bold());
                    println!("{}", format!("Seed: {}", seed).bold());
                    
                    // Save input to file
                    match save_output_to_file(&test_case, "input") {
                        Ok(input_filepath) => {
                            println!("{} {}", "Input saved to:".italic(), input_filepath.display().to_string().blue().underline());
                        },
                        Err(e) => {
                            eprintln!("{} {}", "Failed to save input:".red(), e);
                        }
                    }
                    
                    // Save brute force output to file
                    match save_output_to_file(&expected_answer, "expected") {
                        Ok(expected_filepath) => {
                            println!("{} {}", "Expected output saved to:".italic(), expected_filepath.display().to_string().blue().underline());
                        },
                        Err(e) => {
                            eprintln!("{} {}", "Failed to save expected output:".red(), e);
                        }
                    }
                    
                    // Save main program output to file
                    match save_output_to_file(&actual_answer, "actual") {
                        Ok(actual_filepath) => {
                            println!("{} {}", "Actual output saved to:".italic(), actual_filepath.display().to_string().blue().underline());
                        },
                        Err(e) => {
                            eprintln!("{} {}", "Failed to save actual output:".red(), e);
                        }
                    }
                    
                    // Display abbreviated info
                    println!("{}", "Generated Input:".cyan().bold());
                    println!("{}", ">>>>".cyan().bold());
                    test_case.trim().lines().take(5).for_each(|line| println!("{}", line.cyan())); // Show first 5 lines
                    if test_case.trim().lines().count() > 5 {
                        println!("{}", "... (see file for complete input)".cyan().dimmed());
                    }
                    println!("{}", ">>>>".cyan().bold());
                    
                    println!("{}", "Brute Force Result (Expected):".green().bold());
                    println!("{}", ">>>>".green().bold());
                    expected_answer.trim().lines().take(5).for_each(|line| println!("{}", line.green())); // Show first 5 lines
                    if expected_answer.trim().lines().count() > 5 {
                        println!("{}", "... (see file for complete output)".green().dimmed());
                    }
                    println!("{}", ">>>>".green().bold());
                    
                    println!("{}", "Program Result (Actual):".red().bold());
                    println!("{}", ">>>>".red().bold());
                    actual_answer.trim().lines().take(5).for_each(|line| println!("{}", line.red())); // Show first 5 lines
                    if actual_answer.trim().lines().count() > 5 {
                        println!("{}", "... (see file for complete output)".red().dimmed());
                    }
                    println!("{}", ">>>>".red().bold());
                    
                    println!("{}", "========================".bright_red().bold());
                    break 'seed_loop;
                }

                // Increment seed for next iteration
                seed += 1;
            }
        }

        println!("{}", "\nWaiting for file changes...".dimmed());
        loop {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(event_result) => {
                    if let Ok(event) = event_result {
                        if event.kind.is_modify() || event.kind.is_create() {
                            let current_input_modified = get_file_modified_time(input_path);
                            let current_gen_modified = get_file_modified_time(gen_path);
                            let current_brute_modified = get_file_modified_time(brute_path);

                            if current_input_modified > last_input_modified
                                || current_gen_modified > last_gen_modified
                                || current_brute_modified > last_brute_modified
                            {
                                last_input_modified = current_input_modified;
                                last_gen_modified = current_gen_modified;
                                last_brute_modified = current_brute_modified;
                                break;
                            }
                        }
                    }
                }
                Err(_) => {
                    if !running.load(Ordering::SeqCst) {
                        break 'main_loop;
                    }
                }
            }
        }
    }

    println!("{}", "\nStress test finished.".yellow());
}

// --- Existing Functions (watch_and_run, watch_and_test, compile, run_executable, run_with_input, parse_test_cases, run_tests, get_file_modified_time, timestamp, print_parse_error) ---
// These functions remain the same as in the previous version.
// Make sure `run_tests` still takes `executable_path` and doesn't compile internally.
// (Include the full code for these functions here if needed, or assume they are present from the previous step)
// --- Function for Simple Watch Mode ---
fn watch_and_run(input_path: &Path) {
    let (tx, rx) = channel();
    // Use helper function for watcher setup
    let _watcher = match setup_watcher(tx, &[input_path]) { // Assign to _watcher as it might not be used directly after setup
        Ok(w) => w,
        Err(_) => {
            std::process::exit(1); // Exit if watcher setup fails critically
        }
    };


    let mut last_modified = get_file_modified_time(input_path);
    let output_executable = create_temp_executable();

    // Use helper function for Ctrl+C handler
    let running = setup_ctrlc_handler();

    // --- Perform Initial Compile and Run ---
    println!("{}", "\nPerforming initial compile and run...".yellow());
    if compile(input_path, &output_executable) {
        run_executable(&output_executable, None);
    } else {
        println!("{}", "Initial compilation failed.".red());
    }
    println!("{}", "\nWaiting for file changes...".dimmed());
    // --- End Initial Compile and Run ---

    loop {
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(event_result) => {
                if let Ok(event) = event_result {
                    if event.kind.is_modify() || event.kind.is_create() {
                        let current_modified = get_file_modified_time(input_path);
                        if current_modified > last_modified {
                            last_modified = current_modified;

                            println!(
                                "{}",
                                format!("\nSource file changed at {}. Recompiling and running...", timestamp())
                                    .yellow()
                            );
                            if compile(input_path, &output_executable) {
                                run_executable(&output_executable, None);
                            } else {
                                println!("{}", "\nCompilation failed.".red());
                            }
                            println!("{}", "\nWaiting for file changes...".dimmed());
                        }
                    }
                } else if let Err(e) = event_result {
                    eprintln!("{}", format!("Watch error: {:?}", e).red());
                }
            }
            Err(_) => {
                if !running.load(Ordering::SeqCst) {
                    break;
                }
                if !input_path.exists() {
                    eprintln!("{}", format!("Error: Watched file '{}' no longer exists. Exiting.", input_path.display()).red());
                    break;
                }
            }
        }
    }

    // Temporary file will be automatically cleaned up when `output_executable` goes out of scope.
}

// --- Function for Continuous Test Mode ---
fn watch_and_test(input_path: &Path, test_path: &Path) {
    let (tx, rx) = channel();
    // Use helper function for watcher setup
    let _watcher = match setup_watcher(tx, &[input_path, test_path]) { // Assign to _watcher
        Ok(w) => w,
        Err(_) => {
            std::process::exit(1); // Exit if watcher setup fails critically
        }
    };


    let mut last_input_modified = get_file_modified_time(input_path);
    let mut last_test_modified = get_file_modified_time(test_path);
    let output_executable = create_temp_executable();

    // Use helper function for Ctrl+C handler
    let running = setup_ctrlc_handler();

    // --- Perform Initial Compile, Parse, and Test Run ---
    println!("{}", "\nPerforming initial test run...".yellow());
    if compile(input_path, &output_executable) {
        match parse_test_cases(test_path) {
            Ok(test_cases) => {
                if !test_cases.is_empty() {
                    let test_succeeded = run_tests(&output_executable, &test_cases);
                    if test_succeeded {
                        println!("{}", "Initial test run passed.".green());
                    } else {
                        println!("{}", "Initial test run failed.".red());
                    }
                } else {
                    println!("{}", "No test cases found in file.".yellow());
                }
            }
            Err(e) => {
                print_parse_error(&e, test_path);
            }
        }
    } else {
        println!("{}", "Initial compilation failed. Cannot run tests.".red());
    }
    println!("{}", "\nWaiting for file changes...".dimmed());
    // --- End Initial Run ---

    loop {
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(event_result) => {
                if let Ok(event) = event_result {
                    if event.kind.is_modify() || event.kind.is_create() {
                        let current_input_modified = get_file_modified_time(input_path);
                        let current_test_modified = get_file_modified_time(test_path);

                        if current_input_modified > last_input_modified
                            || current_test_modified > last_test_modified
                        {
                            last_input_modified = current_input_modified;
                            last_test_modified = current_test_modified;

                            println!(
                                "{}",
                                format!(
                                    "\nChange detected at {}. Recompiling and re-running tests...",
                                    timestamp()
                                )
                                .yellow()
                            );

                            if compile(input_path, &output_executable) {
                                match parse_test_cases(test_path) {
                                    Ok(test_cases) => {
                                        if !test_cases.is_empty() {
                                            run_tests(&output_executable, &test_cases);
                                        } else {
                                            println!("{}", "No test cases found in file.".yellow());
                                        }
                                    }
                                    Err(e) => {
                                        print_parse_error(&e, test_path);
                                    }
                                }
                            } else {
                                println!("{}", "\nCompilation failed. Cannot run tests.".red());
                            }
                            println!("{}", "\nWaiting for file changes...".dimmed());
                        }
                    }
                }
            }
            Err(_) => {
                if !running.load(Ordering::SeqCst) {
                    break;
                }
                if !input_path.exists() || !test_path.exists() {
                    eprintln!("{}", "Error: Watched file no longer exists. Exiting.".red());
                    break;
                }
            }
        }
    }

    // Temporary file will be automatically cleaned up when `output_executable` goes out of scope.
}

// --- Compile Function ---
fn compile(input_path: &Path, output_executable: &Path) -> bool {
    println!("{}", format!("Compiling {} -> {} ...", input_path.display(), output_executable.display()).dimmed());
    let compile_output = ProcessCommand::new("g++")
        .args([
            "-std=c++17", "-Wall", "-Wextra", "-O2", // "-g",
            "-lm", "-o", output_executable.to_str().expect("Output path invalid UTF-8"),
            input_path.to_str().expect("Input path invalid UTF-8"),
        ])
        .output()
        .expect("Failed to execute g++ command");

    if !compile_output.status.success() {
        eprintln!("{}", "-------------------".red());
        eprintln!("{}", "Compilation Failed:".red().bold());
        eprintln!("{}", String::from_utf8_lossy(&compile_output.stderr).trim().red());
        eprintln!("{}", "-------------------".red());
        return false;
    } else if !compile_output.stderr.is_empty() {
        println!("{}", "-------------------".yellow());
        println!("{}", "Compilation Warnings:".yellow().bold());
        println!("{}", String::from_utf8_lossy(&compile_output.stderr).trim().yellow());
        println!("{}", "-------------------".yellow());
    } else { /* Implicit success */ }
    true // Return true only if status is success
}

// --- Function to Run Executable (Simple Watch Mode) ---
fn run_executable(executable_path: &Path, input_data: Option<&str>) -> bool {
    println!("{}", "\nRunning executable...".dimmed());
    let mut command = ProcessCommand::new(executable_path);
    if input_data.is_some() {
        command.stdin(Stdio::piped());
    }
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to spawn {}: {}", executable_path.display(), e);
            return false;
        }
    };

    // --- Write Input to Program (if any) ---
    if let Some(input) = input_data {
        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(input.as_bytes()) {
                eprintln!("Failed to write to stdin: {}", e);
            }
            drop(stdin);
        }
    }

    // --- Capture Output ---
    let run_output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Failed to wait for {}: {}", executable_path.display(), e);
            return false;
        }
    };

    // --- Output Section (with markers on new lines) ---
    let stdout_str = String::from_utf8_lossy(&run_output.stdout);
    println!("{}", "Output:".bold());
    println!("{}", ">>>>".cyan().bold());
    if stdout_str.trim().is_empty() {
        println!("{}", "<No output>".dimmed());
    } else {
        println!("{}", stdout_str.trim().blue());
    }
    println!("{}", ">>>>".cyan().bold());

    // --- Error Output Section (only if errors, with markers on new lines) ---
    let stderr_str = String::from_utf8_lossy(&run_output.stderr);
    if !stderr_str.trim().is_empty() {
        println!("{}", "Error:".yellow().bold());
        println!("{}", ">>>>".yellow().bold());
        eprintln!("{}", stderr_str.trim().yellow());
        println!("{}", ">>>>".yellow().bold());
    }

    // --- Check Execution Status ---
    if !run_output.status.success() {
        eprintln!("\n{}", format!("Execution failed: {}", run_output.status).red());
        return false;
    }
    true
}

// --- Function to Run with Input and Capture Output (Test/Stress Modes) ---
fn run_with_input(executable_path: &Path, input_data: &str) -> Result<String, String> {
    let mut command = ProcessCommand::new(executable_path);
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let mut child = command.spawn().map_err(|e| format!("Failed to spawn '{}': {}", executable_path.display(), e))?;
    
    // Write input data to stdin
    let stdin_handle = child.stdin.take().ok_or_else(|| format!("Failed to open stdin for {}", executable_path.display()))?;
    let input_data_owned = input_data.to_string();
    let stdin_thread = std::thread::spawn(move || {
        let mut stdin = stdin_handle;
        stdin.write_all(input_data_owned.as_bytes())
            .map_err(|e| format!("Failed to write to stdin: {}", e))
    });

    // Capture output
    let mut stdout_output = String::new();
    let mut stderr_output = String::new();
    let mut stdout_handle = child.stdout.take().ok_or_else(|| format!("Failed to open stdout for {}", executable_path.display()))?;
    let mut stderr_handle = child.stderr.take().ok_or_else(|| format!("Failed to open stderr for {}", executable_path.display()))?;

    let stdout_thread = std::thread::spawn(move || {
        stdout_handle.read_to_string(&mut stdout_output).map_err(|e| format!("Failed to read stdout: {}", e))?;
        Ok::<String, String>(stdout_output)
    });
    
    let stderr_thread = std::thread::spawn(move || {
        stderr_handle.read_to_string(&mut stderr_output).map_err(|e| format!("Failed to read stderr: {}", e))?;
        Ok::<String, String>(stderr_output)
    });

    // Wait for completion
    let status = child.wait().map_err(|e| format!("Wait failed: {}", e))?;
    match stdin_thread.join() {
        Ok(Ok(())) => {},
        Ok(Err(e)) => return Err(e),
        Err(_) => return Err("Stdin thread panic".to_string()),
    }
    
    let actual_stdout = match stdout_thread.join() {
        Ok(Ok(out)) => out,
        Ok(Err(e)) => return Err(e),
        Err(_) => return Err("Stdout thread panic".to_string()),
    };
    
    let actual_stderr = match stderr_thread.join() {
        Ok(Ok(err)) => err,
        Ok(Err(e)) => return Err(e),
        Err(_) => return Err("Stderr thread panic".to_string()),
    };

    if !status.success() {
        Err(format!("Execution failed: {}\nStderr: {}", status, actual_stderr.trim()))
    } else if !actual_stderr.trim().is_empty() {
        // Only report stderr if it's non-empty but don't print full content unless needed
        Ok(actual_stdout)
    } else {
        Ok(actual_stdout)
    }
}

// --- Function to Parse Test Cases ---
fn parse_test_cases(test_path: &Path) -> Result<Vec<TestCase>, ParseError> {
    let content = fs::read_to_string(test_path)?;
    let mut test_cases = Vec::new(); let mut lines = content.lines().peekable(); let mut line_number = 0;
    while let Some(line) = lines.next() {
        line_number += 1; let trimmed_line = line.trim();
        if trimmed_line.starts_with("@{") && trimmed_line.ends_with('}') {
            let name = trimmed_line[2..trimmed_line.len() - 1].trim().to_string();
            if name.is_empty() { return Err(ParseError::Format(format!( "Missing test case name line {}", line_number))); }
            let start_line = line_number; let mut input_lines = Vec::new(); let mut expected_output_lines = Vec::new();
            let mut in_input_section = true; let mut found_separator = false;
            while let Some(test_line) = lines.peek() {
                line_number += 1; let trimmed_test_line = test_line.trim();
                if trimmed_test_line == "@" {
                    if !in_input_section { return Err(ParseError::Format(format!( "Unexpected second '@' line {} for test '{}' (started {})", line_number, name, start_line))); }
                    lines.next(); in_input_section = false; found_separator = true;
                } else if trimmed_test_line.starts_with("@{") { break; }
                else { let current_line = lines.next().unwrap(); if in_input_section { input_lines.push(current_line); } else { expected_output_lines.push(current_line); } }
            }
            if !found_separator { return Err(ParseError::Format(format!( "Missing '@' separator for test '{}' (started {})", name, start_line))); }
            test_cases.push(TestCase { name, input: input_lines.join("\n"), expected_output: expected_output_lines.join("\n"), });
        } else if !trimmed_line.is_empty() { return Err(ParseError::Format(format!( "Unexpected content line {}: '{}'", line_number, line))); }
    } Ok(test_cases)
}

// --- Function to Run All Tests (Continuous Test Mode) ---

// --- Utility Functions ---
fn get_file_modified_time(path: &Path) -> SystemTime { fs::metadata(path).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH) }
fn timestamp() -> String { chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string() }
fn print_parse_error(e: &ParseError, test_path: &Path) {
    eprintln!("{}", "-------------------".red()); eprintln!("{}", "Test File Parsing Failed:".red().bold());
    match e { ParseError::Io(err) => eprintln!("Error reading '{}': {}", test_path.display(), err), ParseError::Format(msg) => eprintln!("Invalid format '{}': {}", test_path.display(), msg), }
    eprintln!("{}", "-------------------".red());
}

// Update the save_output_to_file function to use temporary files
fn save_output_to_file(content: &str, prefix: &str) -> Result<PathBuf, io::Error> {
    // Create a temporary file with a pattern
    let temp_file = tempfile::Builder::new()
        .prefix(&format!("{}_", prefix))
        .suffix(".txt")
        .tempfile()?;
    
    // Get the path
    let filepath = temp_file.path().to_owned();
    
    // Write content to file
    fs::write(&filepath, content)?;
    
    // Into_temp_path() prevents the file from being deleted when temp_file goes out of scope
    temp_file.into_temp_path().keep()?;
    
    Ok(filepath)
}

// --- Update the run_tests function for test cases ---
fn run_tests(executable_path: &Path, test_cases: &[TestCase]) -> bool {
    let mut all_passed = true;
    let mut passed_count = 0;
    let mut failed_tests = Vec::new();
    
    if !executable_path.exists() { 
        eprintln!("Cannot run: Executable '{}' not found.", executable_path.display());
        return false;
    }

    println!("{}", "Running tests:".dimmed());
    
    for (index, test_case) in test_cases.iter().enumerate() {
        // Print test case name in grey
        print!("  {} ", format!("[{}] {}", index + 1, test_case.name).dimmed());
        io::stdout().flush().unwrap_or_default();
        
        match run_with_input(executable_path, &test_case.input) {
            Ok(actual_output_raw) => {
                let actual_output = actual_output_raw.replace("\r\n", "\n").trim().to_string();
                let expected_output = test_case.expected_output.replace("\r\n", "\n").trim().to_string();
                
                // Save output to file first (regardless of pass/fail)
                let output_filepath = match save_output_to_file(&actual_output, "output") {
                    Ok(path) => Some(path),
                    Err(e) => {
                        eprintln!("{} {}", "Failed to save output:".red(), e);
                        None
                    }
                };
                
                if actual_output == expected_output {
                    passed_count += 1;
                    println!("{}", "[PASS]".green().bold());
                } else {
                    all_passed = false;
                    println!("{}", "[FAIL]".red().bold());
                    failed_tests.push((
                        index + 1,
                        test_case.name.clone(),
                        test_case.input.clone(),
                        expected_output,
                        actual_output,
                        output_filepath
                    ));
                }
            },
            Err(err_msg) => {
                all_passed = false;
                println!("{}", "[ERROR]".yellow().bold());
                
                // Save error to file
                let error_filepath = match save_output_to_file(&err_msg, "error") {
                    Ok(path) => Some(path),
                    Err(e) => {
                        eprintln!("{} {}", "Failed to save error:".red(), e);
                        None
                    }
                };
                
                failed_tests.push((
                    index + 1,
                    test_case.name.clone(),
                    test_case.input.clone(),
                    "".to_string(),
                    err_msg,
                    error_filepath
                ));
            }
        }
    }
    
    println!("\n{}", "Test Results:".bold());
    println!("  {}/{} tests passed", passed_count, test_cases.len());
    
    // Show detailed failure information with markers
    if !failed_tests.is_empty() {
        println!("\n{}", "Failure Details:".red().bold());
        println!("{}", "======================".red());
        
        for (i, (num, name, input, expected, actual, output_file)) in failed_tests.iter().enumerate() {
            println!("{} {} {}", "FAILED TEST".red().bold(), num, name);
            
            // Create comprehensive output file with all test details
            let mut file_content = String::new();
            file_content.push_str(&format!("FAILED TEST {} {}\n\n", num, name));
            file_content.push_str("Input:\n>>>>>\n");
            file_content.push_str(input);
            file_content.push_str("\n>>>>>\n\n");
            
            // Save input to a file too
            let input_filepath = match save_output_to_file(input, "input") {
                Ok(path) => {
                    println!("\n{} {}", "Input saved to:".italic(), path.display().to_string().blue().underline());
                    Some(path)
                },
                Err(e) => {
                    eprintln!("{} {}", "Failed to save input:".red(), e);
                    None
                }
            };
            
            if expected.is_empty() {
                // Error case
                file_content.push_str("Error:\n>>>>>\n");
                file_content.push_str(actual);
                file_content.push_str("\n>>>>>\n");
                
                // Display info (read from file if available)
                println!("{}", "Input:".cyan().bold());
                println!("{}", ">>>>".cyan().bold());
                if let Some(path) = &input_filepath {
                    if let Ok(content) = fs::read_to_string(path) {
                        content.trim().lines().for_each(|line| println!("{}", line.cyan()));
                    } else {
                        input.trim().lines().for_each(|line| println!("{}", line.cyan()));
                    }
                } else {
                    input.trim().lines().for_each(|line| println!("{}", line.cyan()));
                }
                println!("{}", ">>>>".cyan().bold());
                
                println!("{}", "Error:".yellow().bold());
                println!("{}", ">>>>".yellow().bold());
                if let Some(path) = output_file {
                    if let Ok(content) = fs::read_to_string(path) {
                        content.lines().for_each(|line| println!("{}", line.yellow()));
                    } else {
                        actual.lines().for_each(|line| println!("{}", line.yellow()));
                    }
                } else {
                    actual.lines().for_each(|line| println!("{}", line.yellow()));
                }
                println!("{}", ">>>>".yellow().bold());
            } else {
                // Save expected output to file
                let expected_filepath = match save_output_to_file(expected, "expected") {
                    Ok(path) => {
                        println!("{} {}", "Expected output saved to:".italic(), path.display().to_string().blue().underline());
                        Some(path)
                    },
                    Err(e) => {
                        eprintln!("{} {}", "Failed to save expected output:".red(), e);
                        None
                    }
                };
                
                file_content.push_str("Expected Output:\n>>>>>\n");
                file_content.push_str(expected);
                file_content.push_str("\n>>>>>\n\n");
                
                file_content.push_str("Actual Output:\n>>>>>\n");
                file_content.push_str(actual);
                file_content.push_str("\n>>>>>\n");
                
                // Display info (read from files if available)
                println!("{}", "Input:".cyan().bold());
                println!("{}", ">>>>".cyan().bold());
                if let Some(path) = &input_filepath {
                    if let Ok(content) = fs::read_to_string(path) {
                        content.trim().lines().for_each(|line| println!("{}", line.cyan()));
                    } else {
                        input.trim().lines().for_each(|line| println!("{}", line.cyan()));
                    }
                } else {
                    input.trim().lines().for_each(|line| println!("{}", line.cyan()));
                }
                println!("{}", ">>>>".cyan().bold());
                
                println!("{}", "Expected Output:".green().bold());
                println!("{}", ">>>>".green().bold());
                if let Some(path) = &expected_filepath {
                    if let Ok(content) = fs::read_to_string(path) {
                        content.lines().for_each(|line| println!("{}", line.green()));
                    } else {
                        expected.lines().for_each(|line| println!("{}", line.green()));
                    }
                } else {
                    expected.lines().for_each(|line| println!("{}", line.green()));
                }
                println!("{}", ">>>>".green().bold());
                
                println!("{}", "Actual Output:".red().bold());
                println!("{}", ">>>>".red().bold());
                if let Some(path) = output_file {
                    if let Ok(content) = fs::read_to_string(path) {
                        content.lines().for_each(|line| println!("{}", line.red()));
                    } else {
                        actual.lines().for_each(|line| println!("{}", line.red()));
                    }
                } else {
                    actual.lines().for_each(|line| println!("{}", line.red()));
                }
                println!("{}", ">>>>".red().bold());
                
                // Link to output file
                if let Some(path) = output_file {
                    println!("{} {}", "Actual output saved to:".italic(), path.display().to_string().blue().underline());
                }
            }

            if i < failed_tests.len() - 1 {
                println!("{}", "----------------------".dimmed());
            }
        }
        println!("{}", "======================".red());
    }
    
    all_passed
}

// Helper functions for auto-discovery of files
fn find_files(extension: &str, pattern: Option<&str>) -> Result<Vec<PathBuf>, io::Error> {
    let current_dir = std::env::current_dir()?;
    let mut matching_files = Vec::new();
    
    // Ensure extension doesn't include the dot
    let ext = if extension.starts_with('.') {
        &extension[1..]
    } else {
        extension
    };
    
    for entry in fs::read_dir(current_dir)? {
        if let Ok(entry) = entry {
            let path = entry.path();
            if let Some(file_ext) = path.extension() {
                if file_ext == ext {
                    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    // Check if file matches pattern (if provided)
                    if pattern.is_none() || file_name.contains(pattern.unwrap()) {
                        matching_files.push(path);
                    }
                }
            }
        }
    }
    
    Ok(matching_files)
}


// Auto-detect test files and run tests
fn auto_test_mode(pattern: Option<&str>) -> Result<(), String> {
    println!("{}", "Searching for test files...".dimmed());
    
    // Check config file first if no pattern specified
    if pattern.is_none() {
        if let Some((solution_path, test_path)) = load_config_default_testcase() {
            println!("{}", "Using default testcase configuration:".green());
            println!("{}", format!("Using solution file: {}", solution_path.display()).green());
            println!("{}", format!("Using test case file: {}", test_path.display()).green());
            
            let files = [
                ("Test case file:", test_path.as_path()),
                ("Solution file:", solution_path.as_path())
            ];
            
            if !request_confirmation(&files) {
                println!("{}", "Operation cancelled by user.".yellow());
                return Ok(());
            }
            
            watch_and_test(&solution_path, &test_path);
            return Ok(());
        }
    }
    
    // Special case: If no pattern and solution.cpp + test.cases exist, use them
    if pattern.is_none() {
        // Check for exact filenames
        let solution_path = Path::new("solution.cpp");
        let test_path = Path::new("test.cases");
        
        if solution_path.exists() && test_path.exists() {
            println!("{}", "Found exact matches:".green());
            println!("{}", format!("Using solution file: {}", solution_path.display()).green());
            println!("{}", format!("Using test case file: {}", test_path.display()).green());
            
            let files = [
                ("Test case file:", test_path),
                ("Solution file:", solution_path)
            ];
            
            if !request_confirmation(&files) {
                println!("{}", "Operation cancelled by user.".yellow());
                return Ok(());
            }
            
            watch_and_test(solution_path, test_path);
            return Ok(());
        }
    }

    // Find solution file based on patterns and priority
    let solution_file = find_solution_file(pattern)?;
    
    // Find test case file based on patterns and priority
    let test_file = find_test_case_file(pattern)?;
    
    println!("{}", format!("Using solution file: {}", solution_file.display()).green());
    println!("{}", format!("Using test case file: {}", test_file.display()).green());
    
    // Request confirmation before proceeding
    let files = [
        ("Test case file:", test_file.as_path()),
        ("Solution file:", &solution_file)
    ];
    
    if !request_confirmation(&files) {
        println!("{}", "Operation cancelled by user.".yellow());
        return Ok(());
    }
    
    // Run the watch_and_test function with the found files
    watch_and_test(&solution_file, &test_file);
    Ok(())
}

// Find solution file based on search hierarchy

// Find test case file based on search hierarchy

// Helper function to find cpp file with specific pattern
fn find_cpp_file_with_pattern(pattern: &str) -> Result<PathBuf, String> {
    let cpp_files = find_files("cpp", Some(pattern))
        .map_err(|e| format!("Error scanning directory: {}", e))?;
    
    if cpp_files.is_empty() {
        return Err(format!("No .cpp files containing '{}' found.", pattern));
    }
    
    if cpp_files.len() > 1 {
        let mut file_list = String::new();
        for file in &cpp_files {
            file_list.push_str(&format!("  - {}\n", file.display()));
        }
        return Err(format!("Multiple .cpp files matching '{}' found. Please be more specific:\n{}", pattern, file_list));
    }
    
    Ok(cpp_files[0].clone())
}

// Helper function to find cases file with specific pattern
fn find_cases_file_with_pattern(pattern: &str) -> Result<PathBuf, String> {
    let cases_files = find_files("cases", Some(pattern))
        .map_err(|e| format!("Error scanning directory: {}", e))?;
    
    if cases_files.is_empty() {
        return Err(format!("No .cases files containing '{}' found.", pattern));
    }
    
    if cases_files.len() > 1 {
        let mut file_list = String::new();
        for file in &cases_files {
            file_list.push_str(&format!("  - {}\n", file.display()));
        }
        return Err(format!("Multiple .cases files matching '{}' found. Please be more specific:\n{}", pattern, file_list));
    }
    
    Ok(cases_files[0].clone())
}

// Auto-detect stress test files and run stress tests

// Add a function to request user confirmation
// Change this function to accept a slice of tuples instead of a fixed-size array
fn request_confirmation<P: AsRef<Path>>(files: &[(&str, P)]) -> bool {
    println!("\n{}", "The following files will be used:".bold());
    for (description, path) in files {
        println!("  {} {}", description.cyan(), path.as_ref().display());
    }
    
    print!("\n{} (y/n) ", "Do you want to proceed?".bold());
    io::stdout().flush().unwrap_or_default();
    
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        println!("{}", "Failed to read input, aborting.".red());
        return false;
    }
    
    let input = input.trim().to_lowercase();
    input == "y" || input == "yes"
}

// Helper function to check if a filename matches a target pattern with word boundary rules
fn matches_target_pattern(filename: &str, target_patterns: &[&str]) -> bool {
    let lowercase = filename.to_lowercase();
    
    for &pattern in target_patterns {
        // Check if it's the exact file name (without extension)
        if lowercase == pattern {
            return true;
        }
        
        // Check if it's at the start with word boundary after
        if lowercase.starts_with(pattern) {
            if lowercase.len() == pattern.len() {
                return true;
            }
            
            // Check if followed by underscore or uppercase letter (camelCase)
            let next_char = lowercase.chars().nth(pattern.len());
            if next_char == Some('_') {
                return true;
            }
            
            // Check camelCase (check if original file has uppercase at boundary)
            if filename.chars().nth(pattern.len()).map_or(false, |c| c.is_uppercase()) {
                return true;
            }
        }
        
        // Check if it's at the end with word boundary before
        if lowercase.ends_with(pattern) {
            let prefix_end = lowercase.len() - pattern.len();
            if prefix_end == 0 {
                return true;
            }
            
            // Check if preceded by underscore
            let prev_char = lowercase.chars().nth(prefix_end - 1);
            if prev_char == Some('_') {
                return true;
            }
            
            // Check camelCase (uppercase at start of pattern)
            if pattern.chars().next().is_some() { // Check if pattern is not empty
                if filename.chars().nth(prefix_end).map_or(false, |ch| ch.is_uppercase()) {
                    return true;
                }
            }
        }
        
        // Check if it's in middle with word boundaries on both sides
        if let Some(pos) = lowercase.find(pattern) {
            let before = pos == 0 || lowercase.chars().nth(pos - 1) == Some('_');
            let after_pos = pos + pattern.len();
            let after = after_pos == lowercase.len() || 
                        lowercase.chars().nth(after_pos) == Some('_') ||
                        filename.chars().nth(after_pos).map_or(false, |c| c.is_uppercase());
            
            if before && after {
                return true;
            }
        }
    }
    
    false
}

// Updated function to find specific file with proper boundary checks
fn find_specific_cpp_file(target_type: &str, pattern: Option<&str>) -> Result<Option<PathBuf>, io::Error> {
    let current_dir = std::env::current_dir()?;
    let mut matching_files = Vec::new();
    
    // Define pattern prefixes based on target type
    let target_patterns = match target_type {
        "solution" => vec!["solution", "sol"],
        "brute" => vec!["brute", "bru"],
        "generator" => vec!["generator", "gen"],
        _ => vec![target_type],
    };
    
    // Check minimum length requirement (at least 3 characters)
    for &tp in &target_patterns {
        if tp.len() < 3 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput, 
                format!("Target pattern '{}' must be at least 3 characters", tp)
            ));
        }
    }
    
    for entry in fs::read_dir(current_dir)? {
        if let Ok(entry) = entry {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "cpp" {
                    if let Some(filename) = path.file_stem().and_then(|n| n.to_str()) {
                        // Check if file matches target pattern with proper boundaries
                        if matches_target_pattern(filename, &target_patterns) {
                            // If pattern is provided, also check that
                            if pattern.map_or(true, |p| filename.to_lowercase().contains(&p.to_lowercase())) {
                                matching_files.push(path.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Check for ambiguities
    if matching_files.len() > 1 {
        // Found multiple matches which is ambiguous
        return Ok(None);
    }
    
    Ok(matching_files.into_iter().next())
}

// Helper function to handle ambiguous file errors
fn handle_ambiguous_files(file_type: &str, pattern: Option<&str>, ambiguous_files: Vec<PathBuf>) -> Result<PathBuf, String> {
    if ambiguous_files.is_empty() {
        Err(format!("Could not find a {} file{}",
            file_type,
            pattern.map_or(String::new(), |p| format!(" containing '{}'", p))))
    } else if ambiguous_files.len() > 1 {
        let mut file_list = String::new();
        for file in &ambiguous_files {
            file_list.push_str(&format!("  - {}\n", file.display()));
        }
        Err(format!("Found multiple {} files, which is ambiguous:\n{}", file_type, file_list))
    } else {
        // Exactly one file found
        Ok(ambiguous_files[0].clone())
    }
}

// Updated auto-stress mode with better error handling for ambiguities
fn auto_stress_mode(pattern: Option<&str>) -> Result<(), String> {
    println!("{}", "Searching for stress testing files...".dimmed());
    
    // Check config file first if no pattern specified
    if pattern.is_none() {
        if let Some((solution_path, brute_path, generator_path)) = load_config_default_stress() {
            println!("{}", "Using default stress configuration:".green());
            println!("{}", format!("Using solution file: {}", solution_path.display()).green());
            println!("{}", format!("Using brute force file: {}", brute_path.display()).green());
            println!("{}", format!("Using generator file: {}", generator_path.display()).green());
            
            let files = [
                ("Main solution:", solution_path.as_path()),
                ("Generator file:", generator_path.as_path()),
                ("Brute force file:", brute_path.as_path())
            ];
            
            if !request_confirmation(&files) {
                println!("{}", "Operation cancelled by user.".yellow());
                return Ok(());
            }
            
            run_stress_test(&solution_path, &generator_path, &brute_path);
            return Ok(());
        }
    }
    
    // Find generator.cpp using the ambiguity helper
    let all_gen_files = list_all_matching_files("generator", pattern)
        .map_err(|e| format!("Error listing generator files: {}", e))?;
    let gen_path = handle_ambiguous_files("generator", pattern, all_gen_files)?;
    println!("{}", format!("Found generator file: {}", gen_path.display()).green());

    // Find brute.cpp with similar logic using the ambiguity helper
    let all_brute_files = list_all_matching_files("brute", pattern)
        .map_err(|e| format!("Error listing brute force files: {}", e))?;
    let brute_path = handle_ambiguous_files("brute force", pattern, all_brute_files)?;
    println!("{}", format!("Found brute force file: {}", brute_path.display()).green());

    // Find solution.cpp with the same careful handling using the ambiguity helper
    let all_solution_files = list_all_matching_files("solution", pattern)
        .map_err(|e| format!("Error listing solution files: {}", e))?;
    let input_path = handle_ambiguous_files("solution", pattern, all_solution_files)?;
    println!("{}", format!("Found solution file: {}", input_path.display()).green());

    // Proceed with stress testing
    // ...rest of the code remains unchanged...
    
    // Request confirmation before proceeding
    let files = [
        ("Main solution:", &input_path),
        ("Generator file:", &gen_path),
        ("Brute force file:", &brute_path)
    ];
    
    if !request_confirmation(&files) {
        println!("{}", "Operation cancelled by user.".yellow());
        return Ok(());
    }
    
    // Run the stress test with the found files
    run_stress_test(&input_path, &gen_path, &brute_path);
    Ok(())
}

// Helper function to list all files matching a target for ambiguity reporting
fn list_all_matching_files(target_type: &str, pattern: Option<&str>) -> Result<Vec<PathBuf>, io::Error> {
    let current_dir = std::env::current_dir()?;
    let mut matching_files = Vec::new();
    
    // Define pattern prefixes based on target type
    let target_patterns = match target_type {
        "solution" => vec!["solution", "sol"],
        "brute" => vec!["brute", "bru"],
        "generator" => vec!["generator", "gen"],
        "test" => vec!["test", "t"],
        _ => vec![target_type],
    };
    
    for entry in fs::read_dir(current_dir)? {
        if let Ok(entry) = entry {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let expected_ext = if target_type == "test" { "cases" } else { "cpp" };
                if ext == expected_ext {
                    if let Some(filename) = path.file_stem().and_then(|n| n.to_str()) {
                        if matches_target_pattern(filename, &target_patterns) {
                            if pattern.map_or(true, |p| filename.to_lowercase().contains(&p.to_lowercase())) {
                                matching_files.push(path.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(matching_files)
}

// Update find_solution_file for -t mode to use the same pattern matching logic
fn find_solution_file(pattern: Option<&str>) -> Result<PathBuf, String> {
    // If pattern is specified, just find files matching pattern
    if let Some(p) = pattern {
        return find_cpp_file_with_pattern(p);
    }
    
    // No pattern - find files with solution or sol with proper word boundaries
    let solution_files = list_all_matching_files("solution", None)
        .map_err(|e| format!("Error scanning directory: {}", e))?;
    
    if solution_files.is_empty() {
        // No solution files found, check if there's exactly one .cpp file
        let cpp_files = find_files("cpp", None)
            .map_err(|e| format!("Error scanning directory: {}", e))?;
        
        if cpp_files.is_empty() {
            return Err("No .cpp files found in the current directory.".to_string());
        }
        
        if cpp_files.len() == 1 {
            return Ok(cpp_files[0].clone());
        }
        
        // Multiple .cpp files but none match solution pattern
        let mut file_list = String::new();
        for file in &cpp_files {
            file_list.push_str(&format!("  - {}\n", file.display()));
        }
        return Err(format!("Multiple .cpp files found, but none match 'solution' or 'sol' pattern. Use -i or a pattern:\n{}", file_list));
    }
    
    if solution_files.len() > 1 {
        // Multiple solution files is ambiguous
        let mut file_list = String::new();
        for file in &solution_files {
            file_list.push_str(&format!("  - {}\n", file.display()));
        }
        return Err(format!("Multiple files matching 'solution'/'sol' found. Please specify a pattern or use -i:\n{}", file_list));
    }
    
    Ok(solution_files[0].clone())
}

// Update find_test_case_file for -t mode with similar pattern matching logic
fn find_test_case_file(pattern: Option<&str>) -> Result<PathBuf, String> {
    // If pattern is specified, just find files matching pattern
    if let Some(p) = pattern {
        return find_cases_file_with_pattern(p);
    }
    
    // Check for exact match with test.cases
    let test_path = Path::new("test.cases");
    if test_path.exists() {
        return Ok(test_path.to_path_buf());
    }
    
    // No exact match - find files with test with proper word boundaries
    let test_files = list_all_matching_files("test", None)
        .map_err(|e| format!("Error scanning directory: {}", e))?;
    
    if test_files.is_empty() {
        // No test files found, check if there's exactly one .cases file
        let cases_files = find_files("cases", None)
            .map_err(|e| format!("Error scanning directory: {}", e))?;
        
        if cases_files.is_empty() {
            return Err("No .cases files found in the current directory.".to_string());
        }
        
        if cases_files.len() == 1 {
            return Ok(cases_files[0].clone());
        }
        
        // Multiple .cases files but none match test pattern
        let mut file_list = String::new();
        for file in &cases_files {
            file_list.push_str(&format!("  - {}\n", file.display()));
        }
        return Err(format!("Multiple .cases files found, but none match 'test' pattern. Use -c or a pattern:\n{}", file_list));
    }
    
    if test_files.len() > 1 {
        // Multiple test files is ambiguous
        let mut file_list = String::new();
        for file in &test_files {
            file_list.push_str(&format!("  - {}\n", file.display()));
        }
        return Err(format!("Multiple files matching 'test' found. Please specify a pattern or use -c:\n{}", file_list));
    }
    
    Ok(test_files[0].clone())
}

// Function to load the configuration file
fn load_config() -> Option<CppTestConfig> {
    let config_path = Path::new(".cpptestrc");
    if !config_path.exists() {
        return None;
    }
    
    match fs::read_to_string(config_path) {
        Ok(contents) => {
            match serde_yaml::from_str::<CppTestConfig>(&contents) {
                Ok(config) => Some(config),
                Err(e) => {
                    eprintln!("{}", format!("Error parsing .cpptestrc: {}", e).red());
                    None
                }
            }
        },
        Err(e) => {
            eprintln!("{}", format!("Error reading .cpptestrc: {}", e).red());
            None
        }
    }
}

// Function to get default watcher path from config
fn load_config_default_watcher() -> Option<PathBuf> {
    if let Some(config) = load_config() {
        if let Some(path_str) = config.default_watcher {
            let path = PathBuf::from(path_str);
            if path.exists() {
                return Some(path);
            } else {
                eprintln!("{}", format!("Warning: default_watcher file '{}' not found", path.display()).yellow());
            }
        }
    }
    None
}

// Function to get default testcase config
fn load_config_default_testcase() -> Option<(PathBuf, PathBuf)> {
    if let Some(config) = load_config() {
        if let Some(testcase_config) = config.default_testcase {
            let solution_path = PathBuf::from(&testcase_config.solution);
            let testcases_path = PathBuf::from(&testcase_config.testcases);
            
            if !solution_path.exists() {
                eprintln!("{}", format!("Warning: default_testcase solution file '{}' not found", solution_path.display()).yellow());
                return None;
            }
            
            if !testcases_path.exists() {
                eprintln!("{}", format!("Warning: default_testcase testcases file '{}' not found", testcases_path.display()).yellow());
                return None;
            }
            
            return Some((solution_path, testcases_path));
        }
    }
    None
}

// Function to get default stress config
fn load_config_default_stress() -> Option<(PathBuf, PathBuf, PathBuf)> {
    if let Some(config) = load_config() {
        if let Some(stress_config) = config.default_stress {
            let solution_path = PathBuf::from(&stress_config.solution);
            let brute_path = PathBuf::from(&stress_config.brute);
            let generator_path = PathBuf::from(&stress_config.generator);
            
            if !solution_path.exists() {
                eprintln!("{}", format!("Warning: default_stress solution file '{}' not found", solution_path.display()).yellow());
                return None;
            }
            
            if !brute_path.exists() {
                eprintln!("{}", format!("Warning: default_stress brute file '{}' not found", brute_path.display()).yellow());
                return None;
            }
            
            if !generator_path.exists() {
                eprintln!("{}", format!("Warning: default_stress generator file '{}' not found", generator_path.display()).yellow());
                return None;
            }
            
            return Some((solution_path, brute_path, generator_path));
        }
    }
    None
}

// Function to get custom config by name
fn load_config_custom(name: &str) -> Option<CustomConfig> {
    if let Some(config) = load_config() {
        return config.custom.get(name).cloned();  // This now works
    }
    None
}

// Handle custom named configurations
fn handle_custom_config(name: &str, config: &CustomConfig) {
    match config.mode.as_str() {
        "watcher" => {
            if config.solution.is_empty() {
                eprintln!("{}", format!("Custom config '{}' missing solution field", name).red());
                std::process::exit(1);
            }
            
            let solution_path = PathBuf::from(&config.solution);
            if !solution_path.exists() {
                eprintln!("{}", format!("Solution file '{}' not found", solution_path.display()).red());
                std::process::exit(1);
            }
            
            println!("{}", format!("Using custom '{}' watcher configuration", name).green());
            println!("{}", format!("Watching file: {}", solution_path.display()).dimmed());
            watch_and_run(&solution_path);
        },
        "testcase" => {
            if config.solution.is_empty() || config.testcases.is_empty() {
                eprintln!("{}", format!("Custom config '{}' missing solution or testcases field", name).red());
                std::process::exit(1);
            }
            
            let solution_path = PathBuf::from(&config.solution);
            let testcases_path = PathBuf::from(&config.testcases);
            
            if !solution_path.exists() {
                eprintln!("{}", format!("Solution file '{}' not found", solution_path.display()).red());
                std::process::exit(1);
            }
            
            if !testcases_path.exists() {
                eprintln!("{}", format!("Test case file '{}' not found", testcases_path.display()).red());
                std::process::exit(1);
            }
            
            println!("{}", format!("Using custom '{}' testcase configuration", name).green());
            println!("{}", format!("Using solution file: {}", solution_path.display()).green());
            println!("{}", format!("Using test case file: {}", testcases_path.display()).green());
            watch_and_test(&solution_path, &testcases_path);
        },
        "stress" => {
            if config.solution.is_empty() || config.brute.is_empty() || config.generator.is_empty() {
                eprintln!("{}", format!("Custom config '{}' missing required fields for stress mode", name).red());
                std::process::exit(1);
            }
            
            let solution_path = PathBuf::from(&config.solution);
            let brute_path = PathBuf::from(&config.brute);
            let generator_path = PathBuf::from(&config.generator);
            
            if !solution_path.exists() {
                eprintln!("{}", format!("Solution file '{}' not found", solution_path.display()).red());
                std::process::exit(1);
            }
            
            if !brute_path.exists() {
                eprintln!("{}", format!("Brute force file '{}' not found", brute_path.display()).red());
                std::process::exit(1);
            }
            
            if !generator_path.exists() {
                eprintln!("{}", format!("Generator file '{}' not found", generator_path.display()).red());
                std::process::exit(1);
            }
            
            println!("{}", format!("Using custom '{}' stress configuration", name).green());
            println!("{}", format!("Using solution file: {}", solution_path.display()).green());
            println!("{}", format!("Using brute force file: {}", brute_path.display()).green());
            println!("{}", format!("Using generator file: {}", generator_path.display()).green());
            run_stress_test(&solution_path, &generator_path, &brute_path);
        },
        _ => {
            eprintln!("{}", format!("Invalid mode '{}' in custom config '{}'", config.mode, name).red());
            std::process::exit(1);
        }
    }
}

// Function to autodetect solution file for -i mode
fn autodetect_solution_file() -> Result<Option<PathBuf>, String> {
    // First check for config file
    if let Some(file_path) = load_config_default_watcher() {
        println!("{}", "Using default watcher from config file".green());
        return Ok(Some(file_path));
    }

    let cpp_files = find_files("cpp", None)
        .map_err(|e| format!("Error scanning directory: {}", e))?;
    
    if cpp_files.is_empty() {
        return Ok(None);
    }
    
    // Look for files with .sol, .solution, etc. using the matching pattern
    let sol_files: Vec<PathBuf> = cpp_files.iter()
        .filter(|path| {
            if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                matches_target_pattern(name, &["solution", "sol"])
            } else {
                false
            }
        })
        .cloned()
        .collect();
    
    if sol_files.is_empty() {
        // If there's only one cpp file, use it
        if cpp_files.len() == 1 {
            return Ok(Some(cpp_files[0].clone()));
        }
        return Ok(None);
    }
    
    if sol_files.len() > 1 {
        let mut file_list = String::new();
        for file in &sol_files {
            file_list.push_str(&format!("  - {}\n", file.display()));
        }
        return Err(format!("Multiple solution files found. This is ambiguous:\n{}", file_list));
    }
    
    Ok(Some(sol_files[0].clone()))
}