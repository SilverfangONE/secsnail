use super::fsm::FsmStateWrapper;
use super::fsm::FsmWrap;
use std::{io, time::Duration, time::Instant};

use super::fsm::ProtocolIoContext;
use super::fsm::SndEvent;
use super::fsm::SndFsm;
use super::fsm::StateRouter;

pub fn run_snd_fsm_loop(
    ctx: &mut impl ProtocolIoContext,
    max_retransmits: u8,
) -> io::Result<(usize, Duration)> {
    // connection handshake via SYN and file name pkt
    let mut cur_fsm_wrap = SndFsm::init(max_retransmits).wrap();

    let start_time = Instant::now();

    // run fsm
    loop {
        if let FsmStateWrapper::End = cur_fsm_wrap {
            break;
        }

        let event = get_next_event_for_current_state(&mut cur_fsm_wrap, ctx)?;

        cur_fsm_wrap = match cur_fsm_wrap {
            FsmStateWrapper::Start(fsm) => fsm.goto(event, ctx)?,
            FsmStateWrapper::Wait(fsm) => fsm.goto(event, ctx)?,
            FsmStateWrapper::Send(fsm) => fsm.goto(event, ctx)?,

            // end state gets handled above
            FsmStateWrapper::End => unreachable!(),
        };
    }

    Ok((ctx.get_data_counter(), start_time.elapsed()))
}

fn get_next_event_for_current_state(
    wrapper: &mut FsmStateWrapper,
    ctx: &mut impl ProtocolIoContext,
) -> io::Result<SndEvent> {
    match wrapper {
        // blocking until event or timeout occured
        FsmStateWrapper::Wait(_) => ctx.wait_for_ack_or_timeout(),

        // check if data ist available
        FsmStateWrapper::Send(_) => Ok(SndEvent::DataAvailable(ctx.data_available()?)),

        // init event for handshake
        FsmStateWrapper::Start(_) => Ok(SndEvent::InitSYN),

        FsmStateWrapper::End => {
            unreachable!("Never call Event on end state in snd fsm");
        }
    }
}
