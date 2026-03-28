// Bootstring algorithm (RFC 3492) implementation for Punycode
const BASE: u32 = 36;
const TMIN: u32 = 1;
const TMAX: u32 = 26;
const SKEW: u32 = 38;
const DAMP: u32 = 700;
const INITIAL_BIAS: u32 = 72;
const INITIAL_N: u32 = 128;

pub fn encode(input: &str) -> Result<String, &'static str> {
    let mut output = String::new();

    for label in input.split('.') {
        if !output.is_empty() {
            output.push('.');
        }

        if label.chars().all(|c| c.is_ascii()) {
            output.push_str(label);
            continue;
        }

        output.push_str("xn--");
        let mut n = INITIAL_N;
        let mut delta = 0;
        let mut bias = INITIAL_BIAS;

        // Copy basic characters
        let mut h = 0;
        for c in label.chars() {
            if c.is_ascii() {
                output.push(c);
                h += 1;
            }
        }

        let b = h;
        if b > 0 {
            output.push('-');
        }

        let mut m = h;
        let label_len = label.chars().count() as u32;

        while m < label_len {
            let mut min_n = u32::MAX;
            for c in label.chars() {
                let cp = c as u32;
                if cp >= n && cp < min_n {
                    min_n = cp;
                }
            }

            if (min_n - n) > (u32::MAX - delta) / (m + 1) {
                return Err("Punycode overflow");
            }
            delta += (min_n - n) * (m + 1);
            n = min_n;

            for c in label.chars() {
                let cp = c as u32;
                if cp < n {
                    delta += 1;
                } else if cp == n {
                    let mut q = delta;
                    let mut k = BASE;
                    loop {
                        let t = if k <= bias + TMIN {
                            TMIN
                        } else if k >= bias + TMAX {
                            TMAX
                        } else {
                            k - bias
                        };
                        if q < t {
                            break;
                        }
                        let char_val = t + (q - t) % (BASE - t);
                        output.push(value_to_digit(char_val));
                        q = (q - t) / (BASE - t);
                        k += BASE;
                    }
                    output.push(value_to_digit(q));
                    bias = adapt(delta, m + 1, m == b);
                    delta = 0;
                    m += 1;
                }
            }
            delta += 1;
            n += 1;
        }
    }

    Ok(output)
}

fn value_to_digit(v: u32) -> char {
    if v < 26 {
        (v as u8 + b'a') as char
    } else {
        (v as u8 - 26 + b'0') as char
    }
}

fn adapt(mut delta: u32, num_points: u32, first_time: bool) -> u32 {
    delta = if first_time { delta / DAMP } else { delta / 2 };
    delta += delta / num_points;
    let mut k = 0;
    while delta > ((BASE - TMIN) * TMAX) / 2 {
        delta /= BASE - TMIN;
        k += BASE;
    }
    k + (((BASE - TMIN + 1) * delta) / (delta + SKEW))
}
