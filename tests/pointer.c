int printf();

int main() {
  int a;
  int* b;
  a = 1;
  b = &a;
  printf("*b = %d (a = %d)\n", *b, a);
  *b = 2;
  printf("a = %d (*b = %d)\n", a, *b);
}
