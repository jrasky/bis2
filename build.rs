extern crate gcc;
extern crate serde_codegen;

use std::env;
use std::path::Path;

fn main() {
    gcc::compile_library("libbis_c.a", &["src/bis_c.c"]);

    let out_dir = env::var_os("OUT_DIR").unwrap();

    let src = Path::new("src/serde_types.in.rs");
    let dst = Path::new(&out_dir).join("serde_types.rs");

    serde_codegen::expand(&src, &dst).expect("Failed to compile serde types");
}
