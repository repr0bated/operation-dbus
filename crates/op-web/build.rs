fn main() {
    // Trigger rebuild when UI changes
    println!("cargo:rerun-if-changed=ui/dist");
}
