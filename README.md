# capla extractor

A command-line tool that parses Capla files and emits `.c`/`.h` files for use with CertiRocq. Right now it is only able
to extract functions taking and returning either u64s, i64s, [u8;len]s (with some caveats) or f64s (or a mix of them),
which are the primitive types supported by CertiRocq.

## Building

This is a simple Rust project. Just run at the top level of the project.

```bash
cargo build --release
```

This will create an executable in `./target/release/capla_extractor`.

If you want to be able to use it as a normal command you can just move it into your usual bin folder:

```bash
sudo cp "$(pwd)/target/release/capla_extractor" /usr/local/bin/capla_extractor
```

or create a symbolic link so that it is automatically updated when rebuilding:

```bash
sudo ln -s "$(pwd)/target/release/capla_extractor" /usr/local/bin/capla_extractor
```

If you don't want to build an executable, you can substitute `capla_extractor` with
`cargo run -- [OPTIONS] [FILES] ...`. However, this is more cumbersome if one wants to deal with files from other
directories.

## Usage

```
capla_extractor [OPTIONS] [FILES]...
```

By default, if no input files are specified, the tool scans the current directory for all `*.b` files.

## Arguments

| Argument | Default | Description                                                         |
|----------|---------|---------------------------------------------------------------------|
| `FILES`  | `./*.b` | One or more file paths or glob patterns pointing to the capla files |

## Options

| Option                   | Default                 | Description                                                               |
|--------------------------|-------------------------|---------------------------------------------------------------------------|
| `-o, --output-dir <DIR>` | `.` (current directory) | Directory where generated `.c` and `.h` files will be written             |
| `--non-interactive`      | —                       | Skip the interactive TUI. Requires `--prefix` and `--output`              |
| `--prefix <PREFIX>`      | —                       | Prefix to prepend to all exported function names (non-interactive only)   |
| `--output <STEM>`        | —                       | Output file stem for the generated `.c`/`.h` files (non-interactive only) |
| `-h, --help`             | —                       | Print help                                                                |
| `-V, --version`          | —                       | Print version                                                             |

## Modes

### Interactive (default)

Running the tool without `--non-interactive` launches a TUI that lets you browse the parsed
function signatures and configure export names before generating output files. Export names need to be different from
the original ones so that they don't clash with one another.

```bash
# Scan all *.b files in the current directory
capla_extractor

# Specify files explicitly
capla_extractor modular_exp.b float_incr.b

# Use a glob pattern and write output to a custom directory
capla_extractor "src/*.b" --output-dir out/
```

### Non-interactive

Pass `--non-interactive` together with `--prefix` and `--output` to generate files without
any user interaction. This is useful for scripting. However, right now, unlike the interactive version, there is no way
to give individual names to different functions.

```bash
capla_extractor modular_exp.b float_incr.b \
  --non-interactive \
  --prefix certirocq_ \
  --output modular_exp \
  --output-dir generated/
```

This produces `generated/modular_exp.h` and `generated/modular_exp.c`, with every exported
function name prefixed by `certirocq_`.

## Output

For a given `--output <STEM>`, the tool writes two files:

- `<STEM>.h` — header file with function declarations
- `<STEM>.c` — source file with function definitions

## Workflow examples

In order to show how to use the tool, one can follow these examples. All of the required files can be found in the
examples folder. `extraction.v` contains the required Rocq code for both examples. Keep in mind that the generated code
requires the runtime folder from CertiRocq in order to run. If one wants to simply test the examples one can execute
this inside the examples folder

```bash
cp -r path/to/certirocq/runtime ./runtime/
``` 

### Modular arithmetic example

In order to use Capla code when using CertiRocq the process is as follows. For example, if we wanted to use the
modular_exp function from the examples. (execute all of this inside the examples folder):

1. Compile the function using your Capla compiler. Eg:

```bash
capla -c modular_exp.b
``` 

This will generate a `modular_exp.o` file.

