#ifdef SUPPRESS_WARNINGS
#ifndef __clang__
#pragma GCC diagnostic ignored "-Wparentheses"
#endif
#endif
int main(void) {
    return 2 == 2 >= 0;
}

//$ CHECK STATUS : 0