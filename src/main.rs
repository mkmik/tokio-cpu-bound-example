#![allow(unused_imports)]
#![feature(negative_impls)]

use futures::{future, StreamExt};

struct Foo {
    v: i32,
}

impl !Send for Foo {}
impl !Sync for Foo {}

async fn foo() {
    println!("ok");
}

async fn bar() {
    let f = Foo { v: 0 };
    println!("{}", f.v);
    foo().await;
    println!("{}", f.v);
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    bar().await;
}
