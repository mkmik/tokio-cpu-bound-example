// Shell 1:
// cargo run
//
// Shell 2:
// time echo foo | socat -t30 - TCP:localhost:1234
//
// Set WORKER_PRIORITY to 0 and you'll see ^^^ being very slow
// Set it to 10 and you'll see a bounded slowdown (from 800ms to 1100ms on my machine).

use std::io::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::{self, Handle};

const NUM_WORKERS: usize = 60;
const WORKER_PRIORITY: i32 = 10;

async fn process_socket(mut socket: TcpStream) -> Result<()> {
    println!("processing socket");

    let (reader, mut writer) = socket.split();

    // prented to parse http request headers, stop on empty line.
    let mut lines = BufReader::new(reader).lines();
    while let Some(line) = lines.next_line().await? {
        if line.len() == 0 {
            break;
        }
    }

    // heavy_stuff(get_count().await);
    writer
        .write_all(
            b"HTTP/1.1 200 OK\n\
                            Connection: close\n\
                            Content-size: 3\n\
                            \n\
                            ok\n",
        )
        .await?;

    Ok(())
}

fn worker() {
    let rt = runtime::Builder::new_current_thread().build().unwrap();
    rt.block_on(aworker());
}

async fn aworker() {
    loop {
        let count = get_count().await;
        heavy_stuff(count);
    }
}

async fn get_count() -> u64 {
    20000000
}

fn heavy_stuff(count: u64) -> u64 {
    let mut acc = 0;
    for _i in 0..count {
        acc += 1;
    }
    acc
}

fn set_current_thread_priority(prio: i32) {
    // on linux setpriority sets the current thread's priority
    // (as opposed to the current process).
    unsafe { libc::setpriority(0, 0, prio) };
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:1234").await?;

    let rt = Handle::current();
    for _i in 0..NUM_WORKERS {
        let res = rt.spawn_blocking(move || {
            set_current_thread_priority(WORKER_PRIORITY);
            worker()
        });

        rt.spawn(res); // force polling the blocking thread
    }

    loop {
        let (socket, _) = listener.accept().await?;
        process_socket(socket).await?;
    }
}
