use std::io;

use super::super::pck::Flag;

use super::super::pck::Packet;

pub enum SndEvent {
    InitSYN,
    Timeout,
    RecvPck(Option<Packet>),
    DataAvailable(bool),
}

// start
#[derive(Clone)]
pub struct SndStateStart {
    n: u8,
}

impl SndStateStart {
    pub fn new(n: u8) -> Self {
        Self { n }
    }
    pub fn n(&self) -> u8 {
        self.n
    }
}

// wait
#[derive(Clone)]
pub struct SndStateWait {
    n: u8,
    retransmit_counter: u8,
    /// last sent packet
    sndpkt: Packet,
}

impl SndStateWait {
    pub fn new(n: u8, sndpkt: Packet) -> Self {
        Self {
            n,
            retransmit_counter: 0,
            sndpkt,
        }
    }
    pub fn n(&self) -> u8 {
        self.n
    }

    pub fn retransmit_counter(&self) -> u8 {
        self.retransmit_counter
    }

    pub fn sndpkt(&self) -> &Packet {
        &self.sndpkt
    }
}

// send
#[derive(Clone)]
pub struct SndStateSend {
    pub n: u8,
}

impl SndStateSend {
    pub fn new(n: u8) -> Self {
        Self { n }
    }
    pub fn n(&self) -> u8 {
        self.n
    }
}

// end
#[derive(Clone)]
pub struct SndStateEnd;

// fsm
#[derive(Clone, Copy)]
struct Config {
    max_retransmits: u8,
}

impl Config {
    pub fn new(max_retransmits: u8) -> Self {
        Self { max_retransmits }
    }
}

pub struct SndFsm<State: Clone> {
    _state: State,
    _config: Config,
}

impl<State: Clone> SndFsm<State> {
    pub fn new(state: State, max_retransmits: u8) -> Self {
        SndFsm {
            _state: state,
            _config: Config::new(max_retransmits),
        }
    }

    pub fn max_retransmits(&self) -> u8 {
        self._config.max_retransmits
    }

    /// immutable reference
    pub fn state(&self) -> &State {
        &self._state
    }

    pub fn to_send(&self, n: u8) -> SndFsm<SndStateSend> {
        SndFsm {
            _state: SndStateSend::new(n),
            _config: self._config,
        }
    }

    pub fn to_wait(&self, n: u8, sndpkt: Packet) -> SndFsm<SndStateWait> {
        SndFsm {
            _state: SndStateWait::new(n, sndpkt),
            _config: self._config,
        }
    }

    pub fn to_end(&self) -> SndFsm<SndStateEnd> {
        SndFsm {
            _state: SndStateEnd,
            _config: self._config,
        }
    }
}

pub trait FsmWrap {
    fn wrap(self) -> FsmStateWrapper;
}

// impl wrap on all states
impl FsmWrap for SndFsm<SndStateStart> {
    fn wrap(self) -> FsmStateWrapper {
        FsmStateWrapper::Start(self)
    }
}

impl FsmWrap for SndFsm<SndStateWait> {
    fn wrap(self) -> FsmStateWrapper {
        FsmStateWrapper::Wait(self)
    }
}

impl FsmWrap for SndFsm<SndStateSend> {
    fn wrap(self) -> FsmStateWrapper {
        FsmStateWrapper::Send(self)
    }
}

impl FsmWrap for SndFsm<SndStateEnd> {
    fn wrap(self) -> FsmStateWrapper {
        FsmStateWrapper::End(self)
    }
}

// inc retransmit
impl SndFsm<SndStateWait> {
    pub fn inc_retransmit(&self) -> Self {
        let s = SndStateWait {
            retransmit_counter: self.state().retransmit_counter() + 1,
            ..self.state().clone()
        };
        SndFsm::new(s, self.max_retransmits())
    }
}

// fsm entriy point

impl SndFsm<SndStateStart> {
    // Dies ist der "Einstiegspunkt" in die State Machine
    /// fsm start entry point
    pub fn init(max_retransmits: u8) -> SndFsm<SndStateStart> {
        SndFsm::new(SndStateStart::new(0), max_retransmits)
    }
}

pub enum FsmStateWrapper {
    Start(SndFsm<SndStateStart>),
    Wait(SndFsm<SndStateWait>),
    Send(SndFsm<SndStateSend>),
    End(SndFsm<SndStateEnd>),
}

pub trait StateRouter {
    // Gibt immer den Wrapper-Typ zurück, egal wie der tatsächliche Folgezustand heißt.
    // &mut dyn ProtocolIoContext muss dabei sein, um I/O zu ermöglichen.
    fn goto(self, e: SndEvent, ctx: &mut dyn ProtocolIoContext) -> io::Result<FsmStateWrapper>;
}

pub trait ProtocolIoContext {
    /// updates timer if timeout occured before re listening for incoming packet with udp socket
    /// only accepts packets with configured recv_addr in ctx
    fn wait_for_ack_or_timeout(&mut self) -> io::Result<SndEvent>; // Gibt ein FSM Event zurück (RecvAck, Timeout, Corrupt)

    fn data_available(&mut self) -> io::Result<bool>;
    fn make_pkt(&mut self, seq_n: u8, f: Flag) -> io::Result<Packet>;

    /// create start_timer instant and set read timeout to timeout Duration
    fn start_timer(&mut self) -> io::Result<()>;
    fn stop_timer(&mut self) -> io::Result<()>;
    fn udt_send(&mut self, pck: &Packet) -> io::Result<()>;

    /// Track amount of data transmitted
    fn get_data_counter(&self) -> usize;
    fn increase_data_counter(&mut self, n: usize);
}

pub fn next_n(n: u8) -> u8 {
    match n {
        0 => 1,
        _ => 0,
    }
}
