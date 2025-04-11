// Random test generator for sorting algorithms
#include <iostream>
#include <vector>
#include <random>
using namespace std;

int main() {
    int seed;
    cin >> seed;
    
    // Ensure seed is at least 1
    seed = max(1, seed);
    
    // Initialize random generator with the seed value
    mt19937 rng(seed);
    
    // Generate random array size between 1 and 20
    int n = rng() % 20 + 1;
    
    cout << n << endl;
    for (int i = 0; i < n; i++) {
        cout << rng() % 100 - 30 << " ";  // Values between -30 and 69
    }
    cout << endl;
    
    return 0;
}