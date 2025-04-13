# cpp-test

Command-line tool for C++ development, testing, and evaluation for competitive programming

## Table of Contents

- [Features](#features)
- [Installation](#installation)
  - [Using Homebrew](#using-homebrew)
  - [From Source](#from-source)
- [Usage](#usage)
  - [Simple Watch & Run](#simple-watch--run)
  - [Test Case Mode](#test-case-mode)
  - [Stress Testing](#stress-testing)
  - [Auto-detection Modes](#auto-detection-modes)
- [Command Line Options](#command-line-options)
- [File Formats](#file-formats)
  - [Test Case File Format](#test-case-file-cases)
  - [File Structure for Auto Modes](#file-structure-for-auto-modes)
  - [Configuration File (`.cpptestrc`)](#configuration-file-cpptestrc)
- [Examples](#examples)
- [License](#license)

## Features

- **Watch & Run**: Automatically recompile and run your C++ code when changes are detected
- **Test Case Execution**: Run and verify solution outputs against expected results
- **Stress Testing**: Compare your solution against a brute-force correct solution using random inputs
- **Auto-detection**: Automatically find and use appropriately named test and solution files

## Installation

### Using Homebrew

```bash
# Add the custom tap
brew tap zimengxiong/tools

# Install the tool
brew install cpp-test
```

### From Source

```bash
# Clone the repository
git clone https://github.com/zimengxiong/cpp-test.git
cd cpp-test

# Build with Cargo
cargo build --release
```

## Usage

### Simple Watch & Run

Monitors a C++ file for changes, automatically recompiling and running it:

```bash
cpp-test -i solution.cpp
```

### Test Case Mode

Run your solution against test cases and verify outputs:

```bash
cpp-test -i solution.cpp -c tests.cases
```

### Stress Testing

Compare your solution against a brute-force implementation using generated inputs:

```bash
cpp-test -i solution.cpp -g generator.cpp -b brute.cpp
```

### Auto-detection Modes

The tool provides automatic file detection for test case and stress testing modes to streamline usage.

```bash
# Auto-detect test case files and solution
cpp-test -t [optional_pattern]

# Auto-detect stress testing files
cpp-test -s [optional_pattern]
```

**File Detection Hierarchy:**

The auto-detection logic follows a specific hierarchy:

1.  **Configuration File (`.cpptestrc`) Defaults:** If the corresponding `default_*` configuration exists in `.cpptestrc` and the specified files are present, these are used first. (See [Configuration File](#configuration-file-cpptestrc)).
2.  **Pattern Argument (`[optional_pattern]`):** If a pattern is provided, the tool searches for files matching the pattern:
    *   `-t`: Looks for `.cpp` and `.cases` files containing the pattern.
    *   `-s`: Looks for `.cpp` files containing the pattern *and* keywords (`solution`/`sol`, `brute`/`bru`, `generator`/`gen`) to distinguish file types.
3.  **Keyword Matching (No Pattern):**
    *   `-t`:
        *   Checks for `solution.cpp` and `test.cases` specifically.
        *   If not found, looks for `.cpp` files containing `solution` or `sol` and `.cases` files containing `test` (respecting word boundaries like underscores or camelCase).
        *   If still ambiguous, falls back to using the *only* `.cpp` and *only* `.cases` file if exactly one of each exists.
    *   `-s`: Looks for `.cpp` files containing `solution`/`sol`, `brute`/`bru`, and `generator`/`gen` respectively (respecting word boundaries).
4.  **Ambiguity:** If multiple files match at any stage (excluding defaults), the progam exits.

## Command Line Options

| Option | Description |
|--------|-------------|
| `<config_name>` | Run a custom configuration defined in `.cpptestrc` (e.g., `cpp-test myconfig`). |
| `-i, --input <FILE>` | Main C++ solution file to watch or test (overrides auto-detection and config). |
| `-c, --test-cases <FILE>` | Test case file (overrides auto-detection and config). |
| `-g, --generator <FILE>` | Generator C++ file for stress testing (overrides auto-detection and config). |
| `-b, --brute <FILE>` | Brute-force C++ solution for stress testing (overrides auto-detection and config). |
| `-t, --auto-test [PATTERN]` | Auto-find test files. Optional `PATTERN` filters results. |
| `-s, --auto-stress [PATTERN]` | Auto-find stress test files. Optional `PATTERN` filters results. |
| `--version` | Display version information. |
| `--help` | Display help information. |

## File Formats

### Test Case File (`.cases`)

```
@{Test Case Name}
5
1 2 3 4 5
@
15

@{Another Test Case}
3
10 20 30
@
60
```

- `@{Name}` begins a test case with an optional name
- Lines after the name are the input
- The `@` line separates input from expected output
- Lines after the separator are the expected output

### Configuration File (`.cpptestrc`)

An optional `.cpptestrc` file (using YAML format) can be placed in the working directory to define default files and named configurations.

**Structure:**

```yaml
# --- Default Files (Optional) ---
# Used by auto-modes (-t, -s) or simple 'cpp-test' if no arguments are given.
default_watcher: path/to/default_solution.cpp
default_testcase:
  solution: path/to/default_solution.cpp
  testcases: path/to/default_tests.cases
default_stress:
  solution: path/to/default_stress_sol.cpp
  brute: path/to/default_stress_brute.cpp
  generator: path/to/default_stress_gen.cpp

# --- Custom Named Configurations (Optional) ---
# Allows running specific setups by name (e.g., 'cpp-test my_config_name')
my_config_name:
  mode: testcase            # Required: 'watcher', 'testcase', or 'stress'
  solution: path/to/sol.cpp # Required for all modes
  testcases: path/to/tc.cases  # Required only if mode = 'testcase'
  brute: path/to/brute.cpp     # Required only if mode = 'stress'
  generator: path/to/gen.cpp   # Required only if mode = 'stress'

another_config:
  mode: stress
  solution: alt_solution.cpp
  brute: alt_brute.cpp
  generator: alt_generator.cpp
  # ... add more named configurations as needed
```

**Usage:**

*   **Defaults:** If you run `cpp-test -t`, `cpp-test -s`, or just `cpp-test` (for watcher mode) without specifying files, the tool checks for the corresponding `default_*` section in the config file.
*   **Named Configs:** Execute a predefined setup using `cpp-test <config_name>`. This bypasses auto-detection and uses the files specified under that name.

Command-line arguments (`-i`, `-c`, `-b`, `-g`) always take precedence over configuration file settings.

## Examples
See [Examples](/examples/)
* [Basic](examples/basic/README.md)
* [Pattern matching](examples/pattern/README.md)
* [Stress testing](examples/stress/README.md)
* [Configuration file](examples/configuration/README.md)

## License

This project is licensed under the GPLv2 License.
