use super::*;
use crate::error::Result;
use rtcp::packet::Packet;
use util::Marshal;

fn add_run(r: &mut Recorder, sequence_numbers: &[u16], arrival_times: &[i64]) {
    assert_eq!(sequence_numbers.len(), arrival_times.len());

    for i in 0..sequence_numbers.len() {
        r.record(5000, sequence_numbers[i], arrival_times[i]);
    }
}

const TYPE_TCC_DELTA_SCALE_FACTOR: i64 = 250;
const SCALE_FACTOR_REFERENCE_TIME: i64 = 64000;

fn increase_time(arrival_time: &mut i64, increase_amount: i64) -> i64 {
    *arrival_time += increase_amount;
    *arrival_time
}

fn marshal_all(pkts: &[Box<dyn rtcp::packet::Packet + Send + Sync>]) -> Result<()> {
    for pkt in pkts {
        let _ = pkt.marshal()?;
    }
    Ok(())
}
