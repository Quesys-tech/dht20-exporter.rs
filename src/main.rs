use rppal::i2c::I2c;
use rppal::system::DeviceInfo;
use std::{thread::sleep, time::Duration};

const fn pow2(n: u32) -> u32 {
    assert!(n < 32, "n must be less than 32");
    1 << n
}

fn main() {
    println!("Running on a {}.", DeviceInfo::new().unwrap().model());

    let mut i2c = I2c::with_bus(1).expect("Unable to construct I2c");
    i2c.set_slave_address(0x38)
        .expect("Unable to set slave address");

    const TRIGGER: [u8; 3] = [0xAC, 0x33, 0x00];
    i2c.write(&TRIGGER).expect("Unable to send trigger");

    sleep(Duration::from_millis(80));

    let mut received: [u8; 7] = [0xff; 7];
    let len_received = i2c.read(&mut received).expect("Unable to receive data");
    if len_received == 7 && received[0] % 2 == 0 {
        println!("Measurement finished!");

        let s_rh = (received[1] as u32) * pow2(8 + 4)
            + (received[2] as u32) * pow2(4)
            + ((received[3] >> 4) as u32);
        let rh = (s_rh as f32) / (pow2(20) as f32) * 100.0;
        println!("RH (%): {}", rh);

        let s_t = ((received[3] & 0x0f) as u32) * pow2(8 + 8)
            + (received[4] as u32) * pow2(8)
            + (received[5] as u32);
        let t = (s_t as f32) / (pow2(20) as f32) * 200.0 - 50.0;
        println!("T (℃): {}", t);
    } else {
        println!("Retry needed.");
    }
}
