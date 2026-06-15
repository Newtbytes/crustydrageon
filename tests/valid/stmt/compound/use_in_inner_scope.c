int main(void)
{
    int x;
    {
        x = 3;
    }
    {
        return x;
    }
}

//$ CHECK STATUS : 3