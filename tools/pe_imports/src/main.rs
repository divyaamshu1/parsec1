use std::{env, fs, collections::BTreeMap};
use goblin::pe::PE;
use anyhow::Result;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let path = if args.len() > 1 { &args[1] } else { "target/release/parsec-cli.exe" };

    let buf = fs::read(path)?;
    let pe = PE::parse(&buf)?;

    println!("Parsed PE imports: {} entries", pe.imports.len());

    let mut by_dll: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for imp in &pe.imports {
        let dll_key = format!("{:?}", imp.dll);
        let func = format!("{:?}", imp.name);
        by_dll.entry(dll_key).or_default().push(func);
    }

    for (dll, funcs) in by_dll {
        println!("Module: {}", dll);
        for f in funcs {
            println!("  - {}", f);
        }
    }

    Ok(())
}
