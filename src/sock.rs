//! The `SecSnailSocket` represents a bidirectional UDP transport endpoint
//! used by both sending and receiving finite state machines (FSM).
//!
//! Each transfer (either sending or receiving) owns a temporary
//! protocol I/O context (`SendProtocolIoContext`, `RecvProtocolIoContext`)
//! which drives the FSM logic on top of the same socket.
//!
//! For now, the socket supports one transfer at a time (blocking).
//!
//!
//! by Jan Spenneman & Luis Andrés Boden
//!
//!
//!     .----.   @   @
//!    / .-"-.`.  \v/
//!    | | '\ \ \_/ )
//!  ,-\ `-.' /.'  /
//! '---`----'----' hjw
//!
//! Art credit: Hayley Jane Wakenshaw — thank you!

use std::{
    fs::{self, File},
    io::{self, BufRead, BufReader, BufWriter, Read, Write},
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    path::Path,
    time::{Duration, Instant},
    str,
};

use crate::{fsm_recv::{self, driver::run_rcv_fsm_loop, fsm::RcvEvent}, pck::MAX_PAYLOAD_SIZE};

use super::pck::Flag;
use super::pck::Packet;
use super::{fsm_send::driver::run_snd_fsm_loop, util::u8_to_bool};
use crate::fsm_send;

pub const DEFAULT_MAX_RETRANSMITS: u8 = 100;

pub const DEFAULT_SND_TIMEOUT_MS: u64 = 10;
pub const DEFAULT_RCV_TIMEOUT_MS: u64 = 5000;

pub const DEFAULT_FIRST_N: u8 = 0;
pub const DEFAULT_SECSNAIL_PORT: u16 = 55055;

enum RecvResult {
    RecvPkt(Option<Packet>, SocketAddr),
    Timeout,
}

struct SendProtocolIoContext<'a> {
    sock_ref: &'a mut SecSnailSocket,
    timeout: Duration,
    timer_start: Option<Instant>,
    recv_addr: SocketAddr,
    buf_redr: BufReader<File>,
    file_name: String,
    data_counter: usize,
}

impl<'a> SendProtocolIoContext<'a> {
    fn new<P: AsRef<Path>>(
        sock_ref: &'a mut SecSnailSocket,
        recv_addr: SocketAddr,
        path: P,
    ) -> io::Result<Self> {
        // file io
        let path = path.as_ref();
        let file_name = path
            .file_name()
            .and_then(|f| f.to_str())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name"))?
            .to_string();
        let file = File::open(path)?;
        let buf_redr = BufReader::new(file);

        // get timeout of sock_ref before borrowing to ctx
        let timeout = sock_ref.snd_timeout_config;

        Ok(SendProtocolIoContext {
            timer_start: None,
            file_name,
            recv_addr,
            sock_ref,
            buf_redr,
            timeout,
            data_counter: 0
        })
    }
}

impl<'a> fsm_send::fsm::ProtocolIoContext for SendProtocolIoContext<'a> {
    fn wait_for_ack_or_timeout(&mut self) -> io::Result<fsm_send::fsm::SndEvent> {
        let r = self.sock_ref.wait_for_incoming_or_timeout(
            Some(self.recv_addr),
            self.timeout,
            self.timer_start.unwrap(),
        )?;
        match r {
            RecvResult::RecvPkt(rcvpkt, _) => Ok(fsm_send::fsm::SndEvent::RecvPck(rcvpkt)),
            RecvResult::Timeout => Ok(fsm_send::fsm::SndEvent::Timeout),
        }
    }

    fn data_available(&mut self) -> io::Result<bool> {
        Ok(self.buf_redr.fill_buf()?.len() > 0)
    }

