use std::io::{self, Read, Seek, SeekFrom};

pub fn read_until<T: Read>(stream: &mut T, expected: u8, maximum: usize) -> io::Result<(bool, Vec<u8>)> {
    let mut found = false;
    let mut vec = Vec::new();

    for _ in 0..maximum {
        let mut buf = [0; 1];
        stream.read_exact(&mut buf)?;
        if buf[0] == expected {
            found = true;
            break;
        } else {
            vec.push(buf[0]);
        }
    }

    Ok((found, vec))
}

pub fn get_align<T: Seek>(stream: &mut T, align: u64) -> io::Result<u64> {
    let position = stream.stream_position()?;
    Ok(align - (position % align))
}

pub fn align_forward<T: Seek>(mut stream: &mut T, align: u64) -> io::Result<()> {
    let position = stream.stream_position()?;
    let amount = get_align(&mut stream, align)?;
    stream.seek(SeekFrom::Start(position + amount))?;
    Ok(())
}

pub fn to_signed_hex(n: isize) -> String {
    if n < 0 {
        format!("-{:X}", -n)
    } else {
        format!("{:X}", n)
    }
}
