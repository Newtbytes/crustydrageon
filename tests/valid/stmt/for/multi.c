int main(void) {
    int x = 0;
    int y = 255;

    for (int i = 25; i > 0; i = i - 1) {
        x = x + 1;
    }

    for (int i = 0; i < 16; i = i + 1) {
        y = y - 1;
    }

    return x + y;
}

//$ CHECK STATUS : 8