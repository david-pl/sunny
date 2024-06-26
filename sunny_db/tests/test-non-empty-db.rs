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


    // case 1: start time in series, end time large than max time
    let start_time = 1717113600000;
    let end_time = 1718113600000;

    let read_values = tiny_db.get_values_in_range(start_time, end_time).unwrap();

    let start_time_series = read_values.get_start_time().unwrap();
    assert!(start_time_series >= start_time);

    let end_time_series = read_values.get_end_time().unwrap();
    assert!(end_time_series <= end_time);


    // case 2: start time lower than min in series, end time in series
    let start_time = 1617113600000;
    let end_time = 1717113600000;

    let read_values = tiny_db.get_values_in_range(start_time, end_time).unwrap();

    let start_time_series = read_values.get_start_time().unwrap();
    assert!(start_time_series >= start_time);

    let end_time_series = read_values.get_end_time().unwrap();
    assert!(end_time_series <= end_time);

    // case 3: start time in series, end time in series
    let start_time = 1717113600000;
    let end_time = 1717154113550 - 20;

    let read_values = tiny_db.get_values_in_range(start_time, end_time).unwrap();

    let start_time_series = read_values.get_start_time().unwrap();
    assert!(start_time_series >= start_time);

    let end_time_series = read_values.get_end_time().unwrap();

    assert!(end_time_series <= end_time);

    // case 4: start time & end time lower than series
    let start_time = 1617113600000;
    let end_time = 1617154113550;

    let read_values = tiny_db.get_values_in_range(start_time, end_time).unwrap();
    assert!(read_values.is_empty());

    // case 5: start time & end time above series
    let start_time = 1817113600000;
    let end_time = 1817154113550;

    let read_values = tiny_db.get_values_in_range(start_time, end_time).unwrap();
    assert!(read_values.is_empty());

    // case 6: start time below & end time above series
    let start_time = 1617113600000;
    let end_time = 1817154113550;

    let read_values = tiny_db.get_values_in_range(start_time, end_time).unwrap();
    
    assert!(!read_values.is_empty());

    let start_time_series = read_values.get_start_time().unwrap();
    assert!(start_time_series >= start_time);

    let end_time_series = read_values.get_end_time().unwrap();
    assert!(end_time_series <= end_time);
}