2. Use the tool to generate the `.c` and `.h` files. For example, this command will generate a `modular_exp.c` and
   `modular_exp.h` containing the required code for calling modular_exp:

```bash
capla_extractor modular_exp.b \
--non-interactive \
--prefix certirocq_ \
--output modular_exp
```

The contents will be respectively for the `.c` and `.h` files:

```C 
/* Auto-generated by capla_extractor. Do not edit unless necessary. */
#include <stdint.h>
#include "values.h"
extern value mk_float(struct thread_info *tinfo, value my_float);
extern value prim_string_make(struct thread_info *tinfo, value bytes, value d);

extern uint64_t modular_exp(uint64_t base, uint64_t exponent, uint64_t modulus);

value certirocq_modular_exp(value base, value exponent, value modulus) {
	return Val_long(modular_exp(Unsigned_long_val(base), Unsigned_long_val(exponent), Unsigned_long_val(modulus)));
}
```

```C
/* Auto-generated by capla_extractor. Do not edit unless necessary. */
#ifndef MODULAR_EXP_H
#define MODULAR_EXP_H

#include <stdint.h>
#include "values.h"

/**
 * @brief certirocq_modular_exp
 * @param base (native Rocq uint63)
 * @param exponent (native Rocq uint63)
 * @param modulus (native Rocq uint63)
 * @return native Rocq uint63
 */
value certirocq_modular_exp(value base, value exponent, value modulus);

#endif /* MODULAR_EXP_H */
```

3. In order to use the function in Rocq code, create an axiom using either PrimInt63.int or PrimFloat.float:

```Rocq
Axiom modular_exp : forall(base exp modulus: PrimInt63.int), PrimInt63.int.
```

4. Choose your value to extract. We will use the following as an example:

```Rocq
Definition res := modular_exp 123 456 789.
```

5. Register our axiom with the corresponding created c function and header file:

```Rocq
CertiRocq Register [
  modular_exp  => "certirocq_modular_exp"
] Include ["modular_exp.h"].
```

6. Extract the value:

```Rocq
CertiRocq Compile -O 1 -ext "_my_mod_exp" res.
```

7. Create a suitable c file to link with, we will call it `test_main_mod.c`:

```C
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

  printf("Result: %d\n", Long_val(val));

  return 0;
}
```

8. Compile and link everything together. This command can vary a bit depending on what you specifically need, but for
   this example a command like the following is enough:

```bash
gcc -o  my_modular_exp  -w  -fomit-frame-pointer -I$runtime test_main_mod.c $runtime/prim_int63.o  $runtime/gc_stack.c [name_of_extracted_file.c] [capla_compiled_file.o] modular_exp.c
```

If all instructions have been followed exactly and runtime is a folder inside the examples folder, the exact command
should be:

```bash
gcc -o  my_modular_exp  -w  -fomit-frame-pointer -I./runtime test_main_mod.c ./runtime/prim_int63.o  ./runtime/gc_stack.c extraction.res_my_mod_exp.c modular_exp.o modular_exp.c
```

If one wants to simply test the examples one can execute this inside the examples folder

```bash
cp -r path/to/certirocq/runtime ./runtime/
```

9. Execute your program

```bash
./my_modular_exp
```

You should see the result 699.

### Float example

`f64` can be used but require some extra considerations. Let's export a simple increasing function as an example. This
function is the same as the one in the example `floats.b` file:

```capla
fun float_incr(x: f64) -> f64 {
  return x + 1.0;
}
```

1. Compile the function using your Capla compiler. Eg:

```bash
capla -c float_incr.b
``` 

This will generate a `float_incr.o` file.

2. Use the tool to generate the `.c` and `.h` files. For example, this command will generate a `float_incr.c` and
   `float_incr.h` containing the required code for calling float_incr:

```bash
capla_extractor float_incr.b \
--non-interactive \
--prefix certirocq_ \
--output float_incr
```

The contents will be respectively for the `.c` and `.h` files:

