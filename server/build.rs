//   Authors:        Matthew Bartlett              & Arron Harman
//   Major:          (Software Development & Math) & (Software Development)
//   Creation Date:  October  27, 2020
//   Due Date:       November 24, 2020
//   Course:         CSC328
//   Professor Name: Dr. Frye
//   Assignment:     Chat Server
//   Filename:       main.rs
//   Purpose:        Include libcs.a for libc

fn main(){
    println!("cargo:rustc-flags=-L . -l libcs")
}
