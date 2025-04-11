// Random test case generator
#include <iostream>
#include <random>
using namespace std;

int main() {
    int seed;
    cin >> seed;
    
    mt19937 rng(seed);
    int n = rng() % 20 + 1;  // Array size between 1 and 20
    
    cout << n << endl;
    for (int i = 0; i < n; i++) {
        cout << rng() % 200 - 100 << " ";  // Values between -100 and 99
    }
    cout << endl;
    
    return 0;
}