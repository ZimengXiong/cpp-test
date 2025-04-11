// Random test generator for search algorithms
#include <iostream>
#include <vector>
#include <random>
#include <algorithm>
using namespace std;

int main() {
    int seed;
    cin >> seed;
    
    // Ensure seed is at least 1
    seed = max(1, seed);
    
    // Create array with numbers 1 to seed
    vector<int> arr(seed);
    for (int i = 0; i < seed; i++) {
        arr[i] = i + 1;  // Array contains 1,2,3,...,seed
    }
    
    // Initialize random generator with the seed value
    mt19937 rng(seed);
    
    // Shuffle the array
    shuffle(arr.begin(), arr.end(), rng);
    
    // Decide if target will be in array (80% chance) or not
    int target;
    if (rng() % 5 < 4) { // 80% chance
        // Pick a target value between 1 and seed (which is in the array)
        target = (rng() % seed) + 1;
    } else {
        // Pick a target outside the range 1..seed
        target = seed + 1 + (rng() % 10);  // seed+1 to seed+10
    }
    
    // Output the test case
    // First line: size of array and target value to find
    cout << seed << " " << target << endl;
    
    // Second line: the shuffled array
    for (int i = 0; i < seed; i++) {
        cout << arr[i] << " ";
    }
    cout << endl;
    
    return 0;
}