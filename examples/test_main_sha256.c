#include <stdint.h>
#include <stdio.h>
#include <stdio.h>
#include <stdlib.h>
#include "gc_stack.h"
#include <time.h>
#include "values.h"
#include <string.h>

extern value body(struct thread_info *);

int main(int argc, char *argv[]) {
  value val;
  struct thread_info* tinfo;

  tinfo = make_tinfo();
  val = body(tinfo);
  printf("Result: ");
  for (int i = 0; i < 32; i++) {
       printf("%02x", ((unsigned char*) val)[i]);
  }
  printf("\n");
  return 0;
}