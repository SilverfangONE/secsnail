use std::io;

use super::fsm::{FsmStateWrapper, FsmWrap, SndEvent, SndFsm, SndStateStart, StateRouter};

use super::super::pck::Flag;
use super::*;

impl StateRouter for SndFsm<SndStateStart> {
    fn goto(
        self,
        e: SndEvent,
        ctx: &mut dyn fsm::ProtocolIoContext,
    ) -> io::Result<FsmStateWrapper> {
        // mhieron: Das klingt nach einem debug_assert!()
        // Wobei ich persönlich fan davon bin einfach ein assert! zu verwenden. Bug ist bug, und lieber explodiert es unter Production
        // und man weiß zu mindest ab wo das Program falsch abgebogen ist, anstatt falsch weiterzumachen.
        #[cfg(debug_assertions)]
        {
            if self.state().n() != 0 {
                unreachable!("StateStart n should never anything else than 0");
            }
        }

        let n = self.state().n();
        match e {
            // edge 1: start
            SndEvent::InitSYN => {
                let sndpck = ctx.make_pkt(n, Flag::SYN)?;
                ctx.udt_send(&sndpck)?;
                ctx.start_timer()?;
                Ok(self.to_wait(n, sndpck).wrap())
            }

            // ..undefined
            _ => unreachable!("undefined transision"),
        }
    }
}
