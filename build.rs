extern crate gcc;

fn main() {
    gcc::Build::new()
        .file("src/bis_c.c")
        .compile("libbis_c.a");
}
