# cpp_test

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
brew install cpp_test
```

### From Source

```bash
# Clone the repository
git clone https://github.com/zimengxiong/cpp_test.git
cd cpp_test

# Build with Cargo
cargo build --release
```

## Usage

### Simple Watch & Run

Monitors a C++ file for changes, automatically recompiling and running it:

```bash
cpp_test -i solution.cpp
```

### Test Case Mode

Run your solution against test cases and verify outputs:

```bash
cpp_test -i solution.cpp -c tests.cases
```

### Stress Testing

Compare your solution against a brute-force implementation using generated inputs:

```bash
cpp_test -i solution.cpp -g generator.cpp -b brute.cpp
```

### Auto-detection Modes

```bash
# Auto-detect test cases files and solution
cpp_test -t [optional_pattern]

# Auto-detect stress testing files
cpp_test -s [optional_pattern]
```

## Command Line Options

| Option | Description |
|--------|-------------|
| `-i, --input <FILE>` | Main C++ solution file to watch or test |
| `-c, --test-cases <FILE>` | Test case file with inputs and expected outputs |
| `-g, --generator <FILE>` | Generator C++ file for stress testing (requires -b) |
| `-b, --brute <FILE>` | Brute-force/correct C++ solution for stress testing (requires -g) |
| `-t, --auto-test [PATTERN]` | Auto-find test files (looks for `.cases` and `solution.cpp`) |
| `-s, --auto-stress [PATTERN]` | Auto-find stress test files (looks for `generator.cpp`, `brute.cpp`, and `solution.cpp`) |

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

### File Structure for Auto Modes

#### Auto Test Mode
- Solution file: `solution.cpp` or any `.cpp` file with your pattern
- Test cases: Any `.cases` file

#### Auto Stress Mode
- Main solution: `solution.cpp` or any file containing "solution" and your pattern
- Generator: `generator.cpp` or any file containing "generator" and your pattern
- Brute force: `brute.cpp` or any file containing "brute" and your pattern

## Examples

See the [examples](examples/) folder

## License

This project is licensed under the MIT License.
