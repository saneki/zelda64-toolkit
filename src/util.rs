crate const KIBIBYTE: usize = 1024;
crate const MEBIBYTE: usize = KIBIBYTE * 1024;

fn fdivide(length: usize, unit: usize) -> f64 {
    length as f64 / unit as f64
}

crate enum FileSize {
    Int(usize),
    Float(f64),
}

impl FileSize {
    crate fn from(length: usize, unit: usize) -> Self {
        let result = fdivide(length, unit);
        if result.trunc() == result {
            Self::Int(result as usize)
        } else {
            Self::Float(result)
        }
    }
}
