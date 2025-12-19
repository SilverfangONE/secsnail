use std::io;

use crate::{
    fsm_recv::fsm::{FsmStateWrapper, FsmWrap, RcvEvent, RcvFsm, RcvStateWaitForPkt, StateRouter},
    pck::Flag,
};

use super::*;

impl StateRouter for RcvFsm<RcvStateWaitForPkt> {
    fn goto(
        self,
        e: RcvEvent,
        ctx: &mut dyn fsm::ProtocolIoContext,
    ) -> io::Result<FsmStateWrapper> {
        match e {
            // packet corrupt (could not be parsed)
            RcvEvent::RecvPck(None, _) => Ok(self.wrap()),
            // edge 8: rcvpkt corrupt (checksum) oder syn
            RcvEvent::RecvPck(Some(rcvpkt), _) if rcvpkt.corrupt() || rcvpkt.is_SYN() => Ok(self.wrap()),

            // edge 9: rcvpkt (data) with wrong n => resend ack (last sndpkt)
            RcvEvent::RecvPck(Some(rcvpkt), _)
                if rcvpkt.notcorrupt() && rcvpkt.n() == self.state().sndpkt().n() && rcvpkt.is_not_SYN() =>
            {
                ctx.udt_send(self.state().sndpkt())?;
                ctx.restart_connection_timer()?;
                Ok(self.wrap())
            }

            // edge 10: rcvpkt (data) with correct n
            RcvEvent::RecvPck(Some(rcvpkt), _)
                if rcvpkt.notcorrupt() && rcvpkt.n() != self.state().sndpkt().n() && rcvpkt.is_Data() =>
            {
                let data = ctx.extract_data(&rcvpkt);
                ctx.append(data)?;
                ctx.increase_data_counter(data.len());
                let sndpkt = ctx.make_pkt(rcvpkt.n(), Flag::ACK)?;
                ctx.udt_send(&sndpkt)?;
                ctx.restart_connection_timer()?;
                Ok(self.to_wait_for_pkt(sndpkt).wrap())
            }

            // edge 11: connection timeout
            RcvEvent::ConnectionTimeout => {
                println!("Connection Timeout after {} Bytes", ctx.get_data_counter());
                ctx.close_file()?;
                Ok(self.to_wait_for_connection().wrap())
            }

            // edge 12: fin rcvpkt with correct n
            RcvEvent::RecvPck(Some(rcvpkt), _)
                if rcvpkt.notcorrupt() && rcvpkt.n() != self.state().sndpkt().n() && rcvpkt.is_FIN() =>
            {
                println!("Connection Closed after {} Bytes", ctx.get_data_counter());
                let sndpkt = ctx.make_pkt(rcvpkt.n(), Flag::FINACK)?;
                ctx.udt_send(&sndpkt)?;
                ctx.stop_connection_timer()?;
                ctx.close_file()?;
                Ok(self.to_wait_for_connection().wrap())
            }

            // ..undefined
            _ => unreachable!("undefined transisions"),
        }
    }
}
