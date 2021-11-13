use std::fs::File;
use std::io::{BufWriter, Write};

fn main() -> std::io::Result<()> {
    let mut f = BufWriter::new(File::create("benches/input/mod.rs")?);

    writeln!(f, "{}", "//! THIS FILE IS AUTO-GENERATED")?;
    writeln!(f, "{}", "#![allow(warnings)]")?;
    writeln!(f)?;

    write!(f, "{}", "pub static SHORT: [u32; 100] = [")?;
    for _ in 0..100 {
        write!(f, "{}, ", rand::random::<u32>())?;
    }
    writeln!(f, "{}", "];")?;
    writeln!(f)?;

    write!(f, "{}", "pub static LONG: [u32; 128 * 1024] = [")?;
    for _ in 0..128 * 1024 {
        write!(f, "{}, ", rand::random::<u32>())?;
    }
    writeln!(f, "{}", "];")?;

    Ok(())
}
