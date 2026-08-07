int main(void) {
    int a = 0;
    int b = 1;

    while (a < 5)
        a = a + 2;

    while (b < 10)
        b = b + 1;

    return a + b;
}

//$ CHECK STATUS : 16