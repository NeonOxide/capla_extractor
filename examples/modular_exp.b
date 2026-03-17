fun modular_exp(base: u64, exponent: u64, modulus: u64) -> u64 {
  if modulus == 1 {
    return 0;
  }

  let result: u64 = 1;
  let base_mod: u64 = base % modulus;
  let exp: u64 = exponent;

  while exp > 0 {
    if exp % 2 == 1 {
      result = (result * base_mod) % modulus;
    }
    exp = exp >> 1;
    base_mod = (base_mod * base_mod) % modulus;
  }
  return result;
}

