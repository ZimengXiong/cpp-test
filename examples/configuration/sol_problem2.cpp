// A simple solution for problem 2 (find max element)
#include <iostream>
#include <vector>
#include <algorithm>
using namespace std;

int main() {
    int n;
    cin >> n;
    
    if (n == 0) {
        cout << "0" << endl;
        return 0;
    }
    
    vector<int> nums(n);
    for (int i = 0; i < n; i++) {
        cin >> nums[i];
    }
    
    cout << *max_element(nums.begin(), nums.end()) << endl;
    return 0;
}