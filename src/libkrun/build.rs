fn main() {
    println!("cargo:rustc-link-search=/usr/local/lib64");
    println!("cargo:rustc-link-lib=krunfw");
}