    fn make_pkt(&mut self, seq_n: u8, f: Flag) -> io::Result<Packet> {
        let payload: Vec<u8> = match f {
            Flag::Data => {
                let mut buf: Vec<u8> = vec![0; Packet::max_pck_payload_size()];
                let n = self.buf_redr.read(&mut buf)?;

                let slice: &[u8] = &buf[..n];
                slice.to_vec()
            }
            Flag::SYN => {
                // init data: is file_name
                self.file_name.clone().into_bytes()
            }

            // ACK, FIN, FINACK
            _ => vec![],
        };

        Packet::new(u8_to_bool(seq_n), f, payload)
    }

    /// create start_timer instant and set read timeout to timeout Duration
    fn start_timer(&mut self) -> io::Result<()> {
        self.timer_start = Some(Instant::now());
        // no timeout occures by starting timer
        _ = self
            .sock_ref
            .update_udp_sock_timeout(self.timer_start.unwrap(), self.timeout)?;
        Ok(())
    }

    fn stop_timer(&mut self) -> io::Result<()> {
        self.timer_start.take();
        // TODO: can man vielleicht raus nehmen not sure though
        self.sock_ref.inner.set_read_timeout(Some(self.timeout))?;
        Ok(())
    }

    fn udt_send(&mut self, pck: &Packet) -> io::Result<()> {
        // TODO: count read bytes for analyises metrics
        self.sock_ref.udt_send(pck, self.recv_addr)?;
        Ok(())
    }

    fn get_data_counter(&self) -> usize {
        self.data_counter
    }

    fn increase_data_counter(&mut self, n: usize) {
        self.data_counter += n;
    }
}


struct RecvProtocolIoContext<'a> {
    sock_ref: &'a mut SecSnailSocket,
    snd_addr: Option<SocketAddr>,
    buf_wrt: Option<BufWriter<File>>,
    connection_timeout: Duration,
    connection_timer_start: Option<Instant>,
    target_dir: &'a Path,
    data_counter: usize
}

impl<'a> RecvProtocolIoContext<'a> {
    pub fn new(    
        sock_ref: &'a mut SecSnailSocket,
        target_dir: &'a Path,
        connection_timeout: Duration,
    ) -> Self {
        Self {
            sock_ref,
            target_dir,
            connection_timeout,
            connection_timer_start: None,
            snd_addr: None,
            buf_wrt: None,
            data_counter: 0
        }
    }
}
impl<'b> fsm_recv::fsm::ProtocolIoContext for RecvProtocolIoContext<'b> {
    fn set_snd_addr(&mut self, snd_addr: SocketAddr) {
        self.snd_addr.replace(snd_addr);
    }

