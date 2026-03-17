fun multiply(num: f64, times: u64) -> f64 {
  let acc = 0.0;
  for i = 0..times {
    acc = acc + num;
  }
  return acc;
}


fun threshold_scale(x: f64, n: u64) -> u64 {
  if x > 0.0 {
    return n * 2;
  }
  if x < 0.0 {
    return n / 2;
  }
  return n;
}
