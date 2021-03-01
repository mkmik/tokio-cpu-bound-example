#![allow(unused_imports)]

use std::io::Result;
use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicUsize, Arc};
use std::{thread, time};
use tokio::io::{copy, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Handle;

const NUM_WORKERS: usize = 60;
const WORKER_PRIORITY: i32 = 10;

async fn process_socket(mut socket: TcpStream) -> Result<()> {
    println!("processing socket");

    let (mut reader, mut writer) = socket.split();

    writer.write_all(b"some heavy computing...").await?;
    heavy_stuff();
    writer.write_all(b"done. echoing\n").await?;

    copy(&mut reader, &mut writer).await?;

    Ok(())
}

fn worker(i: usize, counters: Arc<Vec<AtomicUsize>>) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    rt.block_on(aworker(i, counters));
}

async fn aworker(i: usize, counters: Arc<Vec<AtomicUsize>>) {
    loop {
        heavy_stuff();
        counters[i].fetch_add(1, Ordering::Relaxed);
    }
}

fn heavy_stuff() -> u64 {
    let mut acc = 0;
    for _i in 0..20000000 {
        acc += 1;
    }
    acc
}

fn set_current_thread_priority(prio: i32) {
    // on linux setpriority sets the current thread's priority
    // (as opposed to the current process).
    unsafe { libc::setpriority(0, 0, prio) };
}

async fn reporter(counters: Arc<Vec<AtomicUsize>>) {
    loop {
        let snapshot = counters
            .iter()
            .map(|i| i.load(Ordering::Relaxed))
            .collect::<Vec<_>>();

        let living = snapshot.iter().filter(|i| **i > 0).count();
        if living == counters.len() {
            print!("All workers had a chance to run at least once");
        } else if living < counters.len() / 2 {
            print!("living: ");
            for (i, &n) in snapshot.iter().enumerate() {
                if n > 0 {
                    print!("{}, ", i);
                }
            }
        } else {
            print!("starved: ");
            for (i, &n) in snapshot.iter().enumerate() {
                if n == 0 {
                    print!("{}, ", i);
                }
            }
        }

        println!();
        thread::sleep(time::Duration::from_secs(2));
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:1234").await?;

    let counters = Arc::new(
        std::iter::repeat_with(|| AtomicUsize::new(0))
            .take(NUM_WORKERS)
            .collect::<Vec<_>>(),
    );
    tokio::spawn(reporter(counters.clone()));

    let rt = Handle::current();
    for i in 0..NUM_WORKERS {
        let counters = counters.clone();
        let res = rt.spawn_blocking(move || {
            set_current_thread_priority(WORKER_PRIORITY);
            worker(i, counters)
        });

        rt.spawn(res); // force polling the blocking thread
    }

    loop {
        println!("looping in main");
        let (socket, _) = listener.accept().await?;
        process_socket(socket).await?;
    }
}
