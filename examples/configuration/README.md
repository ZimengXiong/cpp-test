## .cpptestrc

The .cpptestrc file lets you define default files and custom configurations in YAML format:

```yaml
# Default files for each mode
default_watcher: sol_problem1.cpp
default_testcase:
  solution: sol_problem1.cpp
  testcases: test_problem1.cases
default_stress:
  solution: algo1_solution.cpp
  brute: algo1_brute.cpp
  generator: algo1_generator.cpp

# Custom named configurations
problem1:
  mode: testcase
  solution: sol_problem1.cpp
  testcases: test_problem1.cases

problem2:
  mode: testcase
  solution: sol_problem2.cpp
  testcases: test_problem2.cases

algo1:
  mode: stress
  solution: algo1_solution.cpp
  brute: algo1_brute.cpp
  generator: algo1_generator.cpp

quick:
  mode: watcher
  solution: sol_problem1.cpp
```

# Using Default Configurations
When no files are specified, cpp_test will check for defaults in the config file:
```bash
# Run with default watcher file
cpp_test

# Run with default test case files
cpp_test -t

# Run with default stress test files
cpp_test -s
```

# Using Named Configurations
Run a specific configuration directly using its name:

```bash
# Run problem1 test configuration
cpp_test problem1

# Run problem2 test configuration
cpp_test problem2

# Run algo1 stress test configuration
cpp_test algo1

# Run quick watcher mode
cpp_test quick
```

# Configuration Options
## Default Configurations
```yaml
# Default files for each mode
default_watcher:
default_testcase:
  solution:
  testcases:
default_stress:
  solution: 
  brute: 
  generator: 
  ```

## Custom Configurations
Each custom configuration must specify:

```yaml
<name>:
  mode: watcher|testcase|stress
  solution:
  testcases: only if mode=testcase
  brute: only if mode=stress
  generator: only if mode=stress
```