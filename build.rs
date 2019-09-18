extern crate cc;

fn main() {
    let v8 = env!("V8_PATH");
    println!("cargo:rerun-if-changed=src/ffi.cc");
    println!("cargo:rustc-link-search=native={}/out.gn/libv8/obj", v8);
    println!("cargo:rustc-link-lib=static=v8_monolith");

    cc::Build::new()
        .flag(&format!("-isystem{}/include", v8))
        .flag("-Wno-unused-result")
        .flag("-pthread")
        .flag(&format!("-L{}/out.gn/libv8/obj", v8))
        .flag("-lv8_monolith")
        .flag("-std=c++11")
        .file("src/ffi.cc")
        .cpp(true)
        .compile("libffi.a");
}
