# Stress Testing Example

## File Structure
- sort_solution.cpp      # Optimized sorting solution using quicksort
- sort_brute.cpp         # Brute force sorting using bubble sort
- sort_generator.cpp     # Random array generator

## Usage Examples

1. Run stress testing with pattern filtering:
   `cpp-test -s sort`
   
   This will automatically detect and use:
   - sort_solution.cpp (main solution)
   - sort_brute.cpp (slower but correct implementation)
   - sort_generator.cpp (random test generator)

2. Manual specification:
   `cpp-test -i sort_solution.cpp -b sort_brute.cpp -g sort_generator.cpp`

3. Running stress tests with different patterns:
   When you have multiple algorithms to stress test in one directory:
   - `cpp-test -s sort` (tests sorting algorithm)
   - `cpp-test -s search` (would test search algorithm if files existed)

### NOTE: Unlike `-t`, `-s` requires the files to contain `bru[te] | sol[ution] | gen[erator]` to differentiate between the different files.

Stress testing works by seeding the generator program with ascending numbers starting from 1 and comparing the outputs of your solution against the brute force implementation. If any differences are found, the test stops and shows the input that caused the discrepancy, the diff, as well as the seed fed into the generator program.