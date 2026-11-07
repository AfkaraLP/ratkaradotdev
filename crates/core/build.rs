use std::{env, fmt::Write, fs, path::Path};

fn main() {
    let dir = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("posts");
    println!("cargo::rerun-if-changed={}", dir.display());

    let mut posts: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    posts.sort();

    let mut out = String::from("pub const RAW_POSTS: &[(&str, &str)] = &[\n");
    for path in posts {
        let slug = path.file_stem().unwrap().to_string_lossy();
        _ = writeln!(out, "    ({slug:?}, include_str!({:?})),", path.display());
    }
    out.push_str("];\n");

    let dest = Path::new(&env::var("OUT_DIR").unwrap()).join("posts.rs");
    fs::write(dest, out).unwrap();
}
