use tokio::{io::AsyncReadExt as _, net::TcpListener, spawn};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:1234").await.unwrap();

    let streams = Vec::new();

    while let Ok((mut stream, addr)) = listener.accept().await {
        let (mut read, mut write) = stream.into_split();

        spawn(async move {
            while let Ok(rec) = receive_receiver.recv().await {
                if rec.1 == addr {
                    continue;
                }
                let len: u64 = serialized.as_bytes().len().try_into().unwrap();
                write.write(&len.to_be_bytes()).await.unwrap();
                write.write(serialized.as_bytes()).await.unwrap();
            }
        });
        spawn(async move {
            loop {
                let mut buf: [u8; 8] = [0; 8];
                read.read_exact(&mut buf).await.unwrap();
                let size = u64::from_be_bytes(buf);
                let mut buf = vec![0; size.try_into().unwrap()];
                read.read_exact(&mut buf).await.unwrap();

                receive_sender.send((deserialized, addr)).unwrap();
            }
        });
    }

    Ok(())
}