    fn extract_data<'a>(&mut self, rcvpkt: &'a Packet) -> &'a [u8] {
        rcvpkt.payload()
    }

    fn extract_file_name(&mut self, rcvpkt: &Packet) -> io::Result<String> {
        match str::from_utf8(rcvpkt.payload()) {
            Ok(v) => Ok(v.to_string()),
            Err(e) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid UTF-8 sequence: {}", e),
            )),
        }    
    }

    /// not write to buffer if buffer was not check 
    fn append(&mut self, data: &[u8]) -> io::Result<()> {
        #[cfg(debug_assertions)]
        {
            if self.buf_wrt.is_none() {
                unreachable!("buf_wrt in ctx should always be set by calling append in fmt");
            }
        }
        self.buf_wrt.as_mut().unwrap().write(data)?;
        Ok(())
    }

    /// never call this functino if snd_addr is not set
    fn wait_for_ack_or_timeout(&mut self) -> io::Result<RcvEvent> {
         let r = self.sock_ref.wait_for_incoming_or_timeout(
            self.snd_addr,
            self.connection_timeout,
            self.connection_timer_start.unwrap(),
        )?;
        match r {
            RecvResult::RecvPkt(rcvpkt, rcv_addr) => Ok(RcvEvent::RecvPck(rcvpkt, rcv_addr)),
            RecvResult::Timeout => Ok(RcvEvent::ConnectionTimeout),
        }
    }

    fn wait_for_pck_no_timeout(&mut self) -> io::Result<RcvEvent> {
        self.sock_ref.inner.set_read_timeout(None)?;
        loop {
            match self.sock_ref.rdt_recv() {
                Ok((src, rcv_pck)) => {
                    return Ok(RcvEvent::RecvPck(rcv_pck, src))
                }
                Err(e) => return Err(e),
            }
        }
     }


    fn make_pkt(&mut self, seq_n: u8, f: Flag) -> io::Result<Packet> {
        Packet::new(u8_to_bool(seq_n), f, vec![])
    }

    /// create start_timer instant and set read timeout to timeout Duration
    fn start_connection_timer(&mut self) -> io::Result<()> {
        self.connection_timer_start = Some(Instant::now());
        // no timeout occures by starting timer
        _ = self
            .sock_ref
            .update_udp_sock_timeout(self.connection_timer_start.unwrap(), self.connection_timeout)?;
        Ok(())
    }

    fn stop_connection_timer(&mut self) -> io::Result<()> {
        self.connection_timer_start.take();
        // TODO: can man vielleicht raus nehmen not sure though
        self.sock_ref.inner.set_read_timeout(Some(self.connection_timeout))?;
        Ok(())
    }
    fn restart_connection_timer(&mut self) -> io::Result<()> {
        self.start_connection_timer()
    }

    fn close_file(&mut self) -> io::Result<()> {
        self.buf_wrt.as_mut().unwrap().flush()?;
        self.buf_wrt.take();
        self.snd_addr.take();
        Ok(())
    }

    fn open_file(&mut self, filename: &str) -> io::Result<()> {
        // TODO: incsure filename ohne '/'        
        let file = File::create(self.target_dir.join(filename))?;
        self.buf_wrt.replace(BufWriter::new(file));
        Ok(())
    }

    /// call only if snd_addr is set
    fn udt_send(&mut self, pck: &Packet) -> io::Result<()> {
        // TODO: count read bytes for analyises metrics
        self.sock_ref.udt_send(pck, self.snd_addr.unwrap())?;
        Ok(())
    }

    fn get_data_counter(&self) -> usize {
        self.data_counter
    }

    fn increase_data_counter(&mut self, n: usize) {
        self.data_counter += n;
    }

    fn reset_data_counter(&mut self) {
        self.data_counter = 0;
    }
}

/// Currently it is only possible to sendet one file per time through one socket
/// in result you need to create a new one to send mulitple files in parallel
pub struct SecSnailSocket {
    inner: UdpSocket,
    snd_max_retransmits: u8,
    snd_timeout_config: Duration,
    rcv_timeout_config: Duration,
    error_p: f64,
    loss_p: f64,
    dup_p: f64
}

impl SecSnailSocket {
    pub fn bind_default_port() -> io::Result<SecSnailSocket> {
        SecSnailSocket::bind(format!("0.0.0.0:{DEFAULT_SECSNAIL_PORT}"))
    }

    pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<SecSnailSocket> {
        let sock = UdpSocket::bind(addr)?;
        io::Result::Ok(SecSnailSocket {
            inner: sock,
            snd_max_retransmits: DEFAULT_MAX_RETRANSMITS,
            snd_timeout_config: Duration::from_millis(DEFAULT_SND_TIMEOUT_MS),
            rcv_timeout_config: Duration::from_millis(DEFAULT_RCV_TIMEOUT_MS),
            error_p:  0.0,
            dup_p:  0.0,
            loss_p:  0.0
        })
    }

    pub fn set_unreliable_transmit_parameters(&mut self, loss_p: f64, error_p: f64, dup_p: f64) {
        self.loss_p = loss_p;
        self.error_p = error_p;
        self.dup_p = dup_p;
    }

    // socket blocking functionality

    /// send file with udp socket blocking
    pub fn send_file_blocking<P: AsRef<Path>>(
        &mut self,
        path: P,
        recv_addr: SocketAddr,
    ) -> io::Result<(usize, Duration)> {
        let max_transmits = self.snd_max_retransmits;
        let mut ctx = SendProtocolIoContext::new(self, recv_addr, path)?;
        let ret = run_snd_fsm_loop(&mut ctx, max_transmits)?;
        Ok(ret)
    }

