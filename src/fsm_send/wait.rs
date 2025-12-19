use std::io;

use crate::fsm_send::fsm::{
    FsmStateWrapper, FsmWrap, SndEvent, SndFsm, SndStateWait, StateRouter, next_n,
};

use super::*;

impl StateRouter for SndFsm<SndStateWait> {
    fn goto(
        self,
        e: SndEvent,
        ctx: &mut dyn fsm::ProtocolIoContext,
    ) -> io::Result<FsmStateWrapper> {
        let n = self.state().n();
        match e {
            // edge 2a: timeout < max_retrans
            SndEvent::Timeout if self.state().retransmit_counter() < self.max_retransmits() => {
                ctx.udt_send(self.state().sndpkt())?;
                ctx.start_timer()?;
                Ok(self.inc_retransmit().wrap())
            }

            // edge 2b: timeout > max_retrans
            SndEvent::Timeout => Ok(self.to_end().wrap()),

            // edge 3: valid ack
            SndEvent::RecvPck(Some(rcvpkt))
                if rcvpkt.notcorrupt() && rcvpkt.is_ACK() && n == rcvpkt.n() =>
            {
                ctx.stop_timer()?;
                Ok(self.to_send(next_n(n)).wrap())
            }

            // edge 7: recv fin ack and not data available
            SndEvent::RecvPck(Some(rcvpkt))
                if rcvpkt.notcorrupt()
                    && rcvpkt.is_FINACK()
                    && n == rcvpkt.n()
                    && !ctx.data_available()? =>
            {
                Ok(self.to_end().wrap())
            }

            // corrupt packet (could not be parsed)
            SndEvent::RecvPck(None) => Ok(self.wrap()),

            // edge 8: corrupt/wrong ack -> wait for timeout from driver loop
            SndEvent::RecvPck(Some(rcvpkt))
                if rcvpkt.corrupt() || (rcvpkt.is_ACK() && n != rcvpkt.n()) =>
            {
                Ok(self.wrap())
            }

            // ..undefined
            _ => unreachable!("undefined transition"),
        }
    }
}
