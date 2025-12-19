use super::fsm::FsmStateWrapper;
use super::fsm::FsmWrap;
use std::io;

use super::fsm::ProtocolIoContext;
use super::fsm::RcvEvent;
use super::fsm::RcvFsm;
use super::fsm::StateRouter;

pub fn run_rcv_fsm_loop(
    ctx: &mut impl ProtocolIoContext,
) -> io::Result<()> {
    // connection handshake via SYN and file name pkt
    let mut cur_fsm_wrap = RcvFsm::init().wrap();

    // run fsm
    loop {
        let event = get_next_event_for_current_state(&mut cur_fsm_wrap, ctx)?;

        cur_fsm_wrap = match cur_fsm_wrap {
            FsmStateWrapper::WaitForConnection(fsm) => fsm.goto(event, ctx)?,
            FsmStateWrapper::WaitForPkt(fsm) => fsm.goto(event, ctx)?,
        };
    }

    Ok(())
}

fn get_next_event_for_current_state(
    wrapper: &mut FsmStateWrapper,
    ctx: &mut impl ProtocolIoContext,
) -> io::Result<RcvEvent> {
    match wrapper {
        // blocking until new pck recvd
        FsmStateWrapper::WaitForConnection(_) => ctx.wait_for_pck_no_timeout(),
        
        // check if data is available
        FsmStateWrapper::WaitForPkt(_) => ctx.wait_for_ack_or_timeout(),
    }
}