    /// rcv file with udp socket blocking
    pub fn recv_file_blocking<P: AsRef<Path>>(
        &mut self,
        target_dir: P,
    ) -> io::Result<()> {
        let target_dir = target_dir.as_ref();

        // check if path is a file
        if let Ok(metadata) = fs::metadata(target_dir) {
            if metadata.is_file() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "given dir path '{}' exists and is a file, only a target dir is expected.",
                        target_dir.display()
                    ),
                ));
            }
        }

        fs::create_dir_all(target_dir)?;

        // setup
        let mut ctx = RecvProtocolIoContext::new(
            self, target_dir, self.rcv_timeout_config
        );
        run_rcv_fsm_loop(&mut ctx)?;
        Ok(())
    }

    // socket configuration functions


    pub fn set_snd_file_timeout_ms(&mut self, timeout_ms: u64) {
        self.snd_timeout_config = Duration::from_millis(timeout_ms);
    }

    pub fn set_rcv_file_timeout_ms(&mut self, timeout_ms: u64) {
        self.rcv_timeout_config = Duration::from_millis(timeout_ms);
    }

    pub fn set_snd_file_max_retransmits(&mut self, max: u8) {
        self.snd_max_retransmits = max;
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.inner.peer_addr()
    }

    // utils

    fn wait_for_incoming_or_timeout(
        &mut self,
        recv_addr_opt: Option<SocketAddr>,
        timeout: Duration,
        timer_start: Instant,
    ) -> io::Result<RecvResult> {
        // waiting for correct ack or timeout
        loop {
            if self.update_udp_sock_timeout(timer_start, timeout)? {
                return Ok(RecvResult::Timeout);
            }
            match self.rdt_recv() {
                Ok((src, resp_pck)) => {
                    // skip rcv_pkt only if rcv_addr_opt ist 
                    // set and not same as src
                    return match recv_addr_opt {
                        Some(rcv_addr) if rcv_addr != src => {
                            continue;
                        }
                        _ => Ok(RecvResult::RecvPkt(resp_pck, src))
                    };
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    return Ok(RecvResult::Timeout);
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// update socket udp timer accordinly to remaing timeout
    ///
    /// # Return
    /// true if timeout as reached, else false
    fn update_udp_sock_timeout(
        &mut self,
        timer_start: Instant,
        timeout: Duration,
    ) -> io::Result<bool> {
        // calc remaing timer time
        let elapsed = timer_start.elapsed();
        if elapsed >= timeout {
            // reached timeout
            return Ok(true);
        }

        let remaining = timeout - elapsed;
        self.inner.set_read_timeout(Some(remaining))?;
        Ok(false)
    }

    fn udt_send(&self, sndpkt: &Packet, recv_addr: SocketAddr) -> io::Result<usize> {
        // Simulate Packet loss
        if rand::random_bool(self.loss_p) {
            return Ok(0)
        }

        let mut pkt = sndpkt.encode().to_vec();

        // Simulate Packet Error
        if rand::random_bool(self.error_p) {
            let mask: u8 = 1 << rand::random_range(0..8);
            let l = pkt.len();
            pkt[rand::random_range(0..l)] ^= mask;
        }

        // Simulate Packet Duplication
        if rand::random_bool(self.dup_p) {
            let _ = self.inner.send_to(&pkt, recv_addr);
        }

        self.inner.send_to(&pkt, recv_addr)
    }

    /// TODO: only accept connectionts to recv_addr ??
    fn rdt_recv(&self) -> io::Result<(SocketAddr, Option<Packet>)> {
        let mut buf: Vec<u8> = vec![0; MAX_PAYLOAD_SIZE];
        let (_, src) = self.inner.recv_from(&mut buf)?;
        match Packet::decode(buf) {
            Ok(pck) => Ok((src, Some(pck))),
            Err(_) => Ok((src, None))
        }
    }
}
