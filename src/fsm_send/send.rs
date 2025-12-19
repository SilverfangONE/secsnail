use std::io;

use crate::{
    fsm_send::fsm::{FsmWrap, SndEvent, SndFsm, SndStateSend},
    pck::Flag,
};

use super::{
    fsm::{FsmStateWrapper, StateRouter},
    *,
};

impl StateRouter for SndFsm<SndStateSend> {
    fn goto(
        self,
        e: SndEvent,
        ctx: &mut dyn fsm::ProtocolIoContext,
    ) -> io::Result<FsmStateWrapper> {
        let n = self.state().n();
        match e {
            // edge 4: data available
            SndEvent::DataAvailable(true) => {
                let sndpck = ctx.make_pkt(n, Flag::Data)?;
                ctx.increase_data_counter(sndpck.payload().len());
                ctx.udt_send(&sndpck)?;
                ctx.start_timer()?;
                Ok(self.to_wait(n, sndpck).wrap())
            }

            // edge 5: file end / no data available
            SndEvent::DataAvailable(false) => {
                let sndpck = ctx.make_pkt(n, Flag::FIN)?;
                ctx.udt_send(&sndpck)?;
                ctx.start_timer()?;
                Ok(self.to_wait(n, sndpck).wrap())
            }

            // edge 6: rcv pck
            SndEvent::RecvPck(_) => Ok(self.wrap()),

            // ..undefined
            _ => unreachable!("undefined transisions"),
        }
    }
}
