use clap::Parser;
use nix::sys::socket::{
    self, AddressFamily, ControlMessage, MsgFlags, SockFlag, SockaddrIn, connect, sendmsg, socket,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    error::Error,
    io::IoSlice,
    net::{SocketAddr, ToSocketAddrs},
    os::fd::{AsRawFd, OwnedFd},
    process::exit,
    str::FromStr,
    sync::Mutex,
};

type Result<T> = core::result::Result<T, Box<dyn Error>>;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct UserArgs {
    /// Connection timeout in milliseconds
    #[clap(long, default_value = "2000")]
    timeout: u64,

    /// target list
    hosts: Vec<String>,
}

fn connect_to_targets(addresses: Vec<SocketAddr>, _timeout_ms: u64) -> Result<OwnedFd> {
    for address in addresses {
        let af = match address.is_ipv4() {
            true => AddressFamily::Inet,
            false => AddressFamily::Inet6,
        };

        let socket = socket(af, socket::SockType::Stream, SockFlag::empty(), None)?;

        let addr = SockaddrIn::from_str(&address.to_string())?;

        if connect(socket.as_raw_fd(), &addr).is_ok() {
            return Ok(socket);
        }
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

fn pass_fd(fd: OwnedFd) -> Result<()> {
    let iov = [IoSlice::new(b"\0")];
    let fds = [fd.as_raw_fd()];
    let cmsg = ControlMessage::ScmRights(&fds);
    sendmsg::<()>(1, &iov, &[cmsg], MsgFlags::empty(), None)?;
    Ok(())
}

fn main() -> Result<()> {
    let args = UserArgs::parse();

    if args.hosts.is_empty() {
        return Err("Host Missing".into());
    }

    let passfd_mutex = Mutex::new(0);

    //
    // this'll try all the target at once (par_iter) and whichever
    // gets to the pass_fd() function wins and we exit() the process
    //
    args.hosts.par_iter().for_each(|target| {
        if let Ok(addrs) = parse_target(target) {
            if let Ok(socket) = connect_to_targets(addrs, args.timeout) {
                if passfd_mutex.lock().is_ok() {
                    match pass_fd(socket) {
                        Ok(_) => exit(0),
                        Err(e) => eprintln!("{e}"),
                    }
                }
            }
        }
    });

    Err("Not Connected".into())
}
