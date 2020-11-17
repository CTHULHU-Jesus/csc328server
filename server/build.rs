
fn main(){
    //cargo:rustc-link-search=. cargo:rustc-link-lib=libcs  
    println!("cargo:rustc-flags=-L . -l libcs")
}

