#include <iostream>
#include <vector>
#include <algorithm>
using namespace std;

// Binary search to find the middle element of a sorted array
// If array has even length, returns the lower of the two middle elements
int main() {
    int n;
    cin >> n;
    
    if (n == 0) {
        cout << "Empty array" << endl;
        return 0;
    }
    
    vector<int> arr(n);
    for (int i = 0; i < n; i++) {
        cin >> arr[i];
    }
    
    // Sort the array first
    sort(arr.begin(), arr.end());
    
    // Binary search to find the middle element
    int left = 0;
    int right = n - 1;
    int middle = left + (right - left) / 2;
    
    cout << arr[middle] << endl;
    return 0;
}