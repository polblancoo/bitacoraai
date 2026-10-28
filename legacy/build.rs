use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=src/data/default.json");
    
    // Copiar default.json al directorio de salida
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("default.json");
    fs::copy("src/data/default.json", dest_path).unwrap();
} 