fn main() {
    println!("cargo:rerun-if-changed=../../schemas/migrations");
}
