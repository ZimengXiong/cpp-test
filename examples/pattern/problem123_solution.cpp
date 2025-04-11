#include <iostream>
using namespace std;

// Find maximum element in array
int main() {
    int n;
    cin >> n;
    
    if (n == 0) {
        cout << "0" << endl;
        return 0;
    }
    
    int max_val = -1000000;
    for (int i = 0; i < n; i++) {
        int x;
        cin >> x;
        max_val = max(max_val, x);
    }
    cout << max_val << endl;
    return 0;
}