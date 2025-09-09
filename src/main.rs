use clap::Parser;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    error::Error,
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    sync::{Arc, atomic::AtomicBool},
};
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    runtime::Runtime,
    select,
};

type Result<T> = core::result::Result<T, Box<dyn Error>>;

const IO_BUFFER_SIZE: usize = 8 * 1024;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct UserArgs {
    /// target list
    hosts: Vec<String>,
}

fn connect_to_targets(addresses: Vec<SocketAddr>) -> Result<TcpStream> {
    for address in addresses {
        let stream = TcpStream::connect(address)?;
        return Ok(stream);
    }

    Err("Unable to connect".into())
}

fn parse_target(target: &str) -> Result<Vec<SocketAddr>> {
    let target = match target.contains(":") {
        true => target,
        false => &format!("{target}:22"),
    };

    match target.to_socket_addrs() {
        Ok(v) => Ok(v.collect()),
        Err(_) => Err(format!("Invalid host spec \"{target}\"").into()),
    }
}

async fn io_loop(stream: TcpStream) -> Result<()> {
    stream.set_nonblocking(true).unwrap();

    let mut local_buffer = vec![0u8; IO_BUFFER_SIZE];
    let mut remote_buffer = vec![0u8; IO_BUFFER_SIZE];

    let mut remote_stream = tokio::net::TcpStream::from_std(stream).unwrap();

    let mut stdin_reader = BufReader::new(io::stdin());
    let mut stdout_writer = BufWriter::new(io::stdout());

    loop {
        select! {
            local = stdin_reader.read(&mut local_buffer) => {
                match local {
                    Ok(len) => {
                        if let Err(e) = remote_stream.write_all(&local_buffer[0..len]).await{
                            break Err(format!("error: {e}").into())
                        }
                    }
                    Err(e) => {
                        break Err(format!("error: {e}").into())
                    }
                }
            }
            remote = remote_stream.read(&mut remote_buffer) => {
                match remote {
                    Ok(len) => {
                        if let Err(e) = stdout_writer.write_all(&remote_buffer[0..len]).await{
                            break Err(format!("error: {e}").into())
                        }

                        if let Err(e) = stdout_writer.flush().await{
                            break Err(format!("error: {e}").into())
                        }
                    }
                    Err(e) => {
                        break Err(format!("error: {e}").into())
                    }
                }
            }
        }
    }
}

fn main() -> Result<()> {
    let args = UserArgs::parse();

    if args.hosts.is_empty() {
        return Err("Host Missing".into());
    }

    let connected = Arc::new(AtomicBool::new(false));

    //
    // this'll try all the target at once (par_iter) and whichever
    // connects first will enter the IO loop and the other ones
    // are dropped / disconnected
    //
    args.hosts.par_iter().for_each(|target| {
        if let Ok(addrs) = parse_target(target) {
            if let Ok(stream) = connect_to_targets(addrs) {
                if connected
                    .compare_exchange(
                        false,
                        true,
                        std::sync::atomic::Ordering::Acquire,
                        std::sync::atomic::Ordering::Relaxed,
                    )
                    .is_ok()
                {
                    let rt = Runtime::new().unwrap();
                    rt.block_on(io_loop(stream)).unwrap();
                }
            }
        }
    });

    Ok(())
}
