use std::rc::Rc;

use qubit_reflect::Reflect;
use qubit_reflect::reflect_impl;

#[derive(Reflect)]
struct Worker;

#[reflect_impl]
impl Worker {
    #[reflect(thread_safe)]
    async fn run() -> u8 {
        let local = Rc::new(1_u8);
        std::future::pending::<()>().await;
        *local
    }
}

fn main() {}
