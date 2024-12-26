use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Obtener el directorio de salida
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("default.json");

    // Copiar el archivo JSON al directorio de salida
    fs::copy("src/data/default.json", &dest_path).unwrap();
    
    // Indicar a Cargo que recompile si el archivo JSON cambia
    println!("cargo:rerun-if-changed=src/data/default.json");
} 