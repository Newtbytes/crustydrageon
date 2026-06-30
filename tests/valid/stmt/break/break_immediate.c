int main(void) {
    int a = 10;
    while ((a = 1))
        break;
    return a;
}

//$ CHECK STATUS : 1