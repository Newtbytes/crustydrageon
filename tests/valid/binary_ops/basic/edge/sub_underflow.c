int main(void) {
    // FIXME: -2147483648 is parsed as negate +2147483648, which compiler cannot parse into an i32
    /* return -2147483648 - 1; */
    return -2147483647 - 2;
}

//$ CHECK STATUS : 255