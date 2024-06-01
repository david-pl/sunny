use bitcode::{Decode, Encode};
use sunny_db::timeseries_db;

#[derive(Copy, Clone, Encode, Decode, PartialEq, Debug)]
struct PowerValues {
    power_pv: f64,
    power_to_grid: f64,
    power_from_grid: f64,
    power_used: f64,
}

#[test]
fn read_in_range_test() {
    let test_db_path = "./tests/db-test";

    let tiny_db = timeseries_db::SunnyDB::<PowerValues>::new(200, &test_db_path, 2, 20);

    let start_time = 1717113600000;
    let end_time = 1718113600000;

    let read_values = tiny_db.get_values_in_range(start_time, end_time);

    let start_time_series = read_values.unwrap().get_start_time().unwrap();

    assert!(start_time_series >= start_time);
}
