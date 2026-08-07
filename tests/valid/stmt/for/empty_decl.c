int main(void) {
    int i = 400;
    for (; i != 100; i = i - 100)
        ;
    return 0;
}

//$ CHECK STATUS : 0