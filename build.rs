pub fn main() {
    println!("cargo::rerun-if-changed=tests/valid");
    println!("cargo::rerun-if-changed=tests/invalid");
    println!("cargo::rerun-if-env-changed=BASE_TEST_DIR");
}
