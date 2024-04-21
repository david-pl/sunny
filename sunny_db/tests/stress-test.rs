use serde::{Serialize, Deserialize};
use sunny_db::timeseries_db;
use rand::{thread_rng, Rng};
use std::time::{Duration, Instant, UNIX_EPOCH};

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
struct PowerValues {
    power_pv: f64,
    power_grid: f64,
    power_used: f64,
}

#[test]
fn stress_test() {
    // generate a bunch of data
    let segment_size = 500;
    let segment_number = 1001;
    let test_db_path = "./tests/stress-test-data";

    let mut tiny_db = timeseries_db::SunnyDB::<PowerValues>::new(segment_size, &test_db_path, 2);
    let mut rng = thread_rng();

    let now = Instant::now();
    for _i in 0..segment_number {
        for _j in 0..segment_size {
            let power_vals = PowerValues {
                power_pv: rng.gen_range(-1e3..1e3),
                power_grid: rng.gen_range(-1e3..1e3),
                power_used: rng.gen_range(-1e3..1e3)
            };
            tiny_db.insert_value_at_current_time(power_vals);
            // sleep here is required otherwise data loss occurs because we're writing too fast
            std::thread::sleep(Duration::from_nanos(1));
        }
    }

    // last segment should still be in memory
    assert_eq!(tiny_db.time_series.get_current_values().len(), segment_size);

    println!("Elapsed time for writing {} segments of size {}: {} ms", segment_number, segment_size, now.elapsed().as_millis());

    let now = Instant::now();
    let read_data = tiny_db.get_all_values();
    println!("Elapsed time for reading {} segments of size {}: {} ms", segment_number, segment_size, now.elapsed().as_millis());

    assert!(read_data.is_some(), "Found no data in time series DB!");

    assert_eq!(read_data.as_ref().unwrap().get_current_values().len(), segment_number * segment_size, "Fewer data points read than written!");

    let time_series_start = read_data.unwrap().get_unix_start_timestamp_as_millis().unwrap();
    let start_time = UNIX_EPOCH + Duration::from_millis(time_series_start.try_into().unwrap()) + Duration::from_millis(80);
    let end_time = start_time + Duration::from_millis(50);

    let now = Instant::now();
    let few_values = tiny_db.get_values_in_range(start_time, end_time);
    let read_few_elapsed = now.elapsed().as_millis();

    assert!(few_values.is_some());
    assert!(few_values.as_ref().unwrap().len() >= 1);

    println!("Elapsed time for reading {} values out of {}: {} ms",few_values.as_ref().unwrap().len(), segment_number * segment_size, read_few_elapsed);
    println!("{:?}", few_values.unwrap().len());


    // clean up
    std::fs::remove_dir_all(&test_db_path).ok();
}