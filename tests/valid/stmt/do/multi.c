#ifdef SUPPRESS_WARNINGS
#ifndef __clang__
#pragma GCC diagnostic ignored "-Wempty-body"
#endif
#endif

int main(void) {
    int i = 2147483642;
    int j = 0;

    do ; while ((i = i - 5) >= 256);
    do ; while ((j = j + 5) < 256);

    return i - j;
}

//$ CHECK STATUS : 248