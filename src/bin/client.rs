use clap::Parser;
use secsnail::sock::{DEFAULT_SECSNAIL_PORT, SecSnailSocket};
use std::net::SocketAddr;

/// Demo client starts a secure snail file transmission:
///
///   Use default secsnail port 55055
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let recv_addr: SocketAddr = format!("{}:{}", args.ip, DEFAULT_SECSNAIL_PORT)
        // mhieron: Wenn ihr eh schon ein Result zurückgebt, warum nicht den Fragezeichen Operator verwenden?
        .parse()?;

    // mhieron: Ich finde den Funktionsnamen `bind` etwas misleading. Ein Server macht normalerweise ein `bind`. Ein Client nur ein `connect`.
    let mut secsnail_sock = SecSnailSocket::bind("0.0.0.0:45454")?;

    // mhieron: Warum hier nochmal die Werte setzen? Die sind doch eh schon default.
    secsnail_sock.set_rcv_file_timeout_ms(100);
    secsnail_sock.set_snd_file_max_retransmits(10);
    secsnail_sock.set_unreliable_transmit_parameters(args.loss_p, args.error_p, args.dup_p);

    let (amt_bytes, dur) = secsnail_sock.send_file_blocking(args.file_name, recv_addr)?;

    println!(
        "Sent {amt_bytes} bytes via secure snail 🐌 in {} s",
        dur.as_secs_f64()
    );
    println!(
        "-> Goodput: {} kByte/s",
        amt_bytes as u128 / dur.as_millis()
    );
    Ok(())
}

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about= None)]
struct Args {
    #[arg(short, long)]
    ip: String,
    #[arg(short, long)]
    file_name: String,
    #[arg(short, long, default_value_t = 0.0)]
    loss_p: f64,
    #[arg(short, long, default_value_t = 0.0)]
    error_p: f64,
    #[arg(short, long, default_value_t = 0.0)]
    dup_p: f64,
}
