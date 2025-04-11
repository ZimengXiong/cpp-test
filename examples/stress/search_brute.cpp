// Brute force solution using linear search
#include <iostream>
#include <vector>
#include <algorithm>
using namespace std;

// Linear search implementation
int main() {
    int n, target;
    cin >> n >> target;
    
    vector<int> arr(n);
    for (int i = 0; i < n; i++) {
        cin >> arr[i];
    }
    
    // Sort the array (to match the optimized solution)
    sort(arr.begin(), arr.end());
    
    // Linear search through the sorted array
    int result = -1;
    for (int i = 0; i < n; i++) {
        if (arr[i] == target) {
            result = i;
            break;
        }
    }
    
    // Return the found index or -1
    cout << result << endl;
    
    return 0;
}