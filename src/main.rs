extern crate cannyls;
extern crate cannyls_rpc;
extern crate fibers;
extern crate fibers_rpc;
extern crate futures;
#[macro_use]
extern crate trackable;
extern crate structopt;
#[macro_use]
extern crate clap;

use cannyls::lump::LumpId;
use cannyls_rpc::{Client, DeviceId};
use fibers::{Executor, InPlaceExecutor, Spawn};
use fibers_rpc::client::ClientService;
use futures::{Async, Future};
use std::net::SocketAddr;
use std::str::FromStr;
use std::thread;
use structopt::StructOpt;

macro_rules! wait {
    ($future:expr) => {{
        let mut f = $future;
        loop {
            if let Async::Ready(item) = track_try_unwrap!(f.poll()) {
                break item;
            }
        }
    }};
}

fn to_device_id(d: &str) -> DeviceId {
    DeviceId::new(d)
}

arg_enum! {
    #[derive(Debug)]
    enum Command {
        List,
        Get,
        Head,
        Delete,
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "WatariKani")]
struct Opt {
    #[structopt(long = "rpc-addr", default_value = "127.0.0.1:14278")]
    addr: SocketAddr,

    #[structopt(long = "device")]
    device_id: String,

    #[structopt(long = "lumpid")]
    lump_id: Option<String>,

    #[structopt(raw(
        possible_values = "&Command::variants()",
        requires_ifs = r#"&[
("Get", "lump_id"),
("Head", "lump_id"),
("Delete", "lump_id"),
]"#
    ))]
    command: Command,
}

fn main() {
    let opt = Opt::from_args();
    let server_addr = opt.addr;
    let device_id = to_device_id(&opt.device_id);

    let executor = track_try_unwrap!(track_any_err!(InPlaceExecutor::new()));

    let service = ClientService::new(executor.handle());
    let service_handle = service.handle();
    executor.spawn(service.map_err(|e| panic!("{}", e)));

    thread::spawn(move || {
        if let Err(e) = executor.run() {
            panic!("{}", e);
        }
    });

    let client = Client::new(server_addr, service_handle);
    let request = client.request();

    match opt.command {
        Command::List => {
            let listed = wait!(request.list_lumps(device_id));
            println!("listed.len() = {}", listed.len());
            for e in listed {
                println!("{:?}", e);
            }
        }
        Command::Get => {
            let lump_id = LumpId::from_str(&opt.lump_id.unwrap()).unwrap();
            let object = wait!(request.get_lump(device_id, lump_id));
            if let Some(data) = object {
                println!("{:?}", data);
            } else {
                println!("{:?} does not exist", lump_id);
            }
        }
        Command::Head => {
            let lump_id = LumpId::from_str(&opt.lump_id.unwrap()).unwrap();
            let info = wait!(request.head_lump(device_id, lump_id));
            if let Some(data) = info {
                println!("{:?}", data);
            } else {
                println!("{:?} does not exist", lump_id);
            }
        }
        Command::Delete => {
            let lump_id = LumpId::from_str(&opt.lump_id.unwrap()).unwrap();
            let removed = wait!(request.delete_lump(device_id, lump_id));
            if removed {
                println!("Removed {:?}", lump_id);
            } else {
                println!("There is no {:?}", lump_id);
            }
        }
    }
}
