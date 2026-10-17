//! Fixed-decimal formatting with JavaScript toFixed rounding for finite inputs.
pub(crate) fn fixed(value: f64, precision: u8) -> String {
    let magnitude = value.abs();
    if magnitude >= 1e21 {
        let scientific = format!("{value:e}");
        let (digits, exponent) = scientific.split_once('e').unwrap();
        return format!("{digits}e+{exponent}");
    }
    let mut result = format!(
        "{magnitude:.precision$}",
        precision = usize::from(precision)
    );
    let bits = magnitude.to_bits();
    let raw_exponent = ((bits >> 52) & 0x7ff) as i32;
    let mantissa = (bits & ((1_u64 << 52) - 1)) | if raw_exponent == 0 { 0 } else { 1_u64 << 52 };
    let exponent = if raw_exponent == 0 {
        -1074
    } else {
        raw_exponent - 1023 - 52
    };
    // JS rounds exact halves away from zero; Rust uses ties-to-even. Scaling by
    // 10^p is halfway iff the binary exponent plus p and trailing zeroes is -1.
    if mantissa != 0
        && exponent + i32::from(precision) + mantissa.trailing_zeros() as i32 == -1
        && (mantissa >> mantissa.trailing_zeros()) % 4 == 1
    {
        let mut bytes = result.into_bytes();
        let mut carry = true;
        for digit in bytes.iter_mut().rev().filter(|d| **d != b'.') {
            if *digit == b'9' {
                *digit = b'0';
            } else {
                *digit += 1;
                carry = false;
                break;
            }
        }
        if carry {
            bytes.insert(0, b'1');
        }
        result = String::from_utf8(bytes).unwrap();
    }
    if value < 0. {
        result.insert(0, '-');
    }
    result
}
