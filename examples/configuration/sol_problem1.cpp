// A simple solution for problem 1 (sum of array elements)
#include <iostream>
#include <vector>
using namespace std;

int main() {
    int n;
    cin >> n;
    
    vector<int> nums(n);
    for (int i = 0; i < n; i++) {
        cin >> nums[i];
    }
    
    int sum = 0;
    for (int x : nums) {
        sum += x;
    }
    
    cout << sum << endl;
    return 0;
}