```C 
/* Auto-generated by capla_extractor. Do not edit unless necessary. */
#include <stdint.h>
#include "values.h"
extern value mk_float(struct thread_info *tinfo, value my_float);
extern value prim_string_make(struct thread_info *tinfo, value bytes, value d);

extern double float_incr(double x);

value certirocq_float_incr(struct thread_info *tinfo, value x) {
	return mk_float(tinfo, float_incr(Double_val(x)));
}
```

```C
/* Auto-generated by capla_extractor. Do not edit unless necessary. */
#ifndef FLOAT_INCR_H
#define FLOAT_INCR_H

#include <stdint.h>
#include "values.h"

/**
 * @brief certirocq_float_incr
 * @param x (native Rocq float)
 * @return native Rocq float
 */
value certirocq_float_incr(struct thread_info *tinfo, value x);

#endif /* FLOAT_INCR_H */
```

3. Declare the axiom in Rocq code:

```Rocq
Axiom float_incr : forall(fl: PrimFloat.float), PrimFloat.float.
```

4. Choose a value to extract:

```Rocq
Definition float_res: PrimFloat.float := float_incr(2.0).
```

5. Register our value. Note that we require the with tinfo addition, as our function returns a float and floats require
   memory in order to be allocated. This must only be done when floats are returned from the function, in any other
   case (including taking floats as arguments), this should not be done.

```Rocq
CertiRocq Register [
  float_incr  => "certirocq_float_incr" with tinfo
] Include ["float_incr.h"].
```

6. Extract the value:

```Rocq
CertiRocq Compile -O 1 -ext "_my_float_incr" float_res.
```

7. Create a suitable c file to link with, we will call it `test_main_float.c`:

```C
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
```

8. Compile and link everything together. Compared to the standard one, it needs the addition of `prim_float.o` and also
   the `-lm` flag:

```bash
gcc -o  my_float_incr  -w  -fomit-frame-pointer -I./runtime test_main_float.c ./runtime/prim_int63.o ./runtime/prim_floats.o ./runtime/gc_stack.c [name_of_extracted_file.c] float_incr.o float_incr.c -lm 
```

If all instructions have been followed exactly and runtime is a folder inside the examples folder, the exact command
should be:

```bash
gcc -o  my_float_incr  -w  -fomit-frame-pointer -I./runtime test_main_float.c ./runtime/prim_int63.o ./runtime/prim_floats.o  ./runtime/gc_stack.c extraction.my_float_incr.c float_incr.o float_incr.c -lm 
```

If one wants to simply test the examples one can execute this inside the examples folder

```bash
cp -r path/to/certirocq/runtime ./runtime/
```

9. Execute your program

```bash
./my_float_incr
```

You should obtain a result of 3.0.

### String example

`[u8;len]` can be used but require some extra considerations. Let's export a function that calculates the sha256 of a
string as an example. Strings are quite different from the previous examples given that they need to fulfill some
additional conditions:

- The length of every input array must be taken as an argument.
- If the output of the Rocq function is an string, it must be represented by a mutable argument inside of Capla. This is
  because Capla does not support returning arrays.
- Returned strings from Capla must have either a fixed size or their length must be one of the other arguments.

This function is the same as the one in the example `sha256.b` file:

```capla
fun sha256(len: u64, input: [u8; len], output: mut [u8; 32]) {
  // Sha256 algorithm
  ...
}
```

1. Compile the function using your Capla compiler. Eg:

```bash
capla -c sha256.b
``` 

This will generate a `sha256.o` file.

2. Use the tool to generate the `.c` and `.h` files. For example, this command will generate a `float_incr.c` and
   `sha256.h` containing the required code for calling sha256:

```bash
capla_extractor sha256.b \
--non-interactive \
--prefix certirocq_ \
--output float_incr
```

The contents will be respectively for the `.c` and `.h` files:

