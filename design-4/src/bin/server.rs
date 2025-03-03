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

    let history: Arc<AppendOnlyVec<Vec<u8>>> = Arc::new(AppendOnlyVec::new());

    let (tx, mut rx) = watch::channel(history.len());

    while let Ok((mut stream, addr)) = listener.accept().await {
        let (mut read, mut write) = stream.into_split();

        {
            let mut rx = rx.clone();
            let history = history.clone();
            spawn(async move {
                let mut last_index = 0;
                loop {
                    let current_index = *rx.borrow_and_update();
                    while last_index < current_index {
                        let val = &history[last_index];
                        write.write(&val.len().to_be_bytes()).await.unwrap();
                        write.write(&val).await.unwrap();
                        last_index += 1;
                    }
                    if rx.changed().await.is_err() {
                        break;
                    }
                }
            });
        }
        let history = history.clone();
        spawn(async move {
            loop {
                let mut buf: [u8; 8] = [0; 8];
                read.read_exact(&mut buf).await.unwrap();
                let size = u64::from_be_bytes(buf);
                let mut buf = vec![0; size.try_into().unwrap()];
                read.read_exact(&mut buf).await.unwrap();

                history.push(buf);
            }
        });
    }

    Ok(())
}
