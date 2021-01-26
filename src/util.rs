pub fn to_signed_hex(n: isize) -> String {
    if n < 0 {
        format!("-{:X}", -n)
    } else {
        format!("{:X}", n)
    }
}