```C 
/* Auto-generated by capla_extractor. Do not edit unless necessary. */
#include <stdint.h>
#include "values.h"
extern value mk_float(struct thread_info *tinfo, value my_float);
extern value prim_string_make(struct thread_info *tinfo, value bytes, value d);

extern void sha256(uint64_t len, const char* input, char* restrict output);

value certirocq_sha256(struct thread_info *tinfo, value input) {
	uint64_t len = prim_strlen(input);
	unsigned char* output = prim_string_make(tinfo, 0, 32);
	return (sha256(len, ((char*) input), output), output);
}
```

```C
/* Auto-generated by capla_extractor. Do not edit unless necessary. */
#ifndef SHA256_H
#define SHA256_H

#include <stdint.h>
#include "values.h"

/**
 * @brief certirocq_sha256
 * @param input (native Rocq string)
 * @return native Rocq string of length 32
 */
value certirocq_sha256(struct thread_info *tinfo, value input);

#endif /* SHA256_H */
```

3. Declare the axiom in Rocq code:

```Rocq
Axiom sha256 : (PrimString.string) -> (PrimString.string).
```

4. Choose a value to extract:

```Rocq
Definition myPrimString: PrimString.string := "Hello World".
Definition sha_res := sha256 myPrimString.
```

5. Register our value. Note that we require the with tinfo addition, as our function returns a string and strings
   require
   memory in order to be allocated. This must only be done when strings are returned from the function, in any other
   case (including taking strings as arguments), this should not be done.

```Rocq
CertiRocq Register [
  sha256  => "certirocq_sha256" with tinfo
] Include ["sha256.h"].
```

6. Extract the value:

```Rocq
CertiRocq Compile -O 1 -ext "_my_sha256" sha_res.
```

7. Create a suitable c file to link with, we will call it `test_main_sha256.c`:

```C
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
```

8. Compile and link everything together. Compared to the standard one, it needs the addition of `prim_string.c`:

```bash
gcc -o  my_sha256  -w  -fomit-frame-pointer -I./runtime test_main_sha256.c ./runtime/prim_int63.o ./runtime/prim_string.c  ./runtime/gc_stack.c sha256.o sha256.c [name of extracted file].c 
```

If all instructions have been followed exactly and runtime is a folder inside the examples folder, the exact command
should be:

```bash
gcc -o  my_sha256  -w  -fomit-frame-pointer -I./runtime test_main_sha256.c ./runtime/prim_int63.o ./runtime/prim_string.c  ./runtime/gc_stack.c sha256.o sha256.c extraction.sha_res_my_sha256.c 
```

If one wants to simply test the examples one can execute this inside the examples folder

```bash
cp -r path/to/certirocq/runtime ./runtime/
```

9. Execute your program

```bash
./my_sha256
```

You should obtain a result of a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e.

## Notes

- If you do remove the extern definition of `mk_float` from the generated files and are not careful when linking the
  files, you may encounter an issue where the C compiler assumes the function returns a 32 bit integer instead of a
  `value` and then truncates the result of `mk_float` and re-extend it again, returning a nonsensical pointer and
  causing a SIGSEV when de-referenced. So, unless you are really careful, do not do it.
- If you get an error similar to `undefined reference to sqrt` when dealing with floats, the issue is that most surely
  that you have forgotten to add the `-lm` flag or just moved it somewhere where it is not valid.
- When using [name_of_extracted_file.c], it means the name of the file generated by CertiRocq Compile.
- In all of this examples, runtime is the runtime folder contained in the CertiRocq repository.
- If no `.b` files are found, the tool exits with an error and prints usage hints.
- Functions that return `f64` need to allocate memory in order to create a CertiRocq value, so when extracted the first
  argument to the function is the `tinfo` argument.
- If no function signatures are detected in the provided files (non-interactive mode), the tool exits with an error.
- The output directory is created automatically if it does not exist.
- Input file lists are sorted and deduplicated before processing.
- The tool does not check whether function names are repeated or not, so the user needs to take this into account when
  using it.
