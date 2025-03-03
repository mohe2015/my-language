use std::sync::Arc;

use append_only_vec::AppendOnlyVec;
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpListener,
    spawn,
    sync::watch,
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:1234").await.unwrap();

    let history: Arc<AppendOnlyVec<(Vec<u8>, usize)>> = Arc::new(AppendOnlyVec::new());

    let (tx, rx) = watch::channel(history.len());

    let mut id = 0;
    while let Ok((stream, addr)) = listener.accept().await {
        println!("[+] new connection from {addr}");
        let (mut read, mut write) = stream.into_split();

        {
            let mut rx = rx.clone();
            let history = history.clone();
            spawn(async move {
                let mut last_index = 0;
                loop {
                    let current_index = *rx.borrow_and_update();
                    while last_index < current_index {
                        println!("[+] send {last_index} to {addr}");
                        let val = &history[last_index];
                        if val.1 != id {
                            write.write(&val.0.len().to_be_bytes()).await.unwrap();
                            write.write(&val.0).await.unwrap();
                        }
                        last_index += 1;
                    }
                    if rx.changed().await.is_err() {
                        break;
                    }
                }
            });
        }
        let tx = tx.clone();
        let history = history.clone();
        spawn(async move {
            loop {
                let mut buf: [u8; 8] = [0; 8];
                read.read_exact(&mut buf).await.unwrap();
                let size = u64::from_be_bytes(buf);
                let mut buf = vec![0; size.try_into().unwrap()];
                read.read_exact(&mut buf).await.unwrap();

                history.push((buf, id));
                tx.send(history.len()).unwrap();
                println!("[+] received value from {addr}");
            }
        });

        id += 1;
    }

    Ok(())
}
