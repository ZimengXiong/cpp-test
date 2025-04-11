// Optimized solution using binary search
#include <iostream>
#include <vector>
#include <algorithm>
using namespace std;

// Binary search implementation
int main() {
    int n, target;
    cin >> n >> target;
    
    vector<int> arr(n);
    for (int i = 0; i < n; i++) {
        cin >> arr[i];
    }
    
    // Sort the array (required for binary search)
    sort(arr.begin(), arr.end());
    
    // Binary search algorithm
    int left = 0;
    int right = n - 1;
    int result = -1;
    
    while (left <= right) {
        int mid = left + (right - left) / 2;
        
        if (arr[mid] == target) {
            result = mid;
            break;
        }
        else if (arr[mid] < target) {
            left = mid + 1;
        }
        else {
            right = mid - 1;
        }
    }
    
    // Return the found index or -1
    cout << result << endl;
    
    return 0;
}