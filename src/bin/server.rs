use clap::Parser;
use secsnail::sock::SecSnailSocket;
use std::io;

/// Demo server listens for incoming secure snail file transmissions
///
///   Use default secsnail port 55055
fn main() -> io::Result<()> {
    let args = Args::parse();
    let mut secsnail_sock = SecSnailSocket::bind_default_port().unwrap();
    secsnail_sock.set_unreliable_transmit_parameters(args.loss_p, args.error_p, args.dup_p);
    secsnail_sock.recv_file_blocking(args.destination).unwrap();
    Ok(())
}

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about= None)]
struct Args {
    #[arg(long)]
    destination: String,
    #[arg(short, long, default_value_t = 0.0)]
    loss_p: f64,
    #[arg(short, long, default_value_t = 0.0)]
    error_p: f64,
    #[arg(short, long, default_value_t = 0.0)]
    dup_p: f64,
}
