use std::io;

use crate::{
    fsm_recv::fsm::{
        FsmStateWrapper, FsmWrap, RcvEvent, RcvFsm, RcvStateWaitForConnection, StateRouter,
    },
    pck::Flag,
};

use super::*;

impl StateRouter for RcvFsm<RcvStateWaitForConnection> {
    fn goto(
        self,
        e: RcvEvent,
        ctx: &mut dyn fsm::ProtocolIoContext,
    ) -> io::Result<FsmStateWrapper> {
        match e {
            // corrupt packet (could not be parsed)
            RcvEvent::RecvPck(None, _) =>
            {
                Ok(self.wrap())
            }

            // edge 1a,b,c: not syn pkt, wrong seq n, corrupt pkt (checksum)
            RcvEvent::RecvPck(Some(rcvpkt), _)
                if rcvpkt.corrupt() || 0 != rcvpkt.n() || rcvpkt.is_not_SYN() =>
            {
                Ok(self.wrap())
            }

            // edge 2: recv syn pkt
            //
            // set snd_addr for this file transimsion session
            RcvEvent::RecvPck(Some(rcvpkt), snd_addr)
                if rcvpkt.notcorrupt() && rcvpkt.is_SYN() && 0 == rcvpkt.n() =>
            {
                // set snd_addr for starting session
                ctx.set_snd_addr(snd_addr);
                ctx.reset_data_counter();

                let file_name = ctx.extract_file_name(&rcvpkt)?;
                ctx.open_file(&file_name)?;
                let sndpkt = ctx.make_pkt(rcvpkt.n(), Flag::ACK)?;
                ctx.udt_send(&sndpkt)?;
                ctx.start_connection_timer()?;
                Ok(self.to_wait_for_pkt(sndpkt).wrap())
            }

            // edge 13: recv fin => ack fin
            //
            // n is irrelevant, use n from ack rcvpkt
            // the snd_addr is also irrelevant, every fin will be finack(d)
            RcvEvent::RecvPck(Some(rcvpkt), _) if rcvpkt.notcorrupt() && rcvpkt.is_FIN() => {
                let data = ctx.extract_data(&rcvpkt);
                ctx.append(data)?;
                let sndpkt = ctx.make_pkt(rcvpkt.n(), Flag::FINACK)?;
                ctx.udt_send(&sndpkt)?;
                Ok(self.wrap())
            }

            // ..undefined
            _ => {
                unreachable!("undefined transisions")
            }
        }
    }
}
