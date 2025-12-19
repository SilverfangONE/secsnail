use std::{
    io::{self},
    net::SocketAddr,
};

use super::super::pck::Flag;

use super::super::pck::Packet;

pub enum RcvEvent {
    ConnectionTimeout,
    /// rcvpkt and recv_addr
    RecvPck(Option<Packet>, SocketAddr),
}

// Connection / start
#[derive(Clone)]
pub struct RcvStateWaitForConnection {}

impl RcvStateWaitForConnection {
    pub fn new() -> Self {
        Self {}
    }
}

// Wait for Pkt
#[derive(Clone)]
pub struct RcvStateWaitForPkt {
    /// last sent packet
    sndpkt: Packet,
}

impl RcvStateWaitForPkt {
    pub fn new(sndpkt: Packet) -> Self {
        Self { sndpkt }
    }

    pub fn sndpkt(&self) -> &Packet {
        &self.sndpkt
    }
}

// fsm

#[derive(Clone, Copy)]
struct Config {}

impl Config {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct RcvFsm<State: Clone> {
    _state: State,
    _config: Config,
}

impl<State: Clone> RcvFsm<State> {
    pub fn new(state: State) -> Self {
        RcvFsm {
            _state: state,
            _config: Config::new(),
        }
    }

    /// inmutable refrence
    pub fn state(&self) -> &State {
        &self._state
    }

    pub fn to_wait_for_connection(&self) -> RcvFsm<RcvStateWaitForConnection> {
        RcvFsm {
            _state: RcvStateWaitForConnection::new(),
            _config: self._config,
        }
    }

    pub fn to_wait_for_pkt(&self, sndpkt: Packet) -> RcvFsm<RcvStateWaitForPkt> {
        RcvFsm {
            _state: RcvStateWaitForPkt::new(sndpkt),
            _config: self._config,
        }
    }
}

// wrap trait on all states
pub trait FsmWrap {
    fn wrap(self) -> FsmStateWrapper;
}

impl FsmWrap for RcvFsm<RcvStateWaitForConnection> {
    fn wrap(self) -> FsmStateWrapper {
        FsmStateWrapper::WaitForConnection(self)
    }
}

impl FsmWrap for RcvFsm<RcvStateWaitForPkt> {
    fn wrap(self) -> FsmStateWrapper {
        FsmStateWrapper::WaitForPkt(self)
    }
}

// fsm entry point

impl RcvFsm<RcvStateWaitForConnection> {
    /// fsm start entry point
    pub fn init() -> RcvFsm<RcvStateWaitForConnection> {
        RcvFsm::new(RcvStateWaitForConnection::new())
    }
}

pub enum FsmStateWrapper {
    WaitForConnection(RcvFsm<RcvStateWaitForConnection>),
    WaitForPkt(RcvFsm<RcvStateWaitForPkt>),
}

pub trait StateRouter {
    // Gibt immer den Wrapper-Typ zurück, egal wie der tatsächliche Folgezustand heißt.
    // &mut dyn ProtocolIoContext muss dabei sein, um I/O zu ermöglichen.
    fn goto(self, e: RcvEvent, ctx: &mut dyn ProtocolIoContext) -> io::Result<FsmStateWrapper>;
}

pub trait ProtocolIoContext {
    /// set snd_addr, rcv any other packet will be ignored
    fn set_snd_addr(&mut self, snd_addr: SocketAddr);
    fn extract_data<'a>(&mut self, rcvpkt: &'a Packet) -> &'a [u8];
    fn extract_file_name(&mut self, rcvpkt: &Packet) -> io::Result<String>;
    fn append(&mut self, data: &[u8]) -> io::Result<()>;
    fn wait_for_ack_or_timeout(&mut self) -> io::Result<RcvEvent>; // Gibt ein FSM Event zurück (RecvAck, Timeout, Corrupt)
    fn wait_for_pck_no_timeout(&mut self) -> io::Result<RcvEvent>;

    fn make_pkt(&mut self, seq_n: u8, f: Flag) -> io::Result<Packet>;

    /// create start_timer instant and set read timeout to timeout Duration
    fn start_connection_timer(&mut self) -> io::Result<()>;
    fn stop_connection_timer(&mut self) -> io::Result<()>;
    fn restart_connection_timer(&mut self) -> io::Result<()>;

    fn close_file(&mut self) -> io::Result<()>;
    fn open_file(&mut self, filename: &str) -> io::Result<()>;

    fn udt_send(&mut self, pck: &Packet) -> io::Result<()>;

    /// Track amount of data transmitted
    fn get_data_counter(&self) -> usize;
    fn increase_data_counter(&mut self, n: usize);
    fn reset_data_counter(&mut self);
}
