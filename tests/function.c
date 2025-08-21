int printf();

int add(int a, int b) { return a + b; }

int main() {
  printf("add(3, 4) = %d\n", add(3, 4));
  printf("add(10, 20) = %d\n", add(10, 20));
  printf("add(0, 0) = %d\n", add(0, 0));
  printf("add(-5, 5) = %d\n", add(-5, 5));
  printf("add(-10, -20) = %d\n", add(-10, -20));
  printf("add(100, 200) = %d\n", add(100, 200));
  printf("add(-100, 100) = %d\n", add(-100, 100));
  printf("add(123456789, 987654321) = %d\n", add(123456789, 987654321));
}
