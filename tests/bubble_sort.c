void *malloc();
int printf();
int atoi();
int rand();
void srand();

void bubble_sort(int *arr, int n) {
  int i;
  int j;
  int temp;

  i = 0;
  while (i < n - 1) {
    j = 0;
    while (j < n - i - 1) {
      if (*(arr + j) < *(arr + (j + 1))) {
        temp = *(arr + j);
        *(arr + j) = *(arr + (j + 1));
        *(arr + (j + 1)) = temp;
      }
      j = j + 1;
    }
    i = i + 1;
  }
}

int main(int argc, char **argv) {
  int n;
  int *arr;
  int i;
  int r;
  int q;
  int m;

  if (argc < 2) {
    printf("Usage: %s <n>\n", *argv);
    return 1;
  }

  n = atoi(*(argv + 1));
  arr = malloc(n * 8);

  srand(0);
  i = 0;
  while (i < n) {
    r = rand();
    q = r / (n * 2);
    m = r - (q * (n * 2));
    *(arr + i) = m;
    i = i + 1;
  }

  printf("Original array: ");
  i = 0;
  while (i < n) {
    printf("%d ", *(arr + i));
    i = i + 1;
  }
  printf("\n");

  bubble_sort(arr, n);

  printf("Sorted array: ");
  i = 0;
  while (i < n) {
    printf("%d ", *(arr + i));
    i = i + 1;
  }
  printf("\n");
}
