int main(void) {
    int x = 0;
    int y = 100;

    for (int i = 100; i > 0; i = i - 1) {
        for (int j = 0; j < 100; j = j + 1) {
            x = x + 1;
        }
        y = y - 1;
    }

    return x + y;
}

//$ CHECK STATUS : 16