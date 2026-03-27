fn main() {
    // Expose the compile-time target triple so platform.rs can use env!("TARGET")
    println!(
        "cargo:rustc-env=TARGET={}",
        std::env::var("TARGET").unwrap()
    );
    tauri_build::build()
}
