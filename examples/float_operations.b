fun lerp(a: f64, b: f64, t: f64) -> f64 {
  return a + t * (b - a);
}

fun normalize(x: f64, min: f64, max: f64) -> f64 {
  return (x - min) / (max - min);
}

fun harmonic_mean(a b: f64) -> f64 {
  return (2.0 * a * b) / (a + b);
}