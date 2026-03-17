#include <stdio.h>
#include <stdlib.h>
#include "gc_stack.h"
#include <time.h>
#include "values.h"


extern value body(struct thread_info *);

int main(int argc, char *argv[]) {
  value val;
  struct thread_info* tinfo;

  tinfo = make_tinfo();
  val = body(tinfo);

  printf("Result: %f\n", Double_val(val));

  return 0;
}