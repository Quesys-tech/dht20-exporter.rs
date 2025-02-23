use metrics::{describe_gauge, gauge};
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_util::MetricKindMask;
use rppal::i2c::I2c;
use rppal::system::DeviceInfo;
use std::{thread::sleep, time::Duration};

const fn pow2(n: u32) -> u32 {
    assert!(n < 32, "n must be less than 32");
    1 << n
}

#[derive(Debug)]
enum DHT20Error {
    BusError,
    SendError,
    ReceiveError,
    DataCorruption,
    MeasurementIncomplete,
}

#[derive(Debug)]
struct DHT20Data {
    temperature: f32,
    humidity: f32,
}

fn parse_dht20_data(data: &[u8]) -> Result<DHT20Data, DHT20Error> {
    if data.len() != 7 {
        return Err(DHT20Error::DataCorruption);
    }
    if (data[0] & 0x01) != 0 {
        return Err(DHT20Error::MeasurementIncomplete);
    }
    let s_rh =
        (data[1] as u32) * pow2(8 + 4) + (data[2] as u32) * pow2(4) + ((data[3] >> 4) as u32);
    let s_t =
        ((data[3] & 0x0f) as u32) * pow2(8 + 8) + (data[4] as u32) * pow2(8) + (data[5] as u32);

    Ok(DHT20Data {
        temperature: (s_t as f32) / (pow2(20) as f32) * 200.0 - 50.0,
        humidity: (s_rh as f32) / (pow2(20) as f32) * 100.0,
    })
}

fn get_dht20_data(i2c: &mut I2c) -> Result<DHT20Data, DHT20Error> {
    match i2c.set_slave_address(0x38) {
        Ok(_) => {}
        Err(_) => {
            return Err(DHT20Error::BusError);
        }
    };
    const TRIGGER: [u8; 3] = [0xAC, 0x33, 0x00];
    match i2c.write(&TRIGGER) {
        Ok(_) => {}
        Err(_) => {
            return Err(DHT20Error::SendError);
        }
    };
    sleep(Duration::from_millis(80));

    let mut data: [u8; 7] = [0xff; 7];
    let len_received = match i2c.read(&mut data) {
        Ok(ret) => ret,
        Err(_) => {
            return Err(DHT20Error::ReceiveError);
        }
    };
    return parse_dht20_data(&data[0..len_received]);
}

fn main() {
    println!("Running on a {}.", DeviceInfo::new().unwrap().model());

    let builder = PrometheusBuilder::new().with_http_listener(([0, 0, 0, 0], 9000));
    builder
        .idle_timeout(MetricKindMask::COUNTER, Some(Duration::from_secs(30)))
        .install()
        .expect("failed to install Prometheus recorder");

    describe_gauge!("room_temperature", "Room temperature");
    describe_gauge!("room_relative_humidity", "Room relative humidity");

    let mut i2c = I2c::with_bus(1).expect("Unable to construct I2c");
    loop {
        match get_dht20_data(&mut i2c) {
            Ok(data) => {
                println!(
                    "Temperature: {:.1}°C, Humidity: {:.1}%",
                    data.temperature, data.humidity
                );
                gauge!("room_temperature").set(data.temperature);
                gauge!("room_relative_humidity").set(data.humidity);
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
        sleep(Duration::from_millis(900));
    }
}
