# Pattern Filtering Example

## File Structure
- binsearch.cpp
- binsearch.cases
- problem123_solution.cpp
- problem123.cases

## Usage Examples

1. Filter for binary search problem:
   `cpp_test -t bin`
   
   This will automatically detect and use:
   - binsearch.cpp (solution)
   - binsearch.cases (test cases)

2. Filter for problem 123:
   `cpp_test -t 123`
   
   This will automatically detect and use:
   - problem123_solution.cpp (solution)
   - problem123.cases (test cases